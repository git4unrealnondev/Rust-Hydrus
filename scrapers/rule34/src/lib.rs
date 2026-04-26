use std::{collections::HashSet, time::Duration};

#[path = "../../../src/client.rs"]
mod client;
#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;

pub const SITE_NAME: &str = "rule34";
pub const SITE_FULL: &str = "rule34.xxx";
pub const SITE_URL: &str = "https://rule34.xxx";

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let scraper = sharedtypes::GlobalPluginScraper {
        name: SITE_NAME.into(),
        storage_type: Some(sharedtypes::ScraperOrPlugin::Scraper(
            sharedtypes::ScraperInfo {
                ratelimit: (3, Duration::from_secs(1)),
                sites: vec![SITE_NAME.into(), SITE_FULL.into()],
                priority: DEFAULT_PRIORITY,
                num_threads: Some(1),
                modifiers: vec![],
            },
        )),

        login_type: vec![(
            SITE_NAME.into(),
            sharedtypes::LoginType::Login("Rule34xxx_Login".to_string(), None),
            sharedtypes::LoginNeed::Required,
            Some("Put your API key for the username and user_id for the password.".to_string()),
            false,
        )],

        ..Default::default()
    };

    vec![scraper]
}

///
/// Dumps a list of urls to scrape
///
#[unsafe(no_mangle)]
pub fn url_dump(
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperDataReturn> {
    let mut out = Vec::new();

    let mut url_params = Vec::new();
    let mut search_params = Vec::new();

    for param in params.iter() {
        match param {
            sharedtypes::ScraperParam::Normal(search_term) => {
                search_params.push(search_term.clone());
            }
            sharedtypes::ScraperParam::Url(_url) => {
                //TODO need to support parsing urls here
            }
            sharedtypes::ScraperParam::Login(login) => {
                if let sharedtypes::LoginType::Login(_, Some(logininfo)) = login {
                    url_params.push(("api_key", logininfo.username.expose_secret().clone()));
                    url_params.push(("user_id", logininfo.password.expose_secret().clone()));
                }
            }
            _ => {}
        }
    }

    let mut user_data = scraperdata.job.user_data.clone();

    // Always return a JSON object
    url_params.push(("json", "1".to_string()));

    if !search_params.is_empty() {
        // Sets this as an API
        url_params.push(("page", "dapi".to_string()));

        // Searching Posts
        url_params.push(("s", "post".to_string()));

        // Querting by index??
        url_params.push(("q", "index".to_string()));

        // Sets limit as 100
        let limit = 100;
        url_params.push(("limit", limit.to_string()));
        user_data.insert("limit".into(), limit.to_string());

        // Needed to pull in additional tag data that we need to get
        url_params.push(("fields", "tag_info".to_string()));

        let joined = search_params
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(" ");
        url_params.push(("tags", joined));

        // Sets the page ID if it exists
        if let Some(pid) = user_data.get("pid") {
            let mut pid: u64 = pid.parse().unwrap();
            url_params.push(("pid", pid.to_string()));
            pid += 1;
            user_data.insert("pid".into(), pid.to_string());
        } else {
            url_params.push(("pid", "0".to_string()));
            user_data.insert("pid".into(), "1".to_string());
        }

        let mut url = url::Url::parse_with_params(SITE_URL, url_params).unwrap();

        url.set_path("index.php");

        user_data.insert("base_url".into(), url.as_str().to_string());
        out.push(sharedtypes::ScraperDataReturn {
            job: sharedtypes::DbJobsObj {
                site: SITE_NAME.into(),

                param: vec![sharedtypes::ScraperParam::Url(url.as_str().to_string())],
                jobmanager: sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::Scraper,
                    ..Default::default()
                },
                user_data,

                ..Default::default()
            },
            ..Default::default()
        });
    }

    out
}

///
/// Cleans string of html chars
///
fn optstr_to_cleaned(inp: Option<&str>) -> Option<String> {
    if let Some(inp) = inp {
        return Some(html_escape::decode_html_entities(inp).to_string());
    }

    None
}

