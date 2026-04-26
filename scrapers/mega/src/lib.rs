use async_std::task;
use sysinfo::{MemoryRefreshKind, System};

use chrono::{DateTime, Utc};
use mega::{Node, Nodes};
use regex::Regex;
use reqwest_middleware::ClientBuilder;
use reqwest_proxy_pool::{
    ProxyPoolConfig, ProxyPoolMiddleware, ProxySelectionStrategy, config::ProxySource,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

#[path = "../../../src/client.rs"]
mod client;
#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;
pub const REGEX_COLLECTIONS: &str =
    r"https?://mega\.nz/(folder/|file/|#F?!)[a-zA-Z0-9\-_]*(#|!)[a-zA-Z0-9\-_]*";

pub const DEFAULT_SITE: &str = "mega";
// Time in seconds to cache the result
pub const DEFAULT_CACHE: Option<usize> = Some(600);

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let tag_vec = (
        Some(sharedtypes::SearchType::Regex(REGEX_COLLECTIONS.into())),
        vec![],
        vec![
            "source_url".to_string(),
            "Mega Scraper".into(),
            "Mega_Source".into(),
            "Mega_Timestamp_Url".into(),
        ],
    );

    let callbackvec = vec![
        sharedtypes::GlobalCallbacks::Tag(tag_vec),
        sharedtypes::GlobalCallbacks::Start(sharedtypes::StartupThreadType::SpawnInline),
    ];

    let mut plugin = sharedtypes::return_default_globalpluginparser();
    plugin.name = "Mega Regex Parser".to_string();
    plugin.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_channel: true,
            redirect: Some("mega".into()),
        },
    ));
    plugin.callbacks = callbackvec;

    let mut scraper = sharedtypes::return_default_globalpluginparser();
    scraper.name = "Mega Scraper".into();
    scraper.should_handle_text_scraping = true;
    scraper.should_send_files_on_scrape = true;
    scraper.should_handle_file_download = true;
    scraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec!["mega".into(), "mega.nz".into()],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![],
        },
    ));
    vec![plugin, scraper]
}

