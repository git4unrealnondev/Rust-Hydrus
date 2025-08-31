use async_std::task;
use mega::{Node, Nodes};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use reqwest::Proxy;
use std::{
    collections::{HashMap, HashSet},
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
    params: &Vec<sharedtypes::ScraperParam>,
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

///
/// Filters out the dirs and files from a node
///
fn find_sub_dirs_and_files(
    parent_node: &Nodes,
    parent: &MegaDir,
    filelist: &mut HashSet<sharedtypes::FileObject>,
    taglist: &mut HashSet<sharedtypes::TagObject>,
    url_input: &String,
    client: &mega::Client,
    flag: &mut Vec<sharedtypes::Flags>,
    cnt: &mut i32,
) -> Vec<MegaDirOrFile> {
    let mut out = Arc::new(Mutex::new(Vec::new()));
    let mut file_arc = Arc::new(Mutex::new(HashSet::new()));
    let mut tag_arc = Arc::new(Mutex::new(HashSet::new()));

    let stop = 10;

    //parent.children.iter().for_each(|child| {
    for child in parent.children.iter() {
        if let Some(node) = parent_node.get_node_by_handle(child) {
            match node.kind() {
                mega::NodeKind::File => {
                    if let Some(namespace_id) = client::namespace_get("Mega_Handle".into()) {
                        let mut fpath = parent
                            .parent
                            .iter()
                            .map(|i| format!("/{}", i))
                            .collect::<String>();
                        fpath += &format!("/{}", node.name());

                        let mut search = true;
                        if let Some(tid) = client::tag_get_name(node.handle().into(), namespace_id) {
                           if !client::relationship_get_fileid(tid).is_empty() {
search = false;
                           }
                        }

                        if client::tag_get_name(node.handle().into(), namespace_id).is_none()||search {
                            client::log(format!("MEGA - Downloading: {}", &fpath));

                            download_file(
                                client,
                                file_arc.clone(),
                                tag_arc.clone(),
                                url_input,
                                node,
                                &fpath,
                            );
                            client::log(format!("MEGA - Downloaded: {}", &fpath));
                            out.lock().unwrap().push(MegaDirOrFile::File(MegaFile {
                                file_path: fpath,
                                file_handle: node.handle().to_string(),
                            }));
                            if *cnt >= stop {
                                flag.push(sharedtypes::Flags::Redo);
                                client::log(format!("Stopping due to cnt: {}", cnt));
                                break;
                            }
                            *cnt += 1;
                        } else {
                            let subtag = sharedtypes::SubTag {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "Mega_Source".into(),
                                    description: Some("A mega link.".into()),
                                },
                                tag: url_input.into(),
                                limit_to: None,
                                tag_type: sharedtypes::TagType::Normal,
                            };

                            let tags = vec![
            sharedtypes::TagObject {
                tag: node.handle().into(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(subtag.clone()),
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Mega_Handle".into(),
                    description: Some(
                        "A individual handle that links a file to a dir in mega".into(),
                    ),
                },
            },
            sharedtypes::TagObject {
                tag: fpath,
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(subtag),
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Mega_Name".into(),
                    description: Some("A filepath inside a Mega archive".into()),
                },
            },
        ];
                            for tag in tags {
                                tag_arc.lock().unwrap().insert(tag);
                            }
                        }
                    }
                }
                mega::NodeKind::Folder => {
                    let mut parent = parent.parent.clone();
                    parent.push(node.name().into());
                    out.lock().unwrap().push(MegaDirOrFile::Dir(MegaDir {
                        name: node.name().into(),
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
    _tag: &String,
    _tag_ns: &String,
    regex_match: &String,
    _plugin_callback: &Option<sharedtypes::SearchType>,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut job = sharedtypes::return_default_jobsobj();
    job.site = "mega".into();
    job.param = vec![sharedtypes::ScraperParam::Normal(regex_match.into())];
    job.jobmanager = sharedtypes::DbJobsManager {
        jobtype: sharedtypes::DbJobType::Params,
        recreation: None,
    };

    let mut out = vec![sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: None,
            setting: None,
            relationship: None,
            parents: None,
            jobs: Some(vec![job]),
            namespace: None,
            file: None,
        },
    ])];

    out
}

///
/// Handles downloading and parsing of mega "files"
///
#[unsafe(no_mangle)]
pub fn text_scraping(
    url_input: &String,
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let stop_count = 10;

    let mut cnt = 0;

    let mut file = HashSet::new();
    let mut tag = HashSet::new();

    let mut fileordir = Vec::new();
    let mut flag = Vec::new();

    let http_client = reqwest::Client::builder()
        //.proxy(Proxy::http("socks5://143.110.217.153:1080").unwrap())
        .build()
        .unwrap();

    let client = mega::Client::builder().build(http_client).unwrap();

    let nodes = client.fetch_public_nodes(url_input);

// Weird workaround for if its your first time running this garbage
                if client::namespace_get("Mega_Handle".into()).is_none() {
                    client::namespace_put(
                        "Mega_Handle".into(),
                        Some("A individual handle that links a file to a dir in mega".into()),
                    );
                }


    let nref = task::block_on(nodes);
    if let Ok(nodes) = nref {
        for root in nodes.roots() {
            let rootnew = MegaDir {
                name: root.name().into(),
                parent: vec![root.name().into()],
                children: root.children().to_vec(),
            };

            let mut rootloop = vec![rootnew];

            loop {
                if rootloop.is_empty() {
                                client::log(format!("Stopping due to no more directories to search"));
                    break;
                }
                let rootnew = rootloop.pop().unwrap();

                for item in find_sub_dirs_and_files(
                    &nodes, &rootnew, &mut file, &mut tag, url_input, &client, &mut flag, &mut cnt,
                ) {
                    if let MegaDirOrFile::Dir(ref dir) = item {
                        rootloop.push(dir.clone());
                    }

                    fileordir.push(item);
                }
                if !flag.is_empty() {
                                client::log(format!("Stopping due to flags are empty"));
                    break;
                }
            }
            if root.kind().is_file() {
                let megafile = MegaFile {
                    file_path: format!("/{}", root.name()),
                    file_handle: root.handle().into(),
                };

                let subtag = sharedtypes::SubTag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Source".into(),
                        description: Some("A mega link.".into()),
                    },
                    tag: url_input.into(),
                    limit_to: None,
                    tag_type: sharedtypes::TagType::Normal,
                };

                let tagz = vec![
                    sharedtypes::TagObject {
                        tag: root.handle().into(),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: Some(subtag.clone()),
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Mega_Handle".into(),
                            description: Some(
                                "A individual handle that links a file to a dir in mega".into(),
                            ),
                        },
                    },
                    sharedtypes::TagObject {
                        tag: root.name().into(),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: Some(subtag),
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Mega_Name".into(),
                            description: Some("A filepath inside a Mega archive".into()),
                        },
                    },
                ];

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
                        let mut temp = Vec::new();
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
                        }

                        //fileordir.push(MegaDirOrFile::File(megafile));
                    } else {
                        for tags in tagz {
                            tag.insert(tags);
                        }
                    }
                }
            }
        }
    } else {
        dbg!(nref.err().unwrap());
    }
    Ok(sharedtypes::ScraperObject { file, tag, flag })
}

