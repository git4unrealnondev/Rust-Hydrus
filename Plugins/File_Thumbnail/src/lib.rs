static PLUGIN_NAME: &str = "FileThumbnailer";
static PLUGIN_DESCRIPTION: &str = "Generates thumbnails for image files";

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;
use std::collections::HashMap;
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
            func: format!("{}GenerateThumbnail", PLUGIN_NAME),
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
pub fn FileThumbnailerGenerateThumbnail(
    callback: &sharedtypes::CallbackInfoInput,
) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
    dbg!(&callback);
    None
}

///
/// Just wrapping this incase i mess something up later...
///
fn load_image(byte_c: &[u8]) -> Result<Vec<Thumbnail>, ThumbError> {
    create_thumbnails_unknown_type(
        std::io::BufReader::new(std::io::Cursor::new(byte_c)),
        [ThumbnailSize::Icon],
    )
}

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
