use chrono::{DateTime, FixedOffset};
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;
use json::JsonValue;

use crate::sharedtypes::DEFAULT_PRIORITY;

pub const SITE: &str = "Danbooru";
pub const SITE_URL: &str = "danbooru.donmai.us";
pub const DANBOORU_POST_LIMIT: usize = 20;

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut danbooru = sharedtypes::return_default_globalpluginparser();

    danbooru.name = "danbooru".to_string();
    danbooru.version = 0;
    danbooru.login_type = vec![(
        "danbooru_api".to_string(),
        sharedtypes::LoginType::ApiNamespaced("User_Api_Login".to_string(), None, None),
        sharedtypes::LoginNeed::Optional,
        Some("Username and API key goes in here.".to_string()),
        false,
    )];
    danbooru.stored_info = Some(sharedtypes::StoredInfo::Storage(vec![(
        "loaded_site".to_string(),
        "danbooru".to_string(),
    )]));
    danbooru.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (8, Duration::from_secs(1)),
            sites: vec![SITE.to_string(), SITE.to_lowercase()],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![],
        },
    ));
    danbooru.callbacks = vec![sharedtypes::GlobalCallbacks::Start(
        sharedtypes::StartupThreadType::Inline,
    )];

    vec![danbooru]
}

///
/// Gets a list of URLs to scrape
///
#[unsafe(no_mangle)]
pub fn url_dump(
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut ret = Vec::new();
    let hardlimit = 1000;
    for i in 1..hardlimit {
        if let Some(out) = build_url(params, i) {
            ret.push((out, scraperdata.clone()));
        }
    }
    ret
}

///
/// Generates a list of urls to be scraped
///
fn build_url(params: &[sharedtypes::ScraperParam], pagenum: u32) -> Option<String> {
    let mut url = format!("https://{SITE_URL}/posts.json?");

    let mut login_info = None;

    for param in params {
        if let sharedtypes::ScraperParam::Login(login_type) = param {
            login_info = Some(login_type);
            break;
        }
    }

    if let Some(login_info) = login_info {
        url = url + &format!("login={}&api_key={}&", "not going", "to impliment this")
    }

    url = url + &format!("page={}&tags=", pagenum);
    let mut should_exit_early = true;
    for param in params {
        if let sharedtypes::ScraperParam::Normal(tag) = param {
            should_exit_early = false;
            url = url + &format!("{}+", tag)
        }
    }

    // Needed incase we have no tags for some goofy reason
    if should_exit_early {
        return None;
    }

    // Removes the last + sign for neatness
    url.truncate(url.len() - 1);

    Some(url)
}

#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    _source_url: &str,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut tag = HashSet::new();
    let mut file = HashSet::new();

    let mut post_ids = Vec::new();

    if let Ok(js) = json::parse(html_input) {
        if js.is_empty() {
            return vec![sharedtypes::ScraperReturn::Nothing];
        }
        // Early exit checking if theirs an error or if the post list is empty
        if !js.is_array() || js.is_null() {
            return vec![sharedtypes::ScraperReturn::Stop(
                "Could not parse an array of posts from the html returned".to_string(),
            )];
        }

        for item in js.members() {
            parse_post(item, scraperdata, &mut tag, &mut file, &mut post_ids);
            parse_pool(item, scraperdata, &mut tag);
        }
    } else {
        return vec![sharedtypes::ScraperReturn::Stop(
            "Failed to parse the html returned".to_string(),
        )];
    }

    if !post_ids.is_empty() {
        let mut post_ids_string = String::new();

        for post_id in post_ids.iter() {
            post_ids_string += &format!("id:{} ", post_id);
        }
        post_ids_string.truncate(post_ids_string.len() - 1);

        tag.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "DO NOT PARSE".to_string(),
                description: None,
            },
            tag: "".to_string(),
            relates_to: None,

            tag_type: sharedtypes::TagType::ParseUrl((
                sharedtypes::ScraperData {
                    job: sharedtypes::JobScraper {
                        site: SITE.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(format!(
                            "https://{SITE_URL}/pools.json?search[post_tags_match]={}",
                            post_ids_string
                        ))],
                        job_type: sharedtypes::DbJobType::Scraper,
                    },
                    system_data: BTreeMap::new(),
                    user_data: BTreeMap::new(),
                },
                None,
            )),
        });
    }

    let mut out = vec![sharedtypes::ScraperReturn::Data(
        sharedtypes::ScraperObject {
            file,
            tag,
            flag: vec![],
        },
    )];
    if post_ids.len() <= DANBOORU_POST_LIMIT {
        out.push(sharedtypes::ScraperReturn::Nothing);
    }

    out
}

