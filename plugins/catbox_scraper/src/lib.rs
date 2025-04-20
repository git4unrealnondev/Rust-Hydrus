use scraper::Html;
use scraper::Selector;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::time::Duration;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let tag_vec = (
        Some(sharedtypes::SearchType::Regex(
            "(http(s)?://www.|((www.|http(s)?://)))catbox.moe/c/[a-z0-9]{6}".into(),
        )),
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
            com_type: sharedtypes::PluginThreadType::Inline,
            com_channel: true,
        },
    ));
    plugin.callbacks = callbackvec;

    let mut scraper = sharedtypes::return_default_globalpluginparser();
    scraper.name = "Catbox Scraper".into();
    scraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec!["catbox album".into(), "catbox.moe".into()],
        },
    ));
    vec![plugin, scraper]
}

#[no_mangle]
pub fn parser(
    html_input: &String,
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut file_list = HashSet::new();
    let mut tag_list = HashSet::new();
    let mut url = None;
    for param in params {
        if let sharedtypes::ScraperParam::Url(link) = param {
            url = Some(link);
        }
    }

    if url.is_none() {
        return Err(sharedtypes::ScraperReturn::Stop(
            "Could not find url in params field".into(),
        ));
    }

    let document = Html::parse_document(html_input);
    let selector = Selector::parse(r#"div[class="imagelist"]"#).unwrap();
    let selector_link = Selector::parse(r#"a"#).unwrap();
    if let Some(url) = url {
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
    dbg!(tag, tag_ns);

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
                committype: Some(sharedtypes::CommitType::StopOnNothing),
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
