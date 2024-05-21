static PLUGIN_NAME: &str = "file_thumbnailer";
static DEFAULT_FOLDER_NAME: &str = "thumbnails";
static PLUGIN_DESCRIPTION: &str = "Generates thumbnails for image files";
static LOCATION_THUMBNAILS: &str = "thumbnails";
static SIZE_THUMBNAIL_X: u32 = 100;
static SIZE_THUMBNAIL_Y: u32 = 100;
static DEFAULT_VIDEO_SETTINGS: VideoDefaults = VideoDefaults {
    frames: 25,
    duration: VideoSpacing::Duration(1000),
};

///
/// Default video settings
///
#[derive(Clone)]
pub struct VideoDefaults {
    frames: u32,            // how many frames should be in the animated webp
    duration: VideoSpacing, // how much duration should be before each frame get's captures
}

///
/// Will determine how long the video will be before attempting to take another frame.
///
#[derive(Clone)]
pub enum VideoSpacing {
    Frame(u32),      // X frames of a video before trying to take a frame
    Duration(usize), // Number of ms before attempting to take a frame
}

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use strum_macros::EnumIter;
use thumbnailer::{
    create_thumbnails, create_thumbnails_unknown_type, error::ThumbError, Thumbnail, ThumbnailSize,
};
#[no_mangle]
pub fn return_info() -> sharedtypes::PluginInfo {
    let callbackvec = vec![
        sharedtypes::PluginCallback::OnDownload,
        sharedtypes::PluginCallback::OnStart,
        sharedtypes::PluginCallback::OnCallback(sharedtypes::CallbackInfo {
            name: format!("{}", PLUGIN_NAME),
            func: format!("{}_generate_thumbnail", PLUGIN_NAME),
            vers: 0,
            data_name: [format!("image")].to_vec(),
            data: Some([sharedtypes::CallbackCustomData::U8].to_vec()),
        }),
    ];
    sharedtypes::PluginInfo {
        name: PLUGIN_NAME.to_string(),
        description: PLUGIN_DESCRIPTION.to_string(),
        version: 1.00,
        api_version: 1.00,
        callbacks: callbackvec,
        communication: Some(sharedtypes::PluginSharedData {
            thread: sharedtypes::PluginThreadType::Inline,
            com_channel: Some(sharedtypes::PluginCommunicationChannel::Pipe(
                "beans".to_string(),
            )),
        }),
    }
}

