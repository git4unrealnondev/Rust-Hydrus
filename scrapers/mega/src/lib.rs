use mega::{Node, Nodes};
use regex::Regex;
use std::{collections::HashSet, time::Duration};

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
            redirect: Some("mega scraper".into()),
        },
    ));
    plugin.callbacks = callbackvec;

    let mut scraper = sharedtypes::return_default_globalpluginparser();
    scraper.name = "Mega Scraper".into();
    scraper.should_handle_text_scraping = true;
    scraper.should_send_files_on_scrape = true;
    scraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec!["mega".into(), "mega.nz".into()],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
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
fn find_sub_dirs_and_files(parent_node: &Nodes, parent: &MegaDir) -> Vec<MegaDirOrFile> {
    let mut out = Vec::new();

    for child in parent.children.iter() {
        if let Some(childnode) = parent_node.get_node_by_handle(child) {
            match childnode.kind() {
                mega::NodeKind::File => {
                    let mut fpath = parent
                        .parent
                        .iter()
                        .map(|i| format!("/{}", i))
                        .collect::<String>();
                    fpath += &format!("/{}", childnode.name());
                    out.push(MegaDirOrFile::File(MegaFile {
                        file_path: fpath,
                        file_handle: childnode.handle().to_string(),
                    }));
                }
                mega::NodeKind::Folder => {
                    let mut parent = parent.parent.clone();
                    parent.push(childnode.name().into());
                    out.push(MegaDirOrFile::Dir(MegaDir {
                        name: childnode.name().into(),
                        parent,
                        children: childnode.children().to_vec(),
                    }));
                }
                _ => {}
            }
        }
    }

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
    use async_std::task;

    let mut fileordir = Vec::new();

    let http_client = reqwest::Client::new();

    let client = mega::Client::builder().build(http_client).unwrap();

    let nodes = client.fetch_public_nodes(url_input);

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
                    break;
                }
                let rootnew = rootloop.pop().unwrap();

                for item in find_sub_dirs_and_files(&nodes, &rootnew) {
                    if let MegaDirOrFile::Dir(ref dir) = item {
                        rootloop.push(dir.clone());
                    }

                    fileordir.push(item);
                }
            }
            if root.kind().is_file() {
                let megafile = MegaFile {
                    file_path: format!("/{}", root.name()),
                    file_handle: root.handle().into(),
                };
                fileordir.push(MegaDirOrFile::File(megafile));
            }
        }
    }

    return Ok(sharedtypes::ScraperObject {
        file: HashSet::new(),
        tag: HashSet::new(),
    });
}

#[derive(Debug, Clone)]
struct MegaDir {
    name: String,
    parent: Vec<String>,
    children: Vec<String>,
}

#[derive(Debug)]
struct MegaFile {
    file_path: String,
    file_handle: String,
}

#[derive(Debug)]
enum MegaDirOrFile {
    Dir(MegaDir),
    File(MegaFile),
}

#[unsafe(no_mangle)]
pub fn download_from(file: &sharedtypes::FileObject) -> Option<Vec<u8>> {
    None
}
