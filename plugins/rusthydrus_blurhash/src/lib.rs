use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;

use blurhash::encode;
use image::imageops::resize;
use image::imageops::FilterType;
use image::{self, DynamicImage};

// Default image width to scale to before encoding to blurhash
static WIDTH_IMG: u32 = 500;
static HEIGHT_IMG: u32 = 500;

// "Level of detail" of blur
static ENCODE_IMG_X: u32 = 5;
static ENCODE_IMG_Y: u32 = 5;

static PLUGIN_NAME: &str = "blurhash";
static DB_NAME: &str = "BlurHash-blurhash";
static PLUGIN_DESCRIPTION: &str = "Introduces Blurhash imaging support.";

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut main = sharedtypes::return_default_globalpluginparser();
    main.name = PLUGIN_NAME.to_string();
    main.version = 0;
    main.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_type: sharedtypes::PluginThreadType::SpawnInline,
            com_channel: true,
        },
    ));
    main.callbacks = vec![
        sharedtypes::GlobalCallbacks::Start,
        sharedtypes::GlobalCallbacks::Download,
    ];
    let out = vec![main];

    out
}

///
/// resizes img and inserts into db
///
fn downloadparse(img: DynamicImage) -> String {
    let rescale_img = resize(&img, WIDTH_IMG, HEIGHT_IMG, FilterType::Triangle);
    encode(
        ENCODE_IMG_X,
        ENCODE_IMG_Y,
        WIDTH_IMG,
        HEIGHT_IMG,
        &rescale_img.into_raw(),
    )
    .unwrap()
}

#[no_mangle]
pub fn on_start() {
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
    }

    let table_temp = sharedtypes::LoadDBTable::Files;
    client::load_table(table_temp);

    let mut file_ids = client::file_get_list_all();
    let table_temp = sharedtypes::LoadDBTable::All;
    client::load_table(table_temp);

    // Gets namespace id if it doesn't exist then recreate
    let utable;
    {
        utable = match client::namespace_get(DB_NAME.to_string()) {
            None => client::namespace_put(
                DB_NAME.to_string(),
                Some(PLUGIN_DESCRIPTION.to_string()),
                true,
            ),
            Some(id) => id,
        }
    }

    // Gets the tags inside a namespace
    let nids = client::namespace_get_tagids(utable);
    // Removes fileid if it contains our tag if it has the namespace for it.
    for each in nids {
        for tag in client::relationship_get_fileid(each) {
            file_ids.remove(&tag);
        }
    }

    // Logs info to screen
    client::log(format!(
        "BlurHash - We've got {} files to parse.",
        file_ids.len()
    ));

    file_ids.par_iter().for_each(|fid| {
        if let Some(fbyte) = client::get_file(*fid.0) {
            let byte = std::fs::read(fbyte).unwrap();

            if let Ok(img) = image::load_from_memory(&byte[..]) {
                let string_blurhash = downloadparse(img);
                client::log_no_print(format!(
                    "BlurHash - Adding fid: {} to blurhash HASH: {}",
                    &fid.0, &string_blurhash
                ));
                let tagid = client::tag_add(string_blurhash, utable, true, None);
                client::relationship_add(*fid.0, tagid, true);
            } else {
                client::log_no_print(format!(
                    "{} Cannot load FID: {} as an image.",
                    PLUGIN_NAME, &fid.0
                ));
            }
        }
    });
    client::setting_add(
        format!("{}-shouldrun", PLUGIN_NAME).into(),
        format!(
            "From plugin {} {} Determines if we should run",
            PLUGIN_NAME, PLUGIN_DESCRIPTION
        )
        .into(),
        None,
        Some("False".to_string()),
        true,
    );
    client::transaction_flush();
}

#[no_mangle]
//pub fn OnDownload(byteCursor: Cursor<Bytes>, Hash: &String, Ext: &String, datab: Arc<Mutex<database::Main>>) {
pub fn on_download(
    byte_c: &[u8],
    hash_in: &String,
    ext_in: &String,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut output = Vec::new();

    let lmimg = image::load_from_memory(byte_c);
    match lmimg {
        Ok(good_lmimg) => {
            let string_blurhash = downloadparse(good_lmimg);

            let plugin_output = sharedtypes::DBPluginOutput {
                file: Some(vec![sharedtypes::PluginFileObj {
                    id: None,
                    hash: Some(hash_in.to_string()),
                    ext: Some(ext_in.to_string()),
                    location: None,
                }]),
                jobs: None,
                namespace: Some(vec![sharedtypes::DbPluginNamespace {
                    name: DB_NAME.to_string(),
                    description: Some(PLUGIN_DESCRIPTION.to_string()),
                }]),
                parents: None,
                setting: None,
                tag: Some(vec![sharedtypes::DBPluginTagOut {
                    name: string_blurhash.to_string(),
                    namespace: DB_NAME.to_string(),
                    parents: None,
                }]),
                relationship: Some(vec![sharedtypes::DbPluginRelationshipObj {
                    file_hash: hash_in.to_string(),
                    tag_name: string_blurhash,
                    tag_namespace: DB_NAME.to_string(),
                }]),
            };

            output.push(sharedtypes::DBPluginOutputEnum::Add(vec![plugin_output]));
        }
        Err(err) => {
            client::log_no_print(format!(
                "Plugin: blurhash -- Failed to load: {}, {:?}",
                hash_in, err
            ));
        }
    }
    output
}