/// Determines if we should do external lookups for children or pools
fn should_add_external_lookups(scraperdata: &sharedtypes::ScraperData) -> bool {
    if let Some(recursion) = scraperdata.system_data.get("recursion") {
        recursion != "false"
    } else {
        true
    }
}

fn parse_pool(
    pool: &JsonValue,
    scraperdata: &sharedtypes::ScraperData,
    tag: &mut HashSet<sharedtypes::TagObject>,
) {
    if pool["id"].is_empty() {
        return;
    }

    let pool_id = pool["id"].to_string();
    tag.insert(sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "Danbooru-Pool-Id".to_string(),
            description: Some("The pool id of a pool".to_string()),
        },
        tag: pool_id.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: None,
    });

    if !pool["name"].is_empty() {
        tag.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Pool-Name".to_string(),
                description: Some("The pool's name".to_string()),
            },
            tag: pool["name"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: Some(sharedtypes::SubTag {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Danbooru-Pool-Id".to_string(),
                    description: Some("The pool id of a pool".to_string()),
                },
                limit_to: None,
                tag: pool_id.clone(),
                tag_type: sharedtypes::TagType::Normal,
            }),
        });
    }
    if !pool["created_at"].is_empty() {
        tag.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Pool-Created-At".to_string(),
                description: Some("When the pool was made".to_string()),
            },
            tag: pool["created_at"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: Some(sharedtypes::SubTag {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Danbooru-Pool-Id".to_string(),
                    description: Some("The pool id of a pool".to_string()),
                },
                limit_to: None,
                tag: pool_id.clone(),
                tag_type: sharedtypes::TagType::Normal,
            }),
        });
    }
    // Processes the pool posts
    for (cnt, post_id) in pool["post_ids"].members().enumerate() {
        tag.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Pool-Position".to_string(),
                description: Some("The position of an item in a pool".to_string()),
            },
            tag: cnt.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: Some(sharedtypes::SubTag {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "Danbooru-Id".to_string(),
                    description: Some("A post id inside of danbooru. Is unique".to_string()),
                },
                tag: post_id.to_string(),
                limit_to: Some(sharedtypes::Tag {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Danbooru-Pool-Id".to_string(),
                        description: Some("The pool id of a pool".to_string()),
                    },
                    tag: pool_id.clone(),
                }),
                tag_type: sharedtypes::TagType::Normal,
            }),
        });

        tag.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "DO NOT PARSE".to_string(),
                description: None,
            },
            tag: "".to_string(),
            relates_to: None,

            tag_type: sharedtypes::TagType::ParseUrl((
                sharedtypes::ScraperData {
                    job: sharedtypes::JobScraper {
                        site: SITE.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(format!(
                            "https://{SITE_URL}/posts.json?tags=id:{}",
                            post_id
                        ))],
                        job_type: sharedtypes::DbJobType::Scraper,
                    },
                    system_data: BTreeMap::new(),
                    user_data: BTreeMap::new(),
                },
                // Skips the file if we've already got it
                Some(sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                    tag: post_id.to_string(),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Danbooru-Id".to_string(),
                        description: Some("A post id inside of danbooru. Is unique".to_string()),
                    },
                })),
            )),
        });
    }
}