///
/// Finds URLs indside of the params.
/// Not nessisarly needed but it is nice to have a pre-filter
///
#[unsafe(no_mangle)]
pub fn url_dump(
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperDataReturn> {
    let mut out = Vec::new();

    let regex = Regex::new(REGEX_COLLECTIONS).unwrap();

    let mut scraperdata = scraperdata.clone();
    scraperdata.job.param.clear();

    for param in params {
        if let sharedtypes::ScraperParam::Normal(temp) | sharedtypes::ScraperParam::Url(temp) =
            param
        {
            for item_match in regex.find_iter(temp).map(|c| c.as_str()) {
                let mut sc = scraperdata.clone();
                sc.job
                    .param
                    .push(sharedtypes::ScraperParam::Url(item_match.into()));
                out.push(sharedtypes::ScraperDataReturn {
                    job: sc.job.clone(),
                    ..Default::default()
                });
            }
        }
    }

    out
}

fn get_last_modification_time(nodes: &Nodes) -> Option<DateTime<Utc>> {
    let mut out = None;

    for node in nodes.iter() {
        if node.created_at() > out && nodes.len() != 1 {
            out = node.created_at();
        } else if node.modified_at() > out {
            out = node.modified_at()
        }
    }

    out
}

///
/// Filters out the dirs and files from a node
///
fn find_sub_dirs_and_files(
    parent_node: &Nodes,
    parent: &MegaDir,
    filelist: &mut HashSet<sharedtypes::FileObject>,
    taglist: &mut HashSet<sharedtypes::TagObject>,
    url_input: &str,
    client: &mega::Client,
    flag: &mut Vec<sharedtypes::ScraperFlags>,
    cnt: &mut i32,
    mod_time: sharedtypes::Tag,
) -> Vec<MegaDirOrFile> {
    let out = Arc::new(Mutex::new(Vec::new()));
    let file_arc = Arc::new(Mutex::new(HashSet::new()));
    let tag_arc = Arc::new(Mutex::new(HashSet::new()));

    // Needed to get memory from system
    let mut sys = System::new_all();

    //parent.children.iter().for_each(|child| {
    for child in parent.children.iter() {
        if let Some(node) = parent_node.get_node_by_handle(child) {
            match node.kind() {
                mega::NodeKind::File => {
                    let mut fpath = parent
                        .parent
                        .iter()
                        .map(|i| format!("/{}", i))
                        .collect::<String>();
                    fpath += &format!("/{}", node.name());

                    let sub_link = sharedtypes::SubTag {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Mega_Name".into(),
                            description: Some("A filepath inside a Mega archive".into()),
                        },
                        tag: fpath.clone(),
                        tag_type: sharedtypes::TagType::Normal,
                        limit_to: Some(mod_time.clone()),
                    };

                    let tags = vec![sharedtypes::TagObject {
                        tag: node.handle().into(),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: Some(sub_link),
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Mega_Handle".into(),
                            description: Some(
                                "A individual handle that links a file to a dir in mega".into(),
                            ),
                        },
                    }];
                    for tag in tags {
                        tag_arc.lock().unwrap().insert(tag);
                    }

                    if let Some(namespace_id) = client::namespace_get("Mega_Handle".into()) {
                        let mut search = true;
                        if let Some(tid) = client::tag_get_name(node.handle().into(), namespace_id)
                            && !client::relationship_get_fileid(tid).is_empty()
                        {
                            search = false;
                        }

                        if search {
                            client::log(format!("MEGA - Downloading: {}", &fpath));

                            sys.refresh_memory_specifics(MemoryRefreshKind::everything());

                            // Check if a files size is greatet then amount of availble system memory.
                            // Want to leave 1/2 of memory available because I dont want to estop
                            // the system.
                            // The node size is in bytes but the sys used memory is in kb.
                            if node.size() > sys.free_memory() {
                                flag.push(sharedtypes::ScraperFlags::Redo);
                                client::log(format!(
                                    "Stopping due to Memory being too much: {}",
                                    cnt
                                ));
                                break;
                            }

                            download_file(
                                client,
                                file_arc.clone(),
                                tag_arc.clone(),
                                url_input,
                                node,
                                &fpath,
                                mod_time.clone(),
                            );
                            client::log(format!("MEGA - Downloaded: {}", &fpath));
                            /*out.lock().unwrap().push(MegaDirOrFile::File(MegaFile {
                                file_path: fpath,
                                file_handle: node.handle().to_string(),
                            }));*/
                            /* if *cnt >= stop {
                                flag.push(sharedtypes::Flags::Redo);
                                client::log(format!("Stopping due to cnt: {}", cnt));
                                break;
                            }
                            *cnt += 1;*/
                        }
                    }
                }
                mega::NodeKind::Folder => {
                    let mut parent = parent.parent.clone();
                    parent.push(node.name().into());
                    out.lock().unwrap().push(MegaDirOrFile::Dir(MegaDir {
                        //               name: node.name().into(),
                        parent,
                        children: node.children().to_vec(),
                    }));
                }
                _ => {}
            }
        }
    }
    let temp = Arc::try_unwrap(file_arc).unwrap();
    for file in temp.into_inner().unwrap() {
        filelist.insert(file);
    }
    let temp = Arc::try_unwrap(tag_arc).unwrap();
    for file in temp.into_inner().unwrap() {
        taglist.insert(file);
    }

    out.lock().unwrap().to_vec()
}