fn download_file(
    client: &mega::Client,
    file_list: Arc<Mutex<HashSet<sharedtypes::FileObject>>>,
    tag_list: Arc<Mutex<HashSet<sharedtypes::TagObject>>>,
    url_input: &String,
    node: &Node,
    localpath: &String,
) {
    // Ghetto way to pre-filter if we've already downloaded a file
    if let Some(namespace_id) = client::namespace_get("Mega_Handle".into()) {

        let limit_tag = sharedtypes::Tag {
            tag: url_input.into(),
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Mega_Source".into(),
                description: Some("A mega link.".into()),
            }
        };

        let subtag = sharedtypes::SubTag {
            namespace:sharedtypes::GenericNamespaceObj {
                    name: "Mega_Handle".into(),
                    description: Some(
                        "A individual handle that links a file to a dir in mega".into(),
                    ),
                } ,
            tag:  node.handle().into(),
            limit_to: Some(limit_tag),
            tag_type: sharedtypes::TagType::Normal,
        };

        let mut tags = vec![
            sharedtypes::TagObject {
                tag: node.handle().into(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: None,
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Mega_Handle".into(),
                    description: Some(
                        "A individual handle that links a file to a dir in mega".into(),
                    ),
                },
            },
                    ];



        if client::tag_get_name(node.handle().into(), namespace_id).is_none() {
            let mut temp = Vec::new();
            task::block_on(client.download_node(&node, &mut temp)).unwrap();
            file_list.lock().unwrap().insert(sharedtypes::FileObject {
                source: Some(sharedtypes::FileSource::Bytes(temp)),
                hash: sharedtypes::HashesSupported::None,
                tag_list: tags,
                skip_if: vec![sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                    tag: node.handle().into(),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Mega_Handle".into(),
                        description: Some(
                            "A individual handle that links a file to a dir in mega".into(),
                        ),
                    },
                })],
            });
            //fileordir.push(MegaDirOrFile::File(megafile));
        } else {

tags.push(sharedtypes::TagObject {
                tag: localpath.into(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(subtag),
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Mega_Name".into(),
                    description: Some("A filepath inside a Mega archive".into()),
                },
            }
);

            for tag in tags {
                tag_list.lock().unwrap().insert(tag);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct MegaDir {
    name: String,
    parent: Vec<String>,
    children: Vec<String>,
}

#[derive(Debug, Clone)]
struct MegaFile {
    file_path: String,
    file_handle: String,
}

#[derive(Debug, Clone)]
enum MegaDirOrFile {
    Dir(MegaDir),
    File(MegaFile),
}

#[unsafe(no_mangle)]
pub fn download_from(file: &sharedtypes::FileObject) -> Option<Vec<u8>> {
    None
}
