use regex::Regex;
use scraper::Html;
use scraper::Selector;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;
pub const REGEX_COLLECTIONS: &str =
    "(http(s)?://www.|((www.|http(s)?://)))catbox.moe/c/[a-z0-9]{6}";

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let tag_vec = (
        Some(sharedtypes::SearchType::Regex(REGEX_COLLECTIONS.into())),
        vec![],
        vec!["source_url".to_string(), "Catbox Collection".into()],
    );

    let callbackvec = vec![
        sharedtypes::GlobalCallbacks::Tag(tag_vec),
        sharedtypes::GlobalCallbacks::Start,
    ];

    let mut plugin = sharedtypes::return_default_globalpluginparser();
    plugin.name = "Catbox Regex Parser".to_string();
    plugin.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_type: sharedtypes::PluginThreadType::SpawnInline,
            com_channel: true,
        },
    ));
    plugin.callbacks = callbackvec;

    let mut scraper = sharedtypes::return_default_globalpluginparser();
    scraper.name = "Catbox Scraper".into();
    scraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec![
                "catbox".into(),
                "catbox album".into(),
                "catbox.moe".into(),
                "Catbox Collection".into(),
            ],
            priority: DEFAULT_PRIORITY,
        },
    ));
    vec![plugin, scraper]
}

#[no_mangle]
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

    dbg!(&out);

    out
}

#[no_mangle]
pub fn parser(
    html_input: &String,
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut file_list = HashSet::new();
    let mut tag_list = HashSet::new();
    let mut url = Vec::new();
    let mut html_usertext = Vec::new();

    for param in params {
        if let sharedtypes::ScraperParam::Url(link) = param {
            url.push(link);
        }
    }

    if url.is_empty() {
        return Err(sharedtypes::ScraperReturn::Stop(
            "Could not find url in params field".into(),
        ));
    }

    let document = Html::parse_document(html_input);

    let selector = Selector::parse(r#"div[class="title"]"#).unwrap();

    {
        let mut should_add = false;
        for list_elements in document.select(&selector) {
            for element in list_elements.descendants() {
                if let Some(text) = element.value().as_text() {
                    if should_add {
                        let text = text.to_string();
                        let lines = text
                            .split('\n')
                            .map(|line| line.strip_suffix('\r').unwrap_or(line));
                        for line in lines {
                            html_usertext.push(line.to_string());
                        }
                    }
                    should_add = !should_add;
                }
            }
        }
    }

    let mut html_collection_text = String::new();
    let first = html_usertext.first();
    if let Some(first) = first {
        html_collection_text.push_str(first);
        html_usertext.remove(0);
        let last = html_usertext.pop();
        if let Some(last) = last {
            for text in html_usertext {
                html_collection_text = format!("{}\n{}", html_collection_text, text)
            }
            html_collection_text.push_str(&last);
        }
    }

    if !html_collection_text.is_empty() {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        tag_list.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Catbox Collection Text".to_string(),
                description: Some("The text that is inside of a catbox collection.".to_string()),
            },
            tag: html_collection_text,
            tag_type: sharedtypes::TagType::Normal,
            relates_to: Some(sharedtypes::SubTag {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Catbox Collection".into(),
                    description: Some("A CatBox collection album. Stores Pictures.".into()),
                },
                tag: url.first().unwrap().to_string(),
                tag_type: sharedtypes::TagType::Normal,
                limit_to: Some(sharedtypes::Tag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Catbox Collection Scraping Time".into(),
                        description: Some("When the last time was an album touched".into()),
                    },
                    tag: format!("{}", since_the_epoch),
                }),
            }),
        });
    }

    let selector = Selector::parse(r#"div[class="imagelist"]"#).unwrap();
    let selector_link = Selector::parse(r#"a"#).unwrap();
    for url in url {
        let mut cnt = 0;
        for list_elements in document.select(&selector) {
            for img in list_elements.select(&selector_link) {
                if let Some(link) = img.attr("href") {
                    file_list.insert(sharedtypes::FileObject {
                        source_url: Some(link.into()),
                        hash: sharedtypes::HashesSupported::None,
                        tag_list: vec![sharedtypes::TagObject {
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: "source_url".into(),
                                description: None,
                            },
                            tag: link.into(),
                            tag_type: sharedtypes::TagType::Normal,
                            relates_to: Some(sharedtypes::SubTag {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "Catbox Collection".into(),
                                    description: Some(
                                        "A CatBox collection album. Stores Pictures.".into(),
                                    ),
                                },
                                tag: url.into(),
                                limit_to: None,
                                tag_type: sharedtypes::TagType::Normal,
                            }),
                        }],
                        skip_if: vec![],
                    });

                    tag_list.insert(sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Catbox Collection Position".into(),
                            description: Some(
                                "The position of the image inside of a catbox collection.".into(),
                            ),
                        },
                        tag: format!("{}", cnt),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: Some(sharedtypes::SubTag {
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: "source_url".into(),
                                description: None,
                            },
                            tag: link.into(),
                            limit_to: Some(sharedtypes::Tag {
                                tag: url.into(),
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "Catbox Collection".into(),
                                    description: Some(
                                        "A CatBox collection album. Stores Pictures.".into(),
                                    ),
                                },
                            }),
                            tag_type: sharedtypes::TagType::Normal,
                        }),
                    });
                    cnt += 1;
                }
            }
        }
    }
    Ok(sharedtypes::ScraperObject {
        file: file_list,
        tag: tag_list,
    })
}

