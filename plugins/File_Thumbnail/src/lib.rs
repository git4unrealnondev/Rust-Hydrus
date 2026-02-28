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

#[path = "../../../src/client.rs"]
mod client;
#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

#[path = "../../../generated/client_api.rs"]
mod client_api;

use std::sync::atomic::AtomicUsize;
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
) -> HashMap<String, sharedtypes::CallbackCustomDataReturning> {
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
                if let sharedtypes::CallbackCustomDataReturning::U8(imgdata) = cdreturning { if let sharedtypes::CallbackCustomDataReturning::String(dbloc) = dbloc {
                    let (finpath, outhash) =
                        make_thumbnail_path(&Path::new(dbloc).to_path_buf(), imgdata);
                    let mut out = HashMap::new();
                    out.insert(
                        "path".to_string(),
                        sharedtypes::CallbackCustomDataReturning::String(
                            finpath.join(outhash).to_string_lossy().to_string(),
                        ),
                    );
                    return out;
                } }
            }
        }
    }
    HashMap::new()
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

    if canonicalize(dbloc).is_err() {
        create_dir_all(dbloc);
    }
    // Final folder location path of db
    let folderpath = canonicalize(dbloc)
        .unwrap()
        .join(&hash[0..2])
        .join(&hash[2..4])
        .join(&hash[4..6]);
    if let Ok(path) = std::fs::exists(folderpath.clone()) {
        if path {
            return (folderpath, hash);
        }
    }

    if let Err(err) = create_dir_all(folderpath.clone()) {
        panic!("Faled to make path at: {} {}", dbloc.to_string_lossy(), err);
    }

    (folderpath, hash)
}

