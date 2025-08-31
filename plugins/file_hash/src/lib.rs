use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use strum::{EnumIter, IntoEnumIterator};

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
static PLUGIN_NAME: &str = "File Hash";

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
        sharedtypes::GlobalCallbacks::Start(sharedtypes::StartupThreadType::SpawnInline),
        sharedtypes::GlobalCallbacks::Download,
        sharedtypes::GlobalCallbacks::Import,
    ];
    let out = vec![main];

    out
}

#[no_mangle]
pub fn on_import(byte_c: &[u8], hash_in: &String) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut output = Vec::new();
    for hash in Supset::iter() {
        let hastring = hash_file(hash, byte_c);
        if let Some(st) = hastring {
            let tag_output = sharedtypes::DBPluginOutput {
                file: Some(vec![sharedtypes::PluginFileObj {
                    id: None,
                    hash: Some(hash_in.to_owned()),
                    ext: None,
                    location: None,
                }]),
                jobs: None,
                namespace: Some(vec![sharedtypes::DbPluginNamespace {
                    name: get_set(hash).name,
                    description: get_set(hash).description,
                }]),
                parents: None,
                setting: None,
                tag: Some(vec![sharedtypes::DBPluginTagOut {
                    name: st.to_string(),
                    namespace: get_set(hash).name,
                    parents: None,
                }]),
                relationship: Some(vec![sharedtypes::DbPluginRelationshipObj {
                    file_hash: hash_in.to_owned(),
                    tag_name: st,
                    tag_namespace: get_set(hash.to_owned()).name,
                }]),
            };
            output.push(sharedtypes::DBPluginOutputEnum::Add(vec![tag_output]));
        }
    }

    output
}

#[no_mangle]
pub fn on_download(
    byte_c: &[u8],
    hash_in: &String,
    ext_in: &String,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut output = Vec::new();
    for hash in Supset::iter() {
        let hastring = hash_file(hash, byte_c);
        if let Some(st) = hastring {
            let tag_output = sharedtypes::DBPluginOutput {
                file: Some(vec![sharedtypes::PluginFileObj {
                    id: None,
                    hash: Some(hash_in.to_owned()),
                    ext: Some(ext_in.to_owned()),
                    location: None,
                }]),
                jobs: None,
                namespace: Some(vec![sharedtypes::DbPluginNamespace {
                    name: get_set(hash).name,
                    description: get_set(hash).description,
                }]),
                parents: None,
                setting: None,
                tag: Some(vec![sharedtypes::DBPluginTagOut {
                    name: st.to_string(),
                    namespace: get_set(hash).name,
                    parents: None,
                }]),
                relationship: Some(vec![sharedtypes::DbPluginRelationshipObj {
                    file_hash: hash_in.to_owned(),
                    tag_name: st,
                    tag_namespace: get_set(hash.to_owned()).name,
                }]),
            };
            output.push(sharedtypes::DBPluginOutputEnum::Add(vec![tag_output]));
        }
    }

    output
}

#[no_mangle]
pub fn on_start() {
    check_existing_db();
}

struct SettingInfo {
    name: String,
    description: Option<String>,
}

#[derive(EnumIter, PartialEq, Clone, Copy, Debug, Eq, Hash)]
enum Supset {
    MD5,
    SHA1,
    SHA256,
    SHA512,
    IPFSCID,
    IPFSCID1,
    IMAGEHASH,
}
///
/// Holder for data
///
struct TableData {
    name: String,
    description: Option<String>,
}