#[no_mangle]
pub fn on_start(parserscraper: &sharedtypes::GlobalPluginScraper) {
    client::load_table(sharedtypes::LoadDBTable::Namespace);
    let mut should_reload_regex = false;

    if client::namespace_get("Catbox Collection".into()).is_none() {
        client::namespace_put(
            "Catbox Collection".into(),
            Some("A CatBox collection album. Stores Pictures.".into()),
            true,
        );
        should_reload_regex = true;
    }
    if client::namespace_get("Catbox Collection Position".into()).is_none() {
        client::namespace_put(
            "Catbox Collection Position".into(),
            Some("The position of the image inside of a catbox collection.".into()),
            true,
        );
        should_reload_regex = true;
    }
    if client::namespace_get("Catbox Collection".into()).is_none() {
        client::namespace_put(
            "Catbox Collection".into(),
            Some("A CatBox collection album. Stores Pictures".into()),
            true,
        );
        should_reload_regex = true;
    }

    if should_reload_regex {
        client::reload_regex();
    }

    let should_search_regex;
    match client::settings_get_name("Catbox Collection Regex Has Searched".into()) {
        None => {
            client::setting_add("Catbox Collection Regex Has Searched".into(), Some("Should the catbox regex be run on all tags in db to find any tags that are applicable?".into()), None, Some("True".into()), true);
            should_search_regex = true;
        }
        Some(setting) => {
            if let Some(param) = setting.param {
                should_search_regex = param != "False";
            } else {
                should_search_regex = true;
            }
        }
    }

    if should_search_regex {
        client::log(
            "Starting to run a regex search on tags in the DB for Catbox Regex".to_string(),
        );
        client::load_table(sharedtypes::LoadDBTable::Tags);
        client::load_table(sharedtypes::LoadDBTable::Namespace);
        client::load_table(sharedtypes::LoadDBTable::Jobs);
        let source_url_nsid = match client::namespace_get("source_url".to_string()) {
            None => {
                client::log(
                    "Early Exit for Catbox Regex Search. No source_url namespace id can be found."
                        .to_string(),
                );
                return;
            }
            Some(nsid) => nsid,
        };
        let catbox_collection_nsid = match client::namespace_get("Catbox Collection".to_string()) {
            None => {
                client::log(
                    "Early Exit for Catbox Regex Search. No Catbox Collection namespace id can be found."
                        .to_string(),
                );
                return;
            }
            Some(nsid) => nsid,
        };
        let mut list: Vec<usize> = client::namespace_get_tagids_all();

        let mut removal_namespace_ids = vec![source_url_nsid];

        let removal_namespace_names = ["FileHash", "BlurHash"];

        for nsid in list.iter() {
            if let Some(nsobj) = client::namespace_get_string(*nsid) {
                for item in removal_namespace_names {
                    if nsobj.name.to_lowercase().contains(&item.to_lowercase()) {
                        removal_namespace_ids.push(*nsid);
                    }
                }
            }
        }
        client::log(format!("We've got: {} items to filter", list.len()));
        {
            for removal in removal_namespace_ids.iter() {
                let temp_list = list.clone();
                for (cnt, item) in temp_list.iter().enumerate() {
                    if item == removal {
                        list.remove(cnt);
                    }
                }
            }
        }
        client::log(format!("We've got: {} items post filter", list.len()));

        let mut need_to_search = BTreeSet::new();
        let mut need_to_remove = BTreeSet::new();

        let regex = Regex::new(REGEX_COLLECTIONS).unwrap();
        let mut cnt = 0;
        for item in list {
            for tagid in client::namespace_get_tagids(item) {
                if let Some(tag_nns) = client::tag_get_id(tagid) {
                    cnt += 1;
                    if cnt >= 1000 {
                        cnt = 0;
                    }
                    for item_match in regex.find_iter(&tag_nns.name).map(|c| c.as_str()) {
                        if tag_nns.namespace == catbox_collection_nsid {
                            need_to_remove.insert(item_match.to_string());
                        } else {
                            need_to_search.insert(item_match.to_string());
                        }
                    }
                }
            }
        }

        // Filters anything that we haven't touched yet.
        // If a collection has updated but we've already scraped it then not my problem.
        need_to_search.retain(|e| !need_to_remove.contains(e));

        for item in need_to_search {
            client::log(format!("Adding job to scrape catbox collection: {}", item));
            client::job_add(
                None,
                0,
                0,
                "Catbox Collection".into(),
                vec![sharedtypes::ScraperParam::Url(item)],
                BTreeMap::new(),
                BTreeMap::new(),
                sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::Scraper,
                    recreation: None,
                },
            );
        }
        client::log(
            "Finished running scrape catbox collection job. Telling DB to not run this again."
                .to_string(),
        );

        client::setting_add("Catbox Collection Regex Has Searched".into(), Some("Should the catbox regex be run on all tags in db to find any tags that are applicable?".into()), None, Some("False".into()), true);
    }
}

#[no_mangle]
pub fn on_regex_match(
    tag: &String,
    tag_ns: &String,
    regex_match: &String,
    callback: &Option<sharedtypes::SearchType>,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut out = Vec::new();
    if regex_match.contains("bsky.app") {
        return out;
    }

    out.push(sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: None,
            setting: None,
            relationship: None,
            parents: None,
            jobs: Some(vec![sharedtypes::DbJobsObj {
                id: 0,
                time: 0,
                reptime: Some(0),

                site: "catbox album".to_string(),
                param: vec![sharedtypes::ScraperParam::Url(regex_match.to_string())],
                jobmanager: sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::Scraper,
                    recreation: None,
                },
                isrunning: false,
                system_data: BTreeMap::new(),
                user_data: BTreeMap::new(),
            }]),
            namespace: None,
            file: None,
        },
    ]));

    out
}