/// Parses a posts information
fn parse_post(
    post: &JsonValue,
    scraperdata: &sharedtypes::ScraperData,
    tag: &mut HashSet<sharedtypes::TagObject>,
    file: &mut HashSet<sharedtypes::FileObject>,
    post_ids: &mut Vec<String>,
) {
    let source_url = &post["file_url"];
    let source_md5 = &post["md5"];

    // Early exit incase we cant pull a file url
    if source_url.is_null() || source_md5.is_null() {
        return;
    }

    let mut tag_list = Vec::new();

    let search_table = [
        (
            "tag_string_general",
            sharedtypes::GenericNamespaceObj {
                name: "Danbooru-General".to_string(),
                description: Some("The General Tag from Danbooru".to_string()),
            },
        ),
        (
            "tag_string_character",
            sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Character".to_string(),
                description: Some("The Character Tag from Danbooru".to_string()),
            },
        ),
        (
            "tag_string_copyright",
            sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Copyright".to_string(),
                description: Some("The Copyright Tag from Danbooru".to_string()),
            },
        ),
        (
            "tag_string_artist",
            sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Artist".to_string(),
                description: Some("The Artist Tag from Danbooru".to_string()),
            },
        ),
        (
            "tag_string_meta",
            sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Meta".to_string(),
                description: Some("The Meta Tag from Danbooru".to_string()),
            },
        ),
    ];

    for (json_search, namespace) in search_table {
        if !post[json_search].is_null() {
            for tag in post[json_search].to_string().split(' ') {
                if tag.is_empty() {
                    continue;
                }

                tag_list.push(sharedtypes::TagObject {
                    namespace: namespace.clone(),
                    tag: tag.to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: None,
                });
            }
        }
    }

    if !post["rating"].is_empty() {
        tag_list.push(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Rating".to_string(),
                description: Some("What the item was rated at when scraped".to_string()),
            },
            tag: post["rating"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: None,
        });
    }

    if !post["id"].is_null() {
        let danbooru_id_namespace = sharedtypes::GenericNamespaceObj {
            name: "Danbooru-Id".to_string(),
            description: Some("A post id inside of danbooru. Is unique".to_string()),
        };

        // Stupid way of doing it but i already coded it. Could of pulled from json
        let relates_to =
            scraperdata
                .user_data
                .get("parent-id")
                .map(|parent_id| sharedtypes::SubTag {
                    namespace: danbooru_id_namespace.clone(),
                    tag: parent_id.to_string(),
                    limit_to: None,
                    tag_type: sharedtypes::TagType::Normal,
                });

        tag_list.push(sharedtypes::TagObject {
            namespace: danbooru_id_namespace,
            tag: post["id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to,
        });

        post_ids.push(post["id"].to_string());
    } else {
        return;
    }

    if !post["created_at"].is_empty() {
        let dt: DateTime<FixedOffset> =
            DateTime::parse_from_rfc3339(&post["created_at"].to_string()).unwrap();

        let timestamp = dt.timestamp_millis();

        tag_list.push(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "Danbooru-Created-At".to_string(),
                description: Some(
                    "When the post or pool was created at in unix timestamp format".to_string(),
                ),
            },
            tag: timestamp.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: None,
        });
    }

    if !post["has_active_children"].is_empty() && should_add_external_lookups(scraperdata) {
        let mut user_data = BTreeMap::new();

        user_data.insert("parent-id".to_string(), post["id"].to_string());

        tag.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "DO NOT PARSE".to_string(),
                description: None,
            },
            tag: "".to_string(),
            relates_to: None,

            tag_type: sharedtypes::TagType::ParseUrl((
                sharedtypes::ScraperData {
                    job: sharedtypes::JobScraper {
                        site: SITE.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(format!(
                            "https://{SITE_URL}/posts.json?tags=parent:{}",
                            post["id"]
                        ))],
                        job_type: sharedtypes::DbJobType::Scraper,
                    },
                    system_data: BTreeMap::new(),
                    user_data,
                },
                None,
            )),
        });
    }

    file.insert(sharedtypes::FileObject {
        source: Some(sharedtypes::FileSource::Url(source_url.to_string())),
        hash: sharedtypes::HashesSupported::Md5(source_md5.to_string()),
        tag_list,
        skip_if: vec![],
    });
}