///
/// Gets info. holder for stuff
///
fn get_set(inp: Supset) -> SettingInfo {
    match inp {
        Supset::MD5 => SettingInfo {
            name: "FileHash-MD5".to_string(),
            description: Some("From plugin FileHash. MD5 hash of the file.".to_string()),
        },
        Supset::SHA1 => SettingInfo {
            name: "FileHash-SHA1".to_string(),
            description: Some("From plugin FileHash. SHA1 hash of the file.".to_string()),
        },
        Supset::SHA256 => SettingInfo {
            name: "FileHash-SHA256".to_string(),
            description: Some("From plugin FileHash. SHA256 hash of the file.".to_string()),
        },
        Supset::SHA512 => SettingInfo {
            name: "FileHash-SHA512".to_string(),
            description: Some("From plugin FileHash. SHA512 hash of the file.".to_string()),
        },
        Supset::IPFSCID => SettingInfo {
            name: "FileHash-IPFSCID".to_string(),
            description: Some("From plugin FileHash. IPFS Content ID of the file for usage with the IPFS network.".to_string()),
        },Supset::IPFSCID1 => SettingInfo {
            name: "FileHash-IPFSCID1".to_string(),
            description: Some("From plugin FileHash. IPFS Content ID of the file for usage with the IPFS network. Version 1 more modern".to_string()),
        },
            Supset::IMAGEHASH => SettingInfo {
            name: "FileHash-ImageHash".to_string(),
            description: Some("From plugin FileHash. PHash of the image. Used to deduplicate similar images if the hashes aren't the same".to_string())
        }

    }
}
///
/// Checks and creates tables if not existing.
///
fn check_existing_db_table(table: TableData) -> usize {
    let bns = client::namespace_get(table.name.to_string());
    let uns = match bns {
        None => client::namespace_put(table.name, table.description),
        Some(id) => id,
    };
    client::transaction_flush();
    uns
}