///
/// Callback call to generate the thumbnail
///
#[no_mangle]
pub fn file_thumbnailer_generate_thumbnail_u8(
    callback: &sharedtypes::CallbackInfoInput,
) -> HashMap<String, sharedtypes::CallbackCustomDataReturning> {
    if callback.data_name.contains(&"image".to_string()) {
        let i = callback
            .data_name
            .iter()
            .position(|r| *r == "image".to_string())
            .unwrap();
        if let Some(cdreturning) = callback.data.get(i) {
            if let sharedtypes::CallbackCustomDataReturning::U8(imgdata) = cdreturning {
                if let Ok(thumbnail) = generate_thumbnail_u8(imgdata.to_vec()) {
                    let mut out = HashMap::new();
                    out.insert(
                        "image".to_string(),
                        sharedtypes::CallbackCustomDataReturning::U8(thumbnail),
                    );
                    return out;
                }
            }
        }
    }
    HashMap::new()
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
    use rayon::ThreadPoolBuilder;
    use rayon::iter::IntoParallelRefIterator;
    use rayon::iter::ParallelIterator;

    let api = client_api::RustHydrusApiClient::new("127.0.0.1:3030");

    let should_run = match client::settings_get_name(format!("{}-shouldrun", PLUGIN_NAME)) {
        None => {
            client::setting_add(
                format!("{}-shouldrun", PLUGIN_NAME),
                format!(
                    "From plugin {} {} Determines if we should run",
                    PLUGIN_NAME, PLUGIN_DESCRIPTION
                )
                .into(),
                None,
                Some("True".to_string()),
            );
            "True".to_string()
        }
        Some(loc) => match loc.param {
            None => {
                client::setting_add(
                    format!("{}-shouldrun", PLUGIN_NAME),
                    format!(
                        "From plugin {} {} Determines if we should run",
                        PLUGIN_NAME, PLUGIN_DESCRIPTION
                    )
                    .into(),
                    None,
                    Some("True".to_string()),
                );
                "True".to_string()
            }
            Some(out) => out,
        },
    };

    if should_run == "False" {
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
        utable = match api.namespace_get(&PLUGIN_NAME.to_string()).unwrap() {
            None => api
                .namespace_add(
                    &PLUGIN_NAME.to_string(),
                    &Some(PLUGIN_DESCRIPTION.to_string()),
                )
                .unwrap(),
            Some(id) => id,
        }
    }

    // Gets the tags inside a namespace
    let nids = api.namespace_get_tagids(&utable).unwrap();

    // Removes the fileids that already have thumbnails
    for each in nids.iter() {
        if should_run == "Clear" {
            api.tag_remove(each).unwrap();
        } else {
            for file_id in api.relationship_get_fileid(each).unwrap().iter() {
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
    let pool = ThreadPoolBuilder::new().build().unwrap();

    if let Some(location) = setup_thumbnail_location() {
        pool.install(|| {
            file_ids.par_iter().for_each(|fid| {
                let _ = std::panic::catch_unwind(|| {
                    if let Some(thumb_hash) = process_fid(fid, &location, &utable) {
                        api.add_tags_to_fileid(
                            Some(*fid),
                            &vec![sharedtypes::FileTagAction {
                                operation: sharedtypes::TagOperation::Set,
                                tags: vec![sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: PLUGIN_NAME.to_string(),
                                        description: Some(PLUGIN_DESCRIPTION.to_string()),
                                    },
                                    tag: thumb_hash,
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                }],
                            }],
                        );
                        //let _ = api.relationship_file_tag_add(*fid, thumb_hash, utable, None);
                    }
                });
            });
        });
    }
    client::log(format!("{} - generation done", PLUGIN_NAME));
    api.setting_add(
        format!("{}-shouldrun", PLUGIN_NAME),
        format!(
            "From plugin {} - {} Determines if we should run",
            PLUGIN_NAME, PLUGIN_DESCRIPTION
        )
        .into(),
        None,
        Some("False".to_string()),
    );
    api.transaction_flush();
}

fn process_fid(fid: &usize, location: &PathBuf, _utable: &usize) -> Option<String> {
    match generate_thumbnail(*fid) {
        Ok(thumb_file) => {
            let (thumb_path, thumb_hash) = make_thumbnail_path(location, &thumb_file);
            let thpath = thumb_path.join(thumb_hash.clone());
            let pa = thpath.to_string_lossy().to_string();
            /*client::log(format!(
                "{}: Writing fileid: {} thumbnail to {}",
                PLUGIN_NAME, fid, &pa
            ));*/
            if std::fs::write(pa, thumb_file).is_ok() {
                client::log_no_print(format!(
                    "Plugin: {} -- fid {fid} Wrote: {} to {:?}",
                    PLUGIN_NAME, &thumb_hash, &thumb_path,
                ));
                return Some(thumb_hash);
            }
        }
        Err(st) => {
            client::log(format!("{} Fid: {} ERR- {}", PLUGIN_NAME, &fid, st));
        }
    }
    None
}

///
/// Hashes a file based on a fid
///
#[no_mangle]
pub fn file_thumbnailer_generate_thumbnail_fid(
    callback: &sharedtypes::CallbackInfoInput,
) -> HashMap<String, sharedtypes::CallbackCustomDataReturning> {
    let index = callback.data_name.iter().position(|x| x == "file_id");
    if let Some(index) = index {
        if callback.data.len() >= index {
            if let Some(custom_data) = callback.data.get(index) {
                if let sharedtypes::CallbackCustomDataReturning::Usize(inp) = custom_data {
                    let _counter = &AtomicUsize::new(0);
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

                        if let Some(file_thumb_hash) = process_fid(inp, location, &utable) {
                            client::relationship_file_tag_add(
                                *inp,
                                file_thumb_hash,
                                utable,
                                None,
                            );
                        }
                    }
                }
            }
        }
    }
    HashMap::new()
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
                return Err(Error::other(
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
    use std::io::Error;
    let max_cnt = 5;
    if let Some(fbyte) = client::get_file(fid) {
        for _ in 0..5 {
            if let Ok(byte) = std::fs::read(&fbyte) {
                return generate_thumbnail_u8(byte);
            }
        }
    } else {
        return Err(Error::other("Err  get_file is none"));
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
use image::{AnimationDecoder, DynamicImage, ImageResult, codecs::gif::GifDecoder};
pub fn extract_gif_frames(cursor: std::io::Cursor<Vec<u8>>) -> ImageResult<Vec<DynamicImage>> {
    // Wrap the data in a Cursor so the decoder can read it like a file

    // Create the decoder
    let decoder = GifDecoder::new(cursor)?;

    // Decode the animation into frames
    // into_frames() returns an iterator of Frame objects
    let frames = decoder.into_frames();

    // Convert each frame into a DynamicImage and collect into a vector
    frames
        .map(|f| {
            // f? handles any decoding errors per frame
            let frame = f?;
            Ok(DynamicImage::ImageRgba8(frame.into_buffer()).resize_exact(
                SIZE_THUMBNAIL_X,
                SIZE_THUMBNAIL_Y,
                image::imageops::FilterType::Lanczos3,
            ))
        })
        .collect()
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

    let cursor = Cursor::new(filebytes);

    let res = thumbnailer::get_video_frame_multiple(
        cursor.clone(),
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
        Ok(mut ve) => {
            if ve.is_empty()
                && fileformat.extension() == "gif" {
                    if let Ok(frames) = extract_gif_frames(cursor) {
                        for frame in frames {
                            ve.push(frame);
                        }
                    }
                }

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

                encoder
                    .add_frame(&pixelbuf, (cnt * frate).try_into().unwrap())
                    .unwrap();
                cnt += 1;
            }
            let out = match encoder.finalize(((cnt + 1) * frate).try_into().unwrap()) {
                Ok(out) => out,
                Err(_err) => {
                    return None;
                }
            };
            Some(out.to_vec())
        }
        Err(_err) => None,
    }
}

fn setup_thumbnail_default(api: &client_api::RustHydrusApiClient) -> PathBuf {
    let storage = api.location_get().unwrap();
    let path = Path::new(&storage);
    let finpath = std::fs::canonicalize(path.join(LOCATION_THUMBNAILS))
        .unwrap()
        .to_string_lossy()
        .to_string();
    api.setting_add(
        format!("{}-location", PLUGIN_NAME),
        format!("From plugin {} {}", PLUGIN_NAME, PLUGIN_DESCRIPTION).into(),
        None,
        Some(finpath.clone()),
    );
    let final_location = Path::new(&finpath).to_path_buf();
    final_location
}

///
/// Gets the location to put thumbnails in
///
fn thumbnail_location_get(api: &client_api::RustHydrusApiClient) -> PathBuf {
    match api.settings_get_name(&format!("{}-location", PLUGIN_NAME)) {
        Ok(Some(setting)) => {
            let locpath = match setting.param {
                Some(loc) => Path::new(&loc).to_path_buf(),
                None => setup_thumbnail_default(api),
            };
            locpath
        }
        _ => setup_thumbnail_default(api),
    }
}

fn setup_default_path(path: &Path) -> String {
    let fpath = std::fs::canonicalize(path.join(LOCATION_THUMBNAILS));
    match fpath {
        Ok(_) => {}
        Err(_) => {
            if let Err(err) = std::fs::create_dir_all(path.join(LOCATION_THUMBNAILS)) {
                panic!(
                    "File thumbnailer failed to make thumbnail - {} - {}",
                    err,
                    path.to_string_lossy()
                );
            }
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
            );
            finpath
        }
        Some(loc) => match loc.param {
            None => {
                client::setting_add(
                    format!("{}-location", PLUGIN_NAME),
                    format!("From plugin {} {}", PLUGIN_NAME, PLUGIN_DESCRIPTION).into(),
                    None,
                    Some(finpath.clone()),
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
    api_info: &sharedtypes::ClientAPIInfo,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut output = Vec::new();
    match generate_thumbnail_u8(byte_c.to_vec()) {
        Ok(thumb) => {
    let api = client_api::RustHydrusApiClient::new(api_info.url.to_string());
            let thumbpath = thumbnail_location_get(&api);
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
