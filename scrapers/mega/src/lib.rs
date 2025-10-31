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

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;
pub const REGEX_COLLECTIONS: &str =
    r"https?://mega\.nz/(folder/|file/|#F?!)[a-zA-Z0-9\-_]*(#|!)[a-zA-Z0-9\-_]*";

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
        vec!["source_url".to_string(), "Mega Scraper".into()],
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
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = Vec::new();

    let regex = Regex::new(REGEX_COLLECTIONS).unwrap();

    let mut scraperdata = scraperdata.clone();
    scraperdata.job.job_type = sharedtypes::DbJobType::Scraper;
    scraperdata.job.param.clear();

    for param in params {
        if let sharedtypes::ScraperParam::Normal(temp) = param {
            for item_match in regex.find_iter(temp).map(|c| c.as_str()) {
                let mut sc = scraperdata.clone();
                sc.job
                    .param
                    .push(sharedtypes::ScraperParam::Url(item_match.into()));
                out.push((item_match.into(), sc));
            }
        }
    }

    out
}

fn get_last_modification_time(nodes: &Nodes) -> Option<DateTime<Utc>> {
    let mut out = None;

    for node in nodes.iter() {
        if node.created_at() > out {
            out = node.created_at();
        } else if node.modified_at() > out {
            out = node.modified_at()
        }
    }

    dbg!(&out);

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
    flag: &mut Vec<sharedtypes::Flags>,
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

                        if client::tag_get_name(node.handle().into(), namespace_id).is_none()
                            || search
                        {
                            client::log(format!("MEGA - Downloading: {}", &fpath));

                            sys.refresh_memory_specifics(MemoryRefreshKind::everything());

                            // Check if a files size is greatet then amount of availble system memory.
                            // Want to leave 1/2 of memory available because I dont want to estop
                            // the system.
                            // The node size is in bytes but the sys used memory is in kb.

                            if node.size() * 1000 > sys.used_memory() / 2 {
                                flag.push(sharedtypes::Flags::Redo);
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
                        } else {
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
    let mut job = sharedtypes::return_default_jobsobj();
    job.site = "mega".into();
    job.param = vec![sharedtypes::ScraperParam::Normal(regex_match.into())];
    job.jobmanager = sharedtypes::DbJobsManager {
        jobtype: sharedtypes::DbJobType::Params,
        recreation: None,
    };

    job.priority = 0;

    let out = vec![sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: vec![],
            setting: vec![],
            relationship: vec![],
            jobs: vec![job],
            file: vec![],
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
    _scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut cnt = 0;

    let mut file = HashSet::new();
    let mut tag = HashSet::new();

    let mut fileordir = Vec::new();
    let mut flag = Vec::new();

    client::log("Starting pool init".to_string());

    /* let config = ProxyPoolConfig::builder()
            // free socks5 proxy urls, format like `Free-Proxy`
            .sources(
                /*vec![
                    ProxySource::Proxy("socks5://192.241.156.17:1080".to_string()),
                    ProxySource::Proxy("socks5://134.199.159.23:1080".to_string()),
                    ProxySource::Proxy("socks5://142.54.226.214:4145".to_string()),
                    ProxySource::Proxy("socks5://184.178.172.28:15294".to_string()),
                    ProxySource::Proxy("socks5://184.178.172.13:15311".to_string()),
                    ProxySource::Proxy("socks5://98.188.47.132:4145".to_string()),
                    ProxySource::Proxy("socks5://192.169.140.98:45739".to_string()),
                    ProxySource::Proxy("socks5://8.218.217.168:1100".to_string()),
                    ProxySource::Proxy("socks5://47.250.157.116:1100".to_string()),
                    ProxySource::Proxy("socks5://68.191.23.134:9200".to_string()),
                    ProxySource::Proxy("socks5://8.219.119.119:1024".to_string()),
                    ProxySource::Proxy("socks5://94.254.244.251:8192".to_string()),
                    //ProxySource::Proxy("socks5://:".to_string()),
                ]*/
            )
            .health_check_timeout(Duration::from_secs(10))
            .retry_count(2)
            .selection_strategy(ProxySelectionStrategy::FastestResponse)
            // rate limit for each proxy, lower performance but avoid banned
            .max_requests_per_second(3.0)
            .build();

        let proxy_pool = task::block_on(ProxyPoolMiddleware::new(config)).unwrap();
    */
    let cli = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap();

    let http_client = ClientBuilder::new(cli)
        //.with(proxy_pool)
        .build();

    //let http_client = Client::builder().proxy().build().unwrap()

    let client = mega::Client::builder()
        .timeout(Some(Duration::from_secs(240)))
        .build(http_client)
        .unwrap();

    let nodes = client.fetch_public_nodes(url_input);

    // Weird workaround for if its your first time running this garbage
    if client::namespace_get("Mega_Handle".into()).is_none() {
        client::namespace_put(
            "Mega_Handle".into(),
            Some("A individual handle that links a file to a dir in mega".into()),
        );
    }

    client::log(format!("Starting processing for {}", url_input));

    let nref = task::block_on(nodes);

    if let Ok(ref nodes) = nref {
        client::log("Got proper logs for the url".to_string());

        let modtime = get_last_modification_time(nodes).map(|time| sharedtypes::Tag {
            tag: time.timestamp_millis().to_string(),
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Mega_Last_Modified".to_string(),
                description: Some(
                    "The last time a file was editied or modified inside of a mega folder."
                        .to_string(),
                ),
            },
        });

        for root in nodes.roots() {
            // Inserts a master link between the timestamp and the archive
            let joined_url_time;
            if let Some(ref mod_time) = modtime {
                let time = mod_time.tag.clone();
                joined_url_time = sharedtypes::Tag {
                    tag: format!("{}:{}", time, url_input),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Timestamp_Url".to_string(),
                        description: Some(
                            "A Unique linkage that ties a modification timestamp to a url."
                                .to_string(),
                        ),
                    },
                };

                let subtag = sharedtypes::SubTag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Source".into(),
                        description: Some("A mega link.".into()),
                    },
                    tag: url_input.into(),
                    limit_to: Some(joined_url_time.clone()),
                    tag_type: sharedtypes::TagType::Normal,
                };

                tag.insert(sharedtypes::TagObject {
                    tag: time,
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: Some(subtag.clone()),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Timestamp".into(),
                        description: Some(
                            "A unique tmestamp that represents the last time a folder was edited in mega.".into(),
                        ),
                    },
                });
            } else {
                return Err(sharedtypes::ScraperReturn::Nothing);
            }

            let rootnew = MegaDir {
                //        name: root.name().into(),
                parent: vec![root.name().into()],
                children: root.children().to_vec(),
            };

            let mut rootloop = vec![rootnew];

            loop {
                if rootloop.is_empty() {
                    client::log("Stopping due to no more directories to search".to_string());
                    break;
                }
                let rootnew = rootloop.pop().unwrap();

                for item in find_sub_dirs_and_files(
                    nodes,
                    &rootnew,
                    &mut file,
                    &mut tag,
                    url_input,
                    &client,
                    &mut flag,
                    &mut cnt,
                    joined_url_time.clone(),
                ) {
                    let MegaDirOrFile::Dir(ref dir) = item;
                    rootloop.push(dir.clone());

                    fileordir.push(item);
                }
                if !flag.is_empty() {
                    client::log("Stopping due to flags are empty".to_string());
                    break;
                }
            }
            if root.kind().is_file() {
                //let megafile = MegaFile {
                //    file_path: format!("/{}", root.name()),
                //    file_handle: root.handle().into(),
                //};

                let sub_link = sharedtypes::SubTag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Name".into(),
                        description: Some("A filepath inside a Mega archive".into()),
                    },
                    tag: root.name().into(),
                    tag_type: sharedtypes::TagType::Normal,
                    limit_to: Some(joined_url_time.clone()),
                };

                let tags = vec![sharedtypes::TagObject {
                    tag: root.handle().into(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: Some(sub_link),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Handle".into(),
                        description: Some(
                            "A individual handle that links a file to a dir in mega".into(),
                        ),
                    },
                }];

                // Weird workaround for if its your first time running this garbage
                if client::namespace_get("Mega_Handle".into()).is_none() {
                    client::namespace_put(
                        "Mega_Handle".into(),
                        Some("A individual handle that links a file to a dir in mega".into()),
                    );
                }

                // Ghetto way to pre-filter if we've already downloaded a file
                if let Some(namespace_id) = client::namespace_get("Mega_Handle".into()) {
                    if client::tag_get_name(root.handle().into(), namespace_id).is_none() {
                        /*let mut temp = Vec::new();
                            task::block_on(client.download_node(root, &mut temp)).unwrap();
                            cnt += 1;
                            file.insert(sharedtypes::FileObject {
                                source: Some(sharedtypes::FileSource::Bytes(temp)),
                                hash: sharedtypes::HashesSupported::None,
                                tag_list: tagz,
                                skip_if:
                                    vec![sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                            tag: root.handle().into(),
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: "Mega_Handle".into(),
                                description: Some(
                                    "A individual handle that links a file to a dir in mega".into(),
                                ),
                            },
                        })],
                            });

                            if stop_count == cnt {
                                client::log(format!("Stopping loop due to cnt: {}", cnt));
                                return Err(sharedtypes::ScraperReturn::Timeout(0));
                            }*/

                        //fileordir.push(MegaDirOrFile::File(megafile));
                    } else {
                        for tags in tags {
                            tag.insert(tags);
                        }
                    }
                }
            }
        }
    }
    if let Err(err) = nref {
        dbg!(&err);
    }
    for file in file.iter() {
        dbg!(&file.tag_list);
    }
    Ok(sharedtypes::ScraperObject { file, tag, flag })
}