fn check_existing_db() {
    use rayon::prelude::*;

    let mut utable_storage: HashMap<Supset, usize> = HashMap::new();
    let mut utable_count: HashMap<Supset, usize> = HashMap::new();
    let mut modernstorage: HashMap<sharedtypes::DbFileObj, Vec<Supset>> = HashMap::new();

    let mut table_skip: Vec<Supset> = Vec::new();

    for table in Supset::iter() {
        utable_count.insert(table, 0);
    }
    'suploop: for table in Supset::iter() {
        match client::settings_get_name(get_set(table).name) {
            None => {
                client::setting_add(
                    get_set(table).name,
                    get_set(table).description,
                    None,
                    Some("True".to_string()),
                    true,
                );
                client::transaction_flush();
                client::settings_get_name(get_set(table).name).unwrap()
            }
            Some(name) => {
                if name.param == Some("False".to_string()) {
                    table_skip.push(table);
                    continue 'suploop;
                }
                name
            }
        };

        client::log(format!("Starting to process table: {:?}", &table));

        let table_temp = sharedtypes::LoadDBTable::Files;
        client::load_table(table_temp);
        let table_temp = sharedtypes::LoadDBTable::All;
        client::load_table(table_temp);

        let file_ids = client::file_get_list_all();
        let mut total = file_ids.clone();
        let ctab = TableData {
            name: get_set(table).name,
            description: get_set(table).description,
        };
        let utable = check_existing_db_table(ctab);
        utable_storage.insert(table, utable);
        let huetable = client::namespace_get_tagids(utable);

        for each in huetable {
            for tag in client::relationship_get_fileid(each) {
                total.remove(&tag);
            }
        }

        for item in &total {
            match item.1 {
                sharedtypes::DbFileStorage::Exist(fileobj) => {
                    match modernstorage.get_mut(fileobj) {
                        None => {
                            modernstorage.insert(fileobj.clone(), vec![table]);
                            *utable_count.get_mut(&table).unwrap() += 1;
                        }
                        Some(intf) => {
                            intf.push(table);
                            *utable_count.get_mut(&table).unwrap() += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        client::log(format!("Ended table loop for table: {:?}", &table));
    }
    let failed_id: Arc<Mutex<HashMap<Supset, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let hashed_id: Arc<Mutex<HashMap<Supset, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    for table in Supset::iter() {
        // Early exist for if the table neeeds to be skipped
        if table_skip.contains(&table) {
            client::log_no_print(format!(
                "FileHash - we've got 0 files to parse for {} skipping...",
                get_set(table).name
            ));

            continue;
        }
        let total = *utable_count.get(&table).unwrap();
        // Logs info. into system
        if total == 0 {
            client::log_no_print(format!(
                "FileHash - we've got {} files to parse for {}.",
                total,
                get_set(table).name
            ));
            client::setting_add(
                get_set(table).name,
                get_set(table).description,
                None,
                Some("False".to_string()),
                true,
            );
            client::transaction_flush();
        } else {
            client::log(format!(
                "FileHash - we've got {} files to parse for {}.",
                total,
                get_set(table).name
            ));
            failed_id.lock().unwrap().insert(table, 0);
        }
        hashed_id.lock().unwrap().insert(table, 0);
    }

    // Main loop paralel iterated for each file.
    modernstorage.par_iter().for_each(|modern| {
        if let Some(fbyte) = client::get_file(modern.0.id) {
            let byte = std::fs::read(fbyte).unwrap();
            for hashtype in modern.1 {
                if let Some(hash) = hash_file(*hashtype, &byte) {
                    client::log_no_print(format!(
                        "FileHash - Hashtype: {:?} Hash: {} Fileid: {}",
                        &hashtype, &hash, modern.0.id
                    ));
                    let tid =
                        client::tag_add(hash, *utable_storage.get(&hashtype).unwrap(), true, None);
                    client::relationship_add(modern.0.id, tid);
                    let mut hashed_lock = hashed_id.lock().unwrap();

                    if let Some(hashed_number) = hashed_lock.get_mut(hashtype) {
                        *hashed_number += 1;
                    }
                } else {
                    let mut failed_lock = failed_id.lock().unwrap();
                    if let Some(failed_number) = failed_lock.get_mut(hashtype) {
                        *failed_number += 1;
                    }
                }
            }
        } else {
            client::log(format!(
                "FileHash - Couldn't find: {}   {}",
                modern.0.id, modern.0.hash
            ));
        }
    });

    // Error Checking if we've completed all tables
    let failed_lock = failed_id.lock().unwrap();
    let hashed_lock = hashed_id.lock().unwrap();

    for table in Supset::iter() {
        let failed_total = failed_lock.get(&table).unwrap_or(&0);
        let hashed_total = hashed_lock.get(&table).unwrap_or(&0);
        let utable_total = utable_count.get(&table).unwrap_or(&0);
        if failed_total + hashed_total == *utable_total {
            match client::settings_get_name(get_set(table).name) {
                None => {
                    client::setting_add(
                        get_set(table).name,
                        get_set(table).description,
                        None,
                        Some("True".to_string()),
                        true,
                    );
                }
                Some(name) => {
                    if name.param == Some("True".to_string()) {
                        client::setting_add(
                            get_set(table).name,
                            get_set(table).description,
                            None,
                            Some("False".to_string()),
                            true,
                        );
                    }
                }
            };
        }
    }

    client::transaction_flush();
}

///
/// Hashes a file with the selected hash type.
/// outputs has as a string or an option string.
///
fn hash_file(hashtype: Supset, byte: &[u8]) -> Option<String> {
    use md5::Md5;
    use sha1::{Digest, Sha1};
    use sha2::{Sha256, Sha512};
    match hashtype {
        Supset::MD5 => {
            let mut hasher = Md5::new();
            hasher.update(byte);

            let hash = hex::encode(hasher.finalize());
            Some(hash)
        }
        Supset::SHA1 => {
            let mut hasher = Sha1::new();
            hasher.update(byte);
            let hash = hex::encode(hasher.finalize());
            Some(hash)
        }
        Supset::SHA256 => {
            let mut hasher = Sha256::new();
            hasher.update(byte);
            let hash = hex::encode(hasher.finalize());
            Some(hash)
        }
        Supset::SHA512 => {
            let mut hasher = Sha512::new();
            hasher.update(byte);
            let hash = hex::encode(hasher.finalize());
            Some(hash)
        }
        Supset::IPFSCID => {
            if let Ok(cid) = ipfs_cid::generate_cid_v0(byte) {
                return Some(cid);
            }

            None
        }
        Supset::IPFSCID1 => {
            if let Ok(cid) = ipfs_cid::generate_cid_v1(byte) {
                return Some(cid);
            }

            None
        }
        Supset::IMAGEHASH => {
            use image_hasher::BitOrder;
            use image_hasher::HasherConfig;
            use std::io::Cursor;

            let hasher = HasherConfig::new()
                .hash_alg(image_hasher::HashAlg::Median)
                .bit_order(BitOrder::MsbFirst)
                .preproc_dct()
                .to_hasher();
            if let Ok(img) = image::ImageReader::new(Cursor::new(byte)).with_guessed_format() {
                if let Ok(decode) = img.decode() {
                    return Some(hasher.hash_image(&decode).to_base64());
                }
            }
            None
        }
    }
}
