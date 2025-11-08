static PLUGIN_NAME: &str = "file_thumbnailer";
static PLUGIN_DESCRIPTION: &str = "Generates thumbnails for image files";
static LOCATION_THUMBNAILS: &str = "thumbnails";
static SIZE_THUMBNAIL_X: u32 = 250;
static SIZE_THUMBNAIL_Y: u32 = 250;
static DEFAULT_VIDEO_SETTINGS: VideoDefaults = VideoDefaults {
    frames: 50,
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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use strum_macros::EnumIter;
use thumbnailer::{Thumbnail, ThumbnailSize, create_thumbnails_unknown_type, error::ThumbError};
use webp_animation::EncodingConfig;
use webp_animation::EncodingType;

#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut main = sharedtypes::return_default_globalpluginparser();
    main.name = PLUGIN_NAME.to_string();
    main.version = 0;
    main.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_channel: true,
            redirect: None,
        },
    ));
    main.callbacks = vec![
        sharedtypes::GlobalCallbacks::Start(sharedtypes::StartupThreadType::Spawn),
        sharedtypes::GlobalCallbacks::Download,
        sharedtypes::GlobalCallbacks::Import,
        sharedtypes::GlobalCallbacks::Callback(sharedtypes::CallbackInfo {
            func: format!("{}_generate_thumbnail_fid", PLUGIN_NAME),
            vers: 0,
            data_name: vec!["file_id".into()],
            data: vec![sharedtypes::CallbackCustomData::Usize],
        }),
    ];
    let out = vec![main];

    out
}

