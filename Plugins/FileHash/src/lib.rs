use std::collections::HashSet;
use std::fs::metadata;
use std::time::{Duration, UNIX_EPOCH};
use struct_iterable::Iterable;
use strum::{EnumIter, IntoEnumIterator};

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/file.rs"]
mod file;
#[path = "../../../src/scr/db/helpers.rs"]
mod helpers;
static PLUGIN_NAME: &str = "File Hash";
static PLUGIN_DESCRIPTION: &str = "Gets hash information from a file.";

#[no_mangle]
pub fn return_info() -> sharedtypes::PluginInfo {
    let callbackvec = vec![
        sharedtypes::PluginCallback::OnStart,
        sharedtypes::PluginCallback::OnDownload,
    ];
    sharedtypes::PluginInfo {
        name: PLUGIN_NAME.to_string(),
        description: PLUGIN_DESCRIPTION.to_string(),
        version: 1.00,
        api_version: 1.00,
        callbacks: callbackvec,
        communication: Some(sharedtypes::PluginSharedData {
            thread: sharedtypes::PluginThreadType::Inline,
            com_channel: Some(sharedtypes::PluginCommunicationChannel::pipe(
                "beans".to_string(),
            )),
        }),
    }
}

#[no_mangle]
pub fn on_download(
    byte_c: &[u8],
    hash_in: &String,
    ext_in: &String,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut output = Vec::new();
    /*
    let lmimg = image::load_from_memory(byte_c);
    match lmimg {
        Ok(img) => {
            let width = img.width();
            let height = img.height();
            /*let width_output = sharedtypes::DBPluginOutput {
                file: Some(vec![sharedtypes::PluginFileObj {
                    id: None,
                    hash: Some(hash_in.to_string()),
                    ext: Some(ext_in.to_string()),
                    location: None,
                }]),
                jobs: None,
                namespace: Some(vec![sharedtypes::DbPluginNamespace {
                    name: get_set(Supset::Width).name,
                    description: get_set(Supset::Width).description,
                }]),
                parents: None,
                setting: None,
                tag: Some(vec![sharedtypes::DBPluginTagOut {
                    name: width.to_string(),
                    namespace: get_set(Supset::Width).name,
                    parents: None,
                }]),
                relationship: Some(vec![sharedtypes::DbPluginRelationshipObj {
                    file_hash: hash_in.to_string(),
                    tag_name: width.to_string(),
                    tag_namespace: get_set(Supset::Width).name,
                }]),
            };
            let height_output = sharedtypes::DBPluginOutput {
                file: Some(vec![sharedtypes::PluginFileObj {
                    id: None,
                    hash: Some(hash_in.to_string()),
                    ext: Some(ext_in.to_string()),
                    location: None,
                }]),
                jobs: None,
                namespace: Some(vec![sharedtypes::DbPluginNamespace {
                    name: get_set(Supset::Height).name,
                    description: get_set(Supset::Height).description,
                }]),
                parents: None,
                setting: None,
                tag: Some(vec![sharedtypes::DBPluginTagOut {
                    name: height.to_string(),
                    namespace: get_set(Supset::Height).name,
                    parents: None,
                }]),
                relationship: Some(vec![sharedtypes::DbPluginRelationshipObj {
                    file_hash: hash_in.to_string(),
                    tag_name: height.to_string(),
                    tag_namespace: get_set(Supset::Height).name,
                }]),
            };

            output.push(sharedtypes::DBPluginOutputEnum::Add(vec![
                width_output,
                height_output,
            ]));*/
        }
        Err(_) => {
            client::log(format!("FileInfo - Couldn't parse size from: {}", hash_in));
        }
    }*/

    output
}

#[no_mangle]
pub fn on_start() {
    println!("From FileHash plugin");
    check_existing_db();
    client::transaction_flush();
}

struct SettingInfo {
    name: String,
    description: Option<String>,
}

#[derive(EnumIter, PartialEq, Clone, Copy, Debug)]
enum Supset {
    MD5,
    SHA1,
    SHA256,
    SHA512,
    IPFSCID,
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
        },
    }
}
///
/// Checks and creates tables if not existing.
///
fn check_existing_db_table(table: TableData) -> usize {
    let bns = client::namespace_get(table.name.to_string());
    let uns = match bns {
        None => client::namespace_put(table.name, table.description, true),
        Some(id) => id,
    };
    client::transaction_flush();
    uns
}

fn check_existing_db() {
    use rayon::prelude::*;

    let table = sharedtypes::LoadDBTable::Namespace;
    client::load_table(table);
    let table = sharedtypes::LoadDBTable::Files;
    client::load_table(table);

    let table = sharedtypes::LoadDBTable::Relationship;
    client::load_table(table);
    let table = sharedtypes::LoadDBTable::Tags;
    client::load_table(table);

    let file_ids = client::file_get_list_all();
    'mainloop: for table in Supset::iter() {
        let mut name = client::settings_get_name(get_set(table).name);
        if let None = name {
            client::setting_add(
                get_set(table).name,
                get_set(table).description,
                None,
                Some("True".to_string()),
                true,
            );
            client::transaction_flush();
            name = client::settings_get_name(get_set(table).name);
        }
        // Continues the loop if this has already been checked.
        if let Some(nam) = &name {
            if nam.param.clone().unwrap() == "False" {
                continue 'mainloop;
            }
        }
        let mut total = file_ids.clone();
        let ctab = TableData {
            name: get_set(table).name,
            description: get_set(table).description,
        };
        let utable = check_existing_db_table(ctab);
        let mut hutable = client::namespace_get_tagids(utable);
        let huetable = match hutable {
            None => HashSet::new(),
            Some(set) => set,
        };
        for each in huetable {
            if let Some(tags) = client::relationship_get_fileid(each) {
                for tag in tags {
                    total.remove(&tag);
                }
            }
        }

        // Logs info. into system
        if total.is_empty() {
            client::log_no_print(format!(
                "FileHash - we've got {} files to parse for {}.",
                total.len(),
                get_set(table).name
            ));
        } else {
            client::log(format!(
                "FileHash - we've got {} files to parse for {}.",
                total.len(),
                get_set(table).name
            ));
        }

        // Puts file ids into a vec that will be parallely processed.
        let fids: Vec<&usize> = Vec::from_iter(total.keys());
        fids.par_iter().for_each(|each| {
            if let Some(byte) = client::get_file_bytes(**each) {
                let hash = hash_file(table, &byte);
                if let Some(hash) = hash {
                    client::log_no_print(format!(
                        "FileHash - Hashtype: {:?} Hash: {} Fileid: {}",
                        &table, &hash, **each
                    ));
                    let tid = client::tag_add(hash, utable, true, None);
                    client::relationship_add_db(**each, tid, true);
                }
            }
        });
        client::transaction_flush();
    }
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
            return Some(hash);
        }
        Supset::SHA1 => {
            let mut hasher = Sha1::new();
            hasher.update(byte);
            let hash = hex::encode(hasher.finalize());
            return Some(hash);
        }
        Supset::SHA256 => {
            let mut hasher = Sha256::new();
            hasher.update(byte);
            let hash = hex::encode(hasher.finalize());
            return Some(hash);
        }
        Supset::SHA512 => {
            let mut hasher = Sha512::new();
            hasher.update(byte);
            let hash = hex::encode(hasher.finalize());
            return Some(hash);
        }
        Supset::IPFSCID => {
            if let Ok(cid) = ipfs_cid::generate_cid_hash(byte) {
                return Some(cid);
            }

            return None;
        }
    }
}