#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    _: &str,
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut user_data = scraperdata.job.user_data.clone();
    let mut out = Vec::new();
    let mut files = HashSet::new();

    if let Ok(json) = json::parse(html_input) {
        if let Some(limit) = scraperdata.job.user_data.get("limit")
            && let Some(pid) = scraperdata.job.user_data.get("pid")
        {
            let limit: usize = limit.parse().unwrap();
            let pid: usize = pid.parse().unwrap();
            if json.members().len() == limit {
                let mut jobs = HashSet::new();

                if let Some(base_url) = scraperdata.job.user_data.get("base_url") {
                    let mut url = url::Url::parse(base_url).unwrap();

                    let mut pid_updated = false;

                    {
                        // collect existing params, replacing pid when found
                        let params: Vec<(String, String)> = url
                            .query_pairs()
                            .map(|(k, v)| {
                                let k = k.into_owned();
                                let v = if k == "pid" {
                                    pid_updated = true;
                                    let pid_updated = pid + 1;
                                    user_data.insert("pid".into(), pid_updated.to_string());
                                    pid.to_string()
                                } else {
                                    v.into_owned()
                                };
                                (k, v)
                            })
                            .collect();

                        let mut qp = url.query_pairs_mut();
                        qp.clear();
                        for (k, v) in params {
                            qp.append_pair(&k, &v);
                        }
                    }

                    // if pid wasn't present, optionally add it
                    if !pid_updated {
                        url.query_pairs_mut().append_pair("pid", &pid.to_string());
                        let temp_pid = pid + 1;
                        user_data.insert("pid".into(), temp_pid.to_string());
                    }

                    let final_url = url.to_string();

                    user_data.insert("base_url".to_string(), final_url.clone());

                    jobs.insert(sharedtypes::ScraperDataReturn {
                        job: sharedtypes::DbJobsObj {
                            site: SITE_NAME.into(),
                            param: vec![sharedtypes::ScraperParam::Url(final_url)],
                            jobmanager: sharedtypes::DbJobsManager {
                                jobtype: sharedtypes::DbJobType::Scraper,
                                ..Default::default()
                            },
                            user_data,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }
                out.push(sharedtypes::ScraperReturn::Data(
                    sharedtypes::ScraperObject {
                        jobs,
                        ..Default::default()
                    },
                ));
            }
        }

        for post in json.members() {
            let mut tag_list = Vec::new();
            let mut tags = Vec::new();

            let file_url = match post["file_url"].as_str() {
                Some(v) => v.to_string(),
                None => continue,
            };

            // Gets the post id from post
            if let Some(post_id) = post["id"].as_u64() {
                let relates_to = match post["parent_id"].as_u64() {
                    Some(parent_id) => {
                        if parent_id == 0 {
                            None
                        } else {
                            Some(sharedtypes::SubTag {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "Rule34.xxx_Parent_Id".into(),
                                    description: Some("A post's parent from rule34.xxx".into()),
                                },
                                tag: parent_id.to_string(),
                                ..Default::default()
                            })
                        }
                    }
                    None => None,
                };
                tag_list.push(sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Set,
                    tags: vec![sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Rule34.xxx_Post_Id".into(),
                            description: Some("A post's id from rule34.xxx".into()),
                        },
                        tag: post_id.to_string(),
                        relates_to,
                        ..Default::default()
                    }],
                });
            }

            // Gets the post rating from post
            if let Some(post) = post["rating"].as_str() {
                tag_list.push(sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Set,
                    tags: vec![sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Rule34.xxx_rating".into(),
                            description: Some("A post's rating from rule34.xxx".into()),
                        },
                        tag: post.to_string(),
                        ..Default::default()
                    }],
                });
            }

            // Gets the post last change time from post
            if let Some(post) = post["change"].as_u64() {
                tag_list.push(sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Set,
                    tags: vec![sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Rule34.xxx_change".into(),
                            description: Some(
                                "Last time a post was changed from rule34.xxx".into(),
                            ),
                        },
                        tag: post.to_string(),
                        ..Default::default()
                    }],
                });
            }

            // Gets the post's sources from post
            if let Some(post) = optstr_to_cleaned(post["source"].as_str()) {
                let mut tags = Vec::new();
                for source in post.split(" ") {
                    tags.push(sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "Rule34.xxx_source".into(),
                            description: Some("A source for a post from rule34.xxx".into()),
                        },
                        tag: source.to_string(),
                        ..Default::default()
                    });
                }
                tag_list.push(sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Set,
                    tags,
                });
            }

            // Gets all tags from the post
            for tag_info in post["tag_info"].members() {
                if let Some(tag) = optstr_to_cleaned(tag_info["tag"].as_str())
                    && let Some(name) = optstr_to_cleaned(tag_info["type"].as_str())
                {
                    tags.push(sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: format!("Rule34.xxx_{}", name),
                            description: Some(format!("A {} tag type from rule34.xxx", name)),
                        },
                        tag,
                        ..Default::default()
                    });
                }
            }

            if !tags.is_empty() {
                tag_list.push(sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Set,
                    tags,
                });
            }

            // Adds file into search query
            files.insert(sharedtypes::FileObject {
                source: Some(sharedtypes::FileSource::Url(vec![file_url])),
                hash: sharedtypes::HashesSupported::None,
                tag_list,
                ..Default::default()
            });
        }
    }

    if !files.is_empty() {
        out.push(sharedtypes::ScraperReturn::Data(
            sharedtypes::ScraperObject {
                files,
                ..Default::default()
            },
        ));
    }

    out
}