///
/// Callback call to generate the thumbnail
///
#[no_mangle]
pub fn file_thumbnailer_give_thumbnail_location(
    callback: &sharedtypes::CallbackInfoInput,
) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
    use std::path::Path;

    // If we have both image and thumbnail location
    if callback.data_name.contains(&"image".to_string())
        && callback
            .data_name
            .contains(&"thumbnail_location".to_string())
    {
        // Gets position of data inside of vecs
        let i = callback
            .data_name
            .iter()
            .position(|r| *r == "image".to_string())
            .unwrap();
        let j = callback
            .data_name
            .iter()
            .position(|r| *r == "thumbnail_location".to_string())
            .unwrap();

        if let Some(cdreturning) = callback.data.get(i) {
            if let Some(dbloc) = callback.data.get(j) {
                match cdreturning {
                    sharedtypes::CallbackCustomDataReturning::U8(imgdata) => match dbloc {
                        sharedtypes::CallbackCustomDataReturning::String(dbloc) => {
                            let (finpath, outhash) =
                                make_thumbnail_path(&Path::new(dbloc).to_path_buf(), imgdata);
                            let mut out = HashMap::new();
                            out.insert(
                                "path".to_string(),
                                sharedtypes::CallbackCustomDataReturning::String(
                                    finpath.join(outhash).to_string_lossy().to_string(),
                                ),
                            );
                            return Some(out);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
    None
}

///
/// Generates and tries to create a folder path for thumbnail storage
/// Returns the path of the thumbnail including the hash
///
fn make_thumbnail_path(dbloc: &PathBuf, imgdata: &Vec<u8>) -> (PathBuf, String) {
    use sha2::Digest;
    use sha2::Sha256;
    use std::fs::canonicalize;
    use std::fs::create_dir_all;
    let mut hasher = Sha256::new();
    hasher.update(imgdata);
    let hash = format!("{:X}", hasher.finalize());

    // Final folder location path of db
    let folderpath = canonicalize(dbloc)
        .unwrap()
        .join(hash[0..2].to_string())
        .join(hash[2..4].to_string())
        .join(hash[4..6].to_string());

    create_dir_all(folderpath.clone()).unwrap();

    (folderpath, hash)
}

///
/// Callback call to generate the thumbnail
///
#[no_mangle]
pub fn file_thumbnailer_generate_thumbnail_u8(
    callback: &sharedtypes::CallbackInfoInput,
) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
    if callback.data_name.contains(&"image".to_string()) {
        let i = callback
            .data_name
            .iter()
            .position(|r| *r == "image".to_string())
            .unwrap();
        if let Some(cdreturning) = callback.data.get(i) {
            match cdreturning {
                sharedtypes::CallbackCustomDataReturning::U8(imgdata) => {
                    if let Ok(thumbnail) = generate_thumbnail_u8(imgdata.to_vec()) {
                        let mut out = HashMap::new();
                        out.insert(
                            "image".to_string(),
                            sharedtypes::CallbackCustomDataReturning::U8(thumbnail),
                        );
                        return Some(out);
                    }
                }
                _ => {}
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

    let should_run = match client::settings_get_name(format!("{}-shouldrun", PLUGIN_NAME)) {
        None => {
            client::setting_add(
                format!("{}-shouldrun", PLUGIN_NAME).into(),
                format!(
                    "From plugin {} {} Determines if we should run",
                    PLUGIN_NAME, PLUGIN_DESCRIPTION
                )
                .into(),
                None,
                Some("True".to_string()),
                true,
            );
            client::transaction_flush();
            "True".to_string()
        }
        Some(loc) => match loc.param {
            None => {
                client::setting_add(
                    format!("{}-shouldrun", PLUGIN_NAME).into(),
                    format!(
                        "From plugin {} {} Determines if we should run",
                        PLUGIN_NAME, PLUGIN_DESCRIPTION
                    )
                    .into(),
                    None,
                    Some("True".to_string()),
                    true,
                );
                "True".to_string()
            }
            Some(out) => out,
        },
    };

    if should_run == "False".to_string() {
        client::log_no_print(format!(
            "{} - Returning due to should run is false.",
            PLUGIN_NAME
        ));
        return;
    } else {
        client::log(format!("{} - Running main processing job", PLUGIN_NAME));
    }

    if should_run == "Clear" {
        client::log(format!(
            "{} - Will delete previous file_thumbnails from the system",
            PLUGIN_NAME
        ));
    }

    let table_temp = sharedtypes::LoadDBTable::Files;
    client::load_table(table_temp);
    let mut file_ids = client::file_get_list_id();
    let table_temp = sharedtypes::LoadDBTable::All;
    client::load_table(table_temp);

    // Gets namespace id if it doesn't exist then recreate
    let utable;
    {
        utable = match client::namespace_get(PLUGIN_NAME.to_string()) {
            None => client::namespace_put(
                PLUGIN_NAME.to_string(),
                Some(PLUGIN_DESCRIPTION.to_string()),
            ),
            Some(id) => id,
        }
    }

    // Gets the tags inside a namespace
    let nids = client::namespace_get_tagids(utable);

    // Removes the fileids that already have thumbnails
    for each in nids {
        if should_run == "Clear" {
            client::tag_remove(each);
        } else {
            for file_id in client::relationship_get_fileid(each).iter() {
                file_ids.remove(file_id);
            }
        }
    }

    // Logs info to screen
    client::log(format!(
        "{} - We've got {} files to parse.",
        PLUGIN_NAME,
        file_ids.len()
    ));
    let counter = AtomicUsize::new(0);
    if let Some(location) = setup_thumbnail_location() {
        file_ids.par_iter().for_each(|fid| {
            let _ = std::panic::catch_unwind(|| {
                process_fid(fid, &location, &counter, &utable);
            });
        });
    }
    client::log(format!("{} - generation done", PLUGIN_NAME));
    client::setting_add(
        format!("{}-shouldrun", PLUGIN_NAME).into(),
        format!(
            "From plugin {} - {} Determines if we should run",
            PLUGIN_NAME, PLUGIN_DESCRIPTION
        )
        .into(),
        None,
        Some("False".to_string()),
        true,
    );
    client::transaction_flush();
}

fn process_fid(fid: &usize, location: &PathBuf, counter: &AtomicUsize, utable: &usize) {
    match generate_thumbnail(*fid) {
        Ok(thumb_file) => {
            let (thumb_path, thumb_hash) = make_thumbnail_path(&location, &thumb_file);
            let thpath = thumb_path.join(thumb_hash.clone());
            let pa = thpath.to_string_lossy().to_string();
            /*client::log(format!(
                "{}: Writing fileid: {} thumbnail to {}",
                PLUGIN_NAME, fid, &pa
            ));*/
            if let Ok(_) = std::fs::write(pa, thumb_file) {
                let cnt = counter.fetch_add(1, Ordering::SeqCst);
                client::log_no_print(format!(
                    "Plugin: {} -- fid {fid} Wrote: {} to {:?}",
                    PLUGIN_NAME, &thumb_hash, &thumb_path,
                ));

                let _ = client::relationship_file_tag_add(*fid, thumb_hash, *utable, true, None);
                if cnt == 1000 {
                    client::transaction_flush();
                    counter.store(0, Ordering::SeqCst);
                }
            }
        }
        Err(st) => {
            client::log(format!("{} Fid: {} ERR- {}", PLUGIN_NAME, &fid, st));
        }
    }
}

///
/// Hashes a file based on a fid
///
#[no_mangle]
pub fn file_thumbnailer_generate_thumbnail_fid(
    callback: &sharedtypes::CallbackInfoInput,
) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
    let index = callback.data_name.iter().position(|x| x == "file_id");
    if let Some(index) = index {
        if callback.data.len() >= index {
            if let Some(custom_data) = callback.data.get(index) {
                match custom_data {
                    sharedtypes::CallbackCustomDataReturning::Usize(inp) => {
                        let counter = &AtomicUsize::new(0);
                        if let Some(location) = &setup_thumbnail_location() {
                            // Gets namespace id if it doesn't exist then recreate
                            let utable;
                            {
                                utable = match client::namespace_get(PLUGIN_NAME.to_string()) {
                                    None => client::namespace_put(
                                        PLUGIN_NAME.to_string(),
                                        Some(PLUGIN_DESCRIPTION.to_string()),
                                    ),
                                    Some(id) => id,
                                }
                            }

                            process_fid(&inp, location, counter, &utable);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

///
/// Generate thumbnail function
///
fn generate_thumbnail_u8(inp: Vec<u8>) -> Result<Vec<u8>, std::io::Error> {
    use file_format::{FileFormat, Kind};
    use std::io::{Error, ErrorKind};
    let thumbvec = match load_image(&inp) {
        Ok(t) => t,
        Err(err) => match err {
            ThumbError::Unsupported(fformat) => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    format!(
                        "{PLUGIN_NAME} - Cannot Parse file with format: {:?}.",
                        fformat.kind()
                    ),
                ));
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("{PLUGIN_NAME} - Failed to match err - 190 {:?}", err),
                ));
            }
        },
    };
    let thumb = &thumbvec[0];
    match thumb.return_fileformat().kind() {
        Kind::Image => match thumb.return_fileformat() {
            FileFormat::GraphicsInterchangeFormat => {
                match make_animated_img(
                    inp,
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
                inp,
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
                    inp,
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
    }
}

///
/// Actually generates thumbnails
///
pub fn generate_thumbnail(fid: usize) -> Result<Vec<u8>, std::io::Error> {
    use std::io::{Error, ErrorKind};
    let max_cnt = 5;
    if let Some(fbyte) = client::get_file(fid) {
        for _ in 0..5 {
            if let Ok(byte) = std::fs::read(&fbyte) {
                return generate_thumbnail_u8(byte);
            }
        }
    } else {
        return Err(Error::new(ErrorKind::Other, "Err  get_file is none"));
    }
    Err(std::io::Error::other(format!(
        "Could not load the filter after {}",
        max_cnt
    )))
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
    let frate = 4;
    let res = thumbnailer::get_video_frame_multiple(
        Cursor::new(filebytes),
        fileformat,
        spl.frames as usize,
        frate,
        Some((SIZE_THUMBNAIL_X, SIZE_THUMBNAIL_Y)),
    );
    let webpconfig = EncodingConfig {
        encoding_type: EncodingType::Lossy(webp_animation::LossyEncodingConfig {
            alpha_quality: 50,
            alpha_filtering: 2,
            sns_strength: 70,
            filter_strength: 100,
            preprocessing: true,
            filter_type: 1,
            pass: 10,
            ..Default::default()
        }),
        quality: 50.0,
        method: 6,
    };
    match res {
        Ok(ve) => {
            use webp_animation::Encoder;
            use webp_animation::EncoderOptions;
            let mut encoder = Encoder::new_with_options(
                (SIZE_THUMBNAIL_X, SIZE_THUMBNAIL_Y),
                EncoderOptions {
                    kmin: 3,
                    kmax: 5,
                    encoding_config: Some(webpconfig),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut cnt = 0;
            for each in ve {
                let mut pixelbuf =
                    Vec::with_capacity((each.width() * each.height() * 4).try_into().unwrap());
                for each in each.into_rgba8().pixels() {
                    for test in each.channels() {
                        pixelbuf.push(*test);
                    }
                }

                let _ = encoder.add_frame(&pixelbuf, (cnt * frate).try_into().unwrap());
                cnt += 1;
            }
            let out = match encoder.finalize(((cnt + 1) * frate).try_into().unwrap()) {
                Ok(out) => out,
                Err(_) => return None,
            };
            Some(out.to_vec())
        }
        Err(err) => {
            dbg!("err", err);
            None
        }
    }
}

fn setup_thumbnail_default() -> PathBuf {
    let storage = client::location_get();
    let path = Path::new(&storage);
    let finpath = std::fs::canonicalize(path.join(LOCATION_THUMBNAILS))
        .unwrap()
        .to_string_lossy()
        .to_string();
    client::setting_add(
        format!("{}-location", PLUGIN_NAME),
        format!("From plugin {} {}", PLUGIN_NAME, PLUGIN_DESCRIPTION).into(),
        None,
        Some(finpath.clone()),
        true,
    );
    client::transaction_flush();
    let final_location = Path::new(&finpath).to_path_buf();
    final_location
}

///
/// Gets the location to put thumbnails in
///
fn thumbnail_location_get() -> PathBuf {
    match client::settings_get_name(format!("{}-location", PLUGIN_NAME)) {
        Some(setting) => {
            let locpath = match setting.param {
                Some(loc) => Path::new(&loc).to_path_buf(),
                None => setup_thumbnail_default(),
            };
            locpath
        }
        None => setup_thumbnail_default(),
    }
}

fn setup_default_path(path: &Path) -> String {
    let fpath = std::fs::canonicalize(path.join(LOCATION_THUMBNAILS));
    match fpath {
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::create_dir_all(path.join(LOCATION_THUMBNAILS));
        }
    }
    std::fs::canonicalize(path.join(LOCATION_THUMBNAILS))
        .unwrap()
        .to_string_lossy()
        .to_string()
}

///
/// Setting up the thumbnail folder
/// Uses no unwrap and should be cross compatible across OSes
///
fn setup_thumbnail_location() -> Option<PathBuf> {
    let storage = client::location_get();
    let path = Path::new(&storage);
    let finpath = setup_default_path(path);
    // If we don't have a setting setup for this then make one
    let location = match client::settings_get_name(format!("{}-location", PLUGIN_NAME)) {
        None => {
            client::setting_add(
                format!("{}-location", PLUGIN_NAME),
                format!("From plugin {} {}", PLUGIN_NAME, PLUGIN_DESCRIPTION).into(),
                None,
                Some(finpath.clone()),
                true,
            );
            client::transaction_flush();
            finpath
        }
        Some(loc) => match loc.param {
            None => {
                client::setting_add(
                    format!("{}-location", PLUGIN_NAME),
                    format!("From plugin {} {}", PLUGIN_NAME, PLUGIN_DESCRIPTION).into(),
                    None,
                    Some(finpath.clone()),
                    true,
                );
                finpath
            }
            Some(out) => out,
        },
    };
    let final_location = Path::new(&location).to_path_buf();

    match std::fs::metadata(&final_location) {
        Ok(metadata) => {
            // Checks if the folder exists if it doesn't exist then it tries to create it
            if metadata.is_dir() {
                Some(final_location)
            } else {
                // Tries to create a folder
                match std::fs::create_dir_all(&final_location) {
                    Ok(_) => Some(final_location),
                    Err(_) => None,
                }
            }
        }
        Err(_) => {
            // Tries to create a folder
            match std::fs::create_dir_all(&final_location) {
                Ok(_) => Some(final_location),
                Err(_) => None,
            }
        }
    }
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
    match generate_thumbnail_u8(byte_c.to_vec()) {
        Ok(thumb) => {
            let thumbpath = thumbnail_location_get();
            let (thumb_path, thumb_hash) = make_thumbnail_path(&thumbpath, &thumb);
            let thpath = thumb_path.join(thumb_hash.clone());
            let pa = thpath.to_string_lossy().to_string();
            match std::fs::write(pa, thumb) {
                Ok(_) => {
                    client::log_no_print(format!(
                        "Plugin: {} -- Hash {hash_in} Wrote: {} to {:?}",
                        PLUGIN_NAME, &thumb_hash, &thumb_path,
                    ));

                    let plugin_output = sharedtypes::DBPluginOutput {
                        file: vec![sharedtypes::PluginFileObj {
                            id: None,
                            hash: Some(hash_in.to_string()),
                            ext: Some(ext_in.to_string()),
                            location: None,
                        }],
                        jobs: vec![],
                        setting: vec![],
                        tag: vec![sharedtypes::TagObject {
                            tag: thumb_hash.to_string(),
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: PLUGIN_NAME.to_string(),
                                description: Some(
                                    "A hash of a picture or image for a thumbnail".into(),
                                ),
                            },
                            relates_to: None,
                            tag_type: sharedtypes::TagType::Normal,
                        }],
                        relationship: vec![sharedtypes::DbPluginRelationshipObj {
                            file_hash: hash_in.to_string(),
                            tag_name: thumb_hash,
                            tag_namespace: PLUGIN_NAME.to_string(),
                        }],
                    };

                    output.push(sharedtypes::DBPluginOutputEnum::Add(vec![plugin_output]));
                }
                Err(err) => {
                    client::log(format!(
                        "Plugin: {} -- {hash_in} Failed to write: {}, {:?}",
                        PLUGIN_NAME, hash_in, err,
                    ));
                }
            }
        }
        Err(err) => {
            client::log(format!(
                "Plugin: {} -- {hash_in} Failed to load: {}, {:?}",
                PLUGIN_NAME, hash_in, err,
            ));
        }
    }
    output
}