fn download_file(
    client: &mega::Client,
    file_list: Arc<Mutex<HashSet<sharedtypes::FileObject>>>,
    tag_list: Arc<Mutex<HashSet<sharedtypes::TagObject>>>,
    url_input: &str,
    node: &Node,
    localpath: &String,
    mod_time: sharedtypes::Tag,
) {
    // Ghetto way to pre-filter if we've already downloaded a file
    if let Some(namespace_id) = client::namespace_get("Mega_Handle".into()) {
        let tags = vec![sharedtypes::TagObject {
            tag: node.handle().into(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: None,
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Mega_Handle".into(),
                description: Some("A individual handle that links a file to a dir in mega".into()),
            },
        }];

        let mut err_num = 0;

        if client::tag_get_name(node.handle().into(), namespace_id).is_none() {
            let mut temp = Vec::new();
            'errloop: loop {
                match task::block_on(client.download_node(node, &mut temp)) {
                    Ok(_) => {
                        err_num = 0;
                        file_list.lock().unwrap().insert(sharedtypes::FileObject {
                            source: Some(sharedtypes::FileSource::Bytes(temp)),
                            hash: sharedtypes::HashesSupported::None,
                            tag_list: tags,
                            skip_if:
                                vec![sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                        tag: node.handle().into(),
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Mega_Handle".into(),
                            description: Some(
                                "A individual handle that links a file to a dir in mega".into(),
                            ),
                        },
                    })],
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