///
/// Function runs when the system matches our regex
///
#[unsafe(no_mangle)]
pub fn on_regex_match(
    _tag_name: &str,
    _tag_namespace: &sharedtypes::GenericNamespaceObj,
    regex_match: &str,
    _plugin_callback: &Option<sharedtypes::SearchType>,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let job = sharedtypes::DbJobsObj {
        site: DEFAULT_SITE.to_string(),
        param: vec![sharedtypes::ScraperParam::Normal(regex_match.into())],
        jobmanager: sharedtypes::DbJobsManager {
            jobtype: sharedtypes::DbJobType::Params,
            recreation: None,
        },
        priority: sharedtypes::DEFAULT_PRIORITY - 2,
        ..Default::default()
    };

    let out = vec![sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            jobs: vec![job],
            ..Default::default()
        },
    ])];

    out
}

///
/// Handles downloading and parsing of mega "files"
///
#[unsafe(no_mangle)]
pub fn text_scraping(
    url_input: &str,
    _params: &[sharedtypes::ScraperParam],
    _scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut cnt = 0;
    let mut files = HashSet::new();
    let mut tags = HashSet::new();
    let mut fileordir = Vec::new();
    let mut flags = Vec::new();

    let cli = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap();

    let http_client = ClientBuilder::new(cli).build();

    let client = mega::Client::builder()
        .timeout(Some(Duration::from_secs(240)))
        .build(http_client)
        .unwrap();

    let nodes_future = client.fetch_public_nodes(url_input);
    let nref = task::block_on(nodes_future);

    if let Ok(ref nodes) = nref {
        client::log("Got proper nodes for the url".to_string());

        // Extract modification time for the metadata tag
        let modtime = get_last_modification_time(nodes).map(|time| sharedtypes::Tag {
            tag: time.timestamp_millis().to_string(),
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Mega_Last_Modified".to_string(),
                description: Some("Last modification time in mega archive.".into()),
            },
        });

        // Setup the master timestamp linkage
        let joined_url_time = if let Some(ref mod_time) = modtime {
            let time_str = mod_time.tag.clone();
            let timestamp_tag = sharedtypes::Tag {
                tag: format!("{}:{}", time_str, url_input),
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Mega_Timestamp_Url".to_string(),
                    description: Some("Linkage between timestamp and url.".into()),
                },
            };

            tags.insert(sharedtypes::TagObject {
                tag: time_str,
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(sharedtypes::SubTag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Source".into(),
                        description: Some("A mega link.".into()),
                    },
                    tag: url_input.into(),
                    limit_to: Some(timestamp_tag.clone()),
                    tag_type: sharedtypes::TagType::Normal,
                }),
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Mega_Timestamp".into(),
                    description: Some("Folder edit timestamp.".into()),
                },
            });
            timestamp_tag
        } else {
            // If we can't get a modtime, we can't reliably track changes
            return vec![sharedtypes::ScraperReturn::Nothing];
        };

        for root in nodes.roots() {
            // CASE 1: The link is a FOLDER
            if root.kind().is_folder() {
                let root_dir = MegaDir {
                    parent: vec![root.name().into()],
                    children: root.children().to_vec(),
                };

                let mut dir_stack = vec![root_dir];
                while let Some(current_dir) = dir_stack.pop() {
                    let results = find_sub_dirs_and_files(
                        nodes,
                        &current_dir,
                        &mut files,
                        &mut tags,
                        url_input,
                        &client,
                        &mut flags,
                        &mut cnt,
                        joined_url_time.clone(),
                    );

                    for item in results {
                        if let MegaDirOrFile::Dir(ref dir) = item {
                            dir_stack.push(dir.clone());
                        }
                        fileordir.push(item);
                    }
                }
            }

            // CASE 2: The link is an INDIVIDUAL FILE
            if root.kind().is_file() {
                client::log(format!("Processing single file: {}", root.name()));

                let handle: String = root.handle().into();

                // Prepare metadata tags for this specific file
                let file_name_tag = sharedtypes::SubTag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Name".into(),
                        description: Some("Filepath/Name in Mega".into()),
                    },
                    tag: root.name().into(),
                    tag_type: sharedtypes::TagType::Normal,
                    limit_to: Some(joined_url_time.clone()),
                };

                let handle_tag_obj = sharedtypes::TagObject {
                    tag: handle.clone(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: Some(file_name_tag),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Handle".into(),
                        description: Some("Individual file handle.".into()),
                    },
                };

                // Add handle to global tags so the FileObject picks it up
                tags.insert(handle_tag_obj);

                // Check if already downloaded via namespace
                let mut should_download = true;
                if let Some(ns_id) = client::namespace_get("Mega_Handle".into()) {
                    if client::tag_get_name(handle.clone(), ns_id).is_some() {
                        should_download = false;
                        client::log("File already exists, skipping download...".into());
                    }
                }

                if should_download {
                    let mut buffer = Vec::new();
                    if let Ok(_) = task::block_on(client.download_node(root, &mut buffer)) {
                        cnt += 1;

                        let tag_list = vec![sharedtypes::FileTagAction {
                            operation: sharedtypes::TagOperation::Add,
                            tags: tags.clone().into_iter().collect(),
                        }];

                        files.insert(sharedtypes::FileObject {
                            source: Some(sharedtypes::FileSource::Bytes(buffer)),
                            hash: sharedtypes::HashesSupported::None,
                            tag_list, // Now matches the expected type
                            skip_if: vec![sharedtypes::SkipIf::FileTagRelationship(
                                sharedtypes::Tag {
                                    tag: handle,
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: "Mega_Handle".into(),
                                        description: Some("Individual file handle.".into()),
                                    },
                                },
                            )],
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    if let Err(err) = nref {
        client::log(format!("Mega Error: {:?}", err));
    }

    vec![sharedtypes::ScraperReturn::Data(
        sharedtypes::ScraperObject {
            files,
            tags,
            flags,
            ..Default::default()
        },
    )]
}
fn download_file(
    client: &mega::Client,
    file_list: Arc<Mutex<HashSet<sharedtypes::FileObject>>>,
    _tag_list: Arc<Mutex<HashSet<sharedtypes::TagObject>>>,
    _url_input: &str,
    node: &Node,
    _localpath: &String,
    _mod_time: sharedtypes::Tag,
) {
    // Ghetto way to pre-filter if we've already downloaded a file
    let tag_list = vec![sharedtypes::FileTagAction {
        operation: sharedtypes::TagOperation::Add,
        tags: vec![sharedtypes::TagObject {
            tag: node.handle().into(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: None,
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Mega_Handle".into(),
                description: Some("A individual handle that links a file to a dir in mega".into()),
            },
        }],
    }];

    let mut err_num = 0;

    let mut temp = Vec::new();
    'errloop: loop {
        match task::block_on(client.download_node(node, &mut temp)) {
            Ok(_) => {
                file_list.lock().unwrap().insert(sharedtypes::FileObject {
                    source: Some(sharedtypes::FileSource::Bytes(temp)),
                    hash: sharedtypes::HashesSupported::None,
                    tag_list,
                    skip_if: vec![sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                        tag: node.handle().into(),
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Mega_Handle".into(),
                            description: Some(
                                "A individual handle that links a file to a dir in mega".into(),
                            ),
                        },
                    })],

                    ..Default::default()
                });
                break;
            }
            Err(err) => {
                err_num += 1;
                dbg!("Got ERROR", err);
                if err_num == 3 {
                    break 'errloop;
                }
            }
        };
    }
}

#[derive(Debug, Clone)]
struct MegaDir {
    parent: Vec<String>,
    children: Vec<String>,
}

/*#[derive(Debug, Clone)]
struct MegaFile {
    file_path: String,
    file_handle: String,
}*/

#[derive(Debug, Clone)]
enum MegaDirOrFile {
    Dir(MegaDir),
    //  File,
}

#[unsafe(no_mangle)]
pub fn download_from(_file: &sharedtypes::FileObject) -> Option<Vec<u8>> {
    None
}