///
/// Callback call to generate the thumbnail
///
#[no_mangle]
pub fn file_thumbnailer_generate_thumbnail(
    callback: &sharedtypes::CallbackInfoInput,
) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
    dbg!(&callback);
    if callback.data_name.contains(&"image".to_string()) {
        let i = callback
            .data_name
            .iter()
            .position(|r| *r == "image".to_string())
            .unwrap();
        if let Some(data) = &callback.data {
            if let Some(cdreturning) = data.get(i) {
                match cdreturning {
                    sharedtypes::CallbackCustomDataReturning::U8(imgdata) => {
                        if let Ok(thumbnail) = load_image(&imgdata) {}
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

///
/// Just wrapping this incase i mess something up later...
///
fn load_image(byte_c: &[u8]) -> Result<Vec<Thumbnail>, ThumbError> {
    create_thumbnails_unknown_type(
        std::io::BufReader::new(std::io::Cursor::new(byte_c)),
        [ThumbnailSize::Custom((SIZE_THUMBNAIL_X, SIZE_THUMBNAIL_Y))],
    )
}

///
/// Interface for starting the database
///
#[no_mangle]
pub fn on_start() {
    use rayon::iter::IntoParallelRefIterator;
    use rayon::iter::ParallelIterator;
    let table_temp = sharedtypes::LoadDBTable::Files;
    client::load_table(table_temp);
    let mut file_ids = client::file_get_list_all();
    let table_temp = sharedtypes::LoadDBTable::All;
    client::load_table(table_temp);

    // Gets namespace id if it doesn't exist then recreate
    let utable;
    {
        utable = match client::namespace_get(PLUGIN_NAME.to_string()) {
            None => client::namespace_put(
                PLUGIN_NAME.to_string(),
                Some(PLUGIN_DESCRIPTION.to_string()),
                true,
            ),
            Some(id) => id,
        }
    }

    // Gets the tags inside a namespace
    let nids = match client::namespace_get_tagids(utable) {
        None => HashSet::new(),
        Some(set) => set,
    };

    // Removes fileid if it contains our tag if it has the namespace for it.
    for each in nids {
        if let Some(tag_id) = client::relationship_get_fileid(each) {
            for tag in tag_id {
                file_ids.remove(&tag);
            }
        }
    }

    // Logs info to screen
    client::log(format!(
        "FileThumbnailer - We've got {} files to parse.",
        file_ids.len()
    ));
    if let Some(location) = setup_thumbnail_location() {
        dbg!(&location);
        file_ids.par_iter().for_each(|fid| {
            if fid.1.ext == "gif".to_string() {
                match generate_thumbnail(*fid.0) {
                    Ok(thumb_file) => {
                        let _ = std::fs::write(format!("./test/out-{}.webp", fid.0), thumb_file);
                    }
                    Err(st) => {
                        client::log(format!("FileThumbnailer ERR- {}", st));
                    }
                }
            }
        });
    }
}

///
/// Actually generates thumbnails
///
pub fn generate_thumbnail(fid: usize) -> Result<Vec<u8>, std::io::Error> {
    use file_format::{FileFormat, Kind};
    use std::io::{Error, ErrorKind};
    if let Some(fbyte) = client::get_file(fid) {
        let byte = std::fs::read(fbyte)?;
        let thumbvec = match load_image(&byte) {
            Ok(t) => t,
            Err(err) => match err {
                ThumbError::Unsupported(fformat) => {
                    return Err(Error::new(
                        ErrorKind::Unsupported,
                        format!("Cannot Parse file with format: {:?}.", fformat.kind()),
                    ))
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Failed to match err - 190 {:?}", err),
                    ))
                }
            },
        };
        let thumb = &thumbvec[0];
        return match thumb.return_fileformat().kind() {
            Kind::Image => match thumb.return_fileformat() {
                FileFormat::GraphicsInterchangeFormat => {
                    match make_animated_img(
                        byte,
                        thumb.return_fileformat(),
                        DEFAULT_VIDEO_SETTINGS.clone(),
                    ) {
                        Some(vec) => Ok(vec),
                        None => Err(Error::new(ErrorKind::Unsupported, "GIF Defuzzing failed")),
                    }
                }
                _ => Ok(make_img(thumb.clone())),
            },
            Kind::Video => {
                match make_animated_img(
                    byte,
                    thumb.return_fileformat(),
                    DEFAULT_VIDEO_SETTINGS.clone(),
                ) {
                    Some(vec) => Ok(vec),
                    None => Err(Error::new(ErrorKind::Unsupported, "")),
                }
            }
            Kind::Other => match thumb.return_fileformat() {
                FileFormat::Mpeg4Part14 => {
                    match make_animated_img(
                        byte,
                        thumb.return_fileformat(),
                        DEFAULT_VIDEO_SETTINGS.clone(),
                    ) {
                        Some(vec) => Ok(vec),
                        None => Err(Error::new(ErrorKind::Unsupported, "gif is bad")),
                    }
                }
                _ => Err(Error::new(ErrorKind::Unsupported, "other bad")),
            },
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "Returning fileformat not valid",
            )),
        };
    } else {
        Err(Error::new(ErrorKind::Other, "Err  get_file is none"))
    }
}

///
/// Makes a image from a thumbnail
///
fn make_img(thumb: Thumbnail) -> Vec<u8> {
    use std::io::Cursor;
    let mut buf = Cursor::new(Vec::new());
    thumb.write_webp(&mut buf).unwrap();
    buf.into_inner()
}

///
/// Makes an animated image thumbnail.
///
fn make_animated_img(
    filebytes: Vec<u8>,
    fileformat: file_format::FileFormat,
    spl: VideoDefaults,
) -> Option<Vec<u8>> {
    use image::Pixel;
    use std::io::Cursor;
    let frate = 24;
    let res = thumbnailer::get_video_frame_multiple(
        Cursor::new(filebytes),
        fileformat,
        spl.frames as usize,
        frate,
        Some((SIZE_THUMBNAIL_X, SIZE_THUMBNAIL_Y)),
    );
    match res {
        Ok(ve) => {
            use webp_animation::Encoder;
            let mut encoder = Encoder::new((SIZE_THUMBNAIL_X, SIZE_THUMBNAIL_Y)).unwrap();
            let mut cnt = 0;
            for each in ve {
                let mut pixelbuf =
                    Vec::with_capacity((each.width() * each.height() * 4).try_into().unwrap());
                for each in each.into_rgba8().pixels() {
                    for test in each.channels() {
                        pixelbuf.push(test.clone());
                    }
                }

                let test = encoder.add_frame(&*pixelbuf, (cnt * frate).try_into().unwrap());
                cnt += 1;
            }
            let out = encoder
                .finalize(((cnt + 1) * frate).try_into().unwrap())
                .unwrap();
            Some(out.to_vec())
        }
        Err(_) => return None,
    }
}

///
/// Hash file using SHA512
///
fn hash_file(file: &[u8]) -> String {
    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::new();
    hasher.update(file);
    format!("{:X}", hasher.finalize())
}

///
/// Setting up the thumbnail folder
/// Uses no unwrap and should be cross compatible across OSes
///
fn setup_thumbnail_location() -> Option<PathBuf> {
    let storage = client::location_get();
    let path = Path::new(&storage);
    let final_location = append_dir(path, LOCATION_THUMBNAILS);
    dbg!(&path, &final_location);
    match std::fs::metadata(&final_location) {
        Ok(metadata) => {
            // Checks if the folder exists if it doesn't exist then it tries to create it
            if metadata.is_dir() {
                return Some(final_location);
            } else {
                // Tries to create a folder
                return match std::fs::create_dir_all(&final_location) {
                    Ok(_) => Some(final_location),
                    Err(_) => None,
                };
            }
        }
        Err(_) => {
            // Tries to create a folder
            return match std::fs::create_dir_all(&final_location) {
                Ok(_) => Some(final_location),
                Err(_) => None,
            };
        }
    }
}

///
/// Appends a folder to a dir
/// Yoinked from: https://stackoverflow.com/questions/66193779/how-to-add-a-folder-to-a-path-before-the-filename
/// Uses unwrap should only be an issue if someone is doing something stupid like
/// using their C directly as their location
///
fn append_dir(p: &Path, d: &str) -> PathBuf {
    p.join(d)
}

///
/// Might add support for moving images :D
///
#[derive(EnumIter, PartialEq, Clone, Copy, Debug, Eq, Hash)]
enum Supset {
    StaticImage,  // Used for thumbnails that don't move
    DynamicImage, // Used for thumbnails that do move
}

///
/// Runs when the main program runs the on_download call
///
#[no_mangle]
//pub fn OnDownload(byteCursor: Cursor<Bytes>, Hash: &String, Ext: &String, datab: Arc<Mutex<database::Main>>) {
pub fn on_download(
    byte_c: &[u8],
    hash_in: &String,
    ext_in: &String,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut output = Vec::new();

    let lmimg = load_image(byte_c);
    match lmimg {
        Ok(good_lmimg) => {
            /*let string_blurhash = downloadparse(good_lmimg);

            let plugin_output = sharedtypes::DBPluginOutput {
                file: Some(vec![sharedtypes::PluginFileObj {
                    id: None,
                    hash: Some(hash_in.to_string()),
                    ext: Some(ext_in.to_string()),
                    location: None,
                }]),
                jobs: None,
                namespace: Some(vec![sharedtypes::DbPluginNamespace {
                    name: PLUGIN_NAME.to_string(),
                    description: Some(PLUGIN_DESCRIPTION.to_string()),
                }]),
                parents: None,
                setting: None,
                tag: Some(vec![sharedtypes::DBPluginTagOut {
                    name: string_blurhash.to_string(),
                    namespace: PLUGIN_NAME.to_string(),
                    parents: None,
                }]),
                relationship: Some(vec![sharedtypes::DbPluginRelationshipObj {
                    file_hash: hash_in.to_string(),
                    tag_name: string_blurhash,
                    tag_namespace: PLUGIN_NAME.to_string(),
                }]),
            };

            output.push(sharedtypes::DBPluginOutputEnum::Add(vec![plugin_output]));*/
        }
        Err(err) => {
            client::log(format!(
                "Plugin: {} -- Failed to load: {}, {:?}",
                PLUGIN_NAME, hash_in, err,
            ));
        }
    }
    output
}
