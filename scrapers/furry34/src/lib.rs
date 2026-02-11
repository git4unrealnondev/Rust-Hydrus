use chrono::DateTime;
use scraper::{Html, Selector};
use std::collections::BTreeMap;
use std::{collections::HashSet, time::Duration};
use url::Url;

const SITE_BASE: &str = "https://furry34.com/";
const SITE_NAME: &str = "furry34";
const POST_PRIORITY: &usize = &(sharedtypes::DEFAULT_PRIORITY - 2);

#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut furry34 = sharedtypes::return_default_globalpluginparser();

    furry34.name = "furry34.com".to_string();
    furry34.version = 0;
    furry34.stored_info = Some(sharedtypes::StoredInfo::Storage(vec![]));
    furry34.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (10, Duration::from_secs(1)),
            sites: vec![SITE_NAME.to_string()],
            num_threads: None,
            modifiers: vec![],
            ..Default::default()
        },
    ));
    furry34.callbacks = vec![sharedtypes::GlobalCallbacks::Start(
        sharedtypes::StartupThreadType::Inline,
    )];

    vec![furry34]
}

/// Gets the post id from the url if it exists
fn extract_post_id(url: &str) -> Option<u64> {
    let url = Url::parse(url).ok()?;

    let mut segments = url.path_segments()?;
    match (segments.next(), segments.next(), segments.next()) {
        (Some("post"), Some(id), None) => id.parse::<u64>().ok(),
        _ => None,
    }
}

/// Gets the timestamp from a string if it exists
fn timestamp_ms(s: &str) -> Option<i64> {
    let dt = DateTime::parse_from_rfc3339(s).ok()?;
    Some(dt.timestamp_millis())
}

/// Gets a namespace from a internal type int from their website
fn match_tag_type_to_namespace(type_int: i32) -> Option<sharedtypes::GenericNamespaceObj> {
    match type_int {
        2 => Some(sharedtypes::GenericNamespaceObj {
            name: "furry34_tag_copyright".to_string(),
            description: Some("Who is the copyright holder of this media".to_string()),
        }),
        1 => Some(sharedtypes::GenericNamespaceObj {
            name: "furry34_tag_general".to_string(),
            description: Some("The general tags on the post".to_string()),
        }),
        4 => Some(sharedtypes::GenericNamespaceObj {
            name: "furry34_tag_character".to_string(),
            description: Some("The character that appears in the post".to_string()),
        }),
        8 => Some(sharedtypes::GenericNamespaceObj {
            name: "furry34_tag_artist".to_string(),
            description: Some("The artist who drew the post".to_string()),
        }),

        _ => None,
    }
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

    let mut search_terms = Vec::new();

    for param in params {
        if let sharedtypes::ScraperParam::Url(url_param) = param {
            if let Some(post_id) = extract_post_id(url_param) {
                let mut scraperdata = scraperdata.clone();
                let mut job_user_data = scraperdata.job.user_data.clone();
                job_user_data.insert("post_id".to_string(), post_id.to_string());

                scraperdata
                    .job
                    .user_data
                    .insert("post_id".to_string(), post_id.to_string());
                scraperdata.job = sharedtypes::DbJobsObj {
                    priority: *POST_PRIORITY,
                    site: SITE_NAME.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(url_param.to_string())],
                    jobmanager: sharedtypes::DbJobsManager {
                        jobtype: sharedtypes::DbJobType::Scraper,
                        ..Default::default()
                    },
                    user_data: job_user_data,
                    ..Default::default()
                };
                out.push(scraperdata);
            }
        } else if let sharedtypes::ScraperParam::Normal(search_param) = param {
            search_terms.push(search_param.clone());
        }
    }

    // This code handles adding search terms into the query object
    if !search_terms.is_empty() | scraperdata.job.user_data.contains_key("search_skip") {
        let mut job_user_data = scraperdata.job.user_data.clone();
        if let Some(_skip) = job_user_data.get("search_skip")
            && let Some(take) = job_user_data.get("search_take")
            && let Some(offset) = job_user_data.get("search_offset")
        {
            let new_offset = offset.parse::<u32>().unwrap() + 1;
            let skip_amt = take.parse::<u32>().unwrap() * new_offset;
            job_user_data.insert("search_skip".to_string(), skip_amt.to_string());
            job_user_data.insert("search_offset".to_string(), new_offset.to_string());
        } else {
            job_user_data.insert("search_skip".to_string(), "0".to_string());
            job_user_data.insert("search_take".to_string(), "30".to_string());
            job_user_data.insert("search_offset".to_string(), "0".to_string());
        }

        let search_terms_string = json::from(search_terms.clone());
        job_user_data.insert("search_terms".to_string(), search_terms_string.to_string());

        let data_skip = job_user_data
            .get("search_skip")
            .unwrap()
            .parse::<u32>()
            .unwrap();
        let data_take = job_user_data
            .get("search_take")
            .unwrap()
            .parse::<u32>()
            .unwrap();

        let data = match scraperdata.job.user_data.get("cursor") {
            None => {
                json::object! {
                    checkHasMore: true,
                    countTotal: true,
                    filterAi: false,
                    includeTags: search_terms.clone(),
                    skip: data_skip,
                    take: data_take,
                    sortBy: 0
                }
            }
            Some(cursor_text) => {
                json::object! {
                        checkHasMore: true,
                        countTotal: true,
                        filterAi: false,
                        includeTags: search_terms,
                        skip: data_skip,
                        take: data_take,
                        sortBy: 0,
                cursor: cursor_text.to_string()
                    }
            }
        };

        let url = format!("{SITE_BASE}api/v2/post/search/root");
        let job = sharedtypes::DbJobsObj {
            priority: sharedtypes::DEFAULT_PRIORITY,
            site: SITE_NAME.to_string(),
            param: vec![sharedtypes::ScraperParam::UrlPost(sharedtypes::UrlPost {
                url,
                post_data: data.to_string(),
                modifiers: vec![sharedtypes::TargetModifiers {
                    target: sharedtypes::ModifierTarget::Text,
                    modifier: sharedtypes::ScraperModifiers::Header((
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )),
                }],
            })],
            jobmanager: sharedtypes::DbJobsManager {
                jobtype: sharedtypes::DbJobType::Scraper,
                ..Default::default()
            },
            user_data: job_user_data,
            ..Default::default()
        };
        out.push(sharedtypes::ScraperDataReturn {
            job,
            ..Default::default()
        });
    }
    out
}

/// Extracts data from posts
fn parse_post(
    js: json::JsonValue,
    post_id: Option<&String>,
    out: &mut Vec<sharedtypes::ScraperReturn>,
    scraperdata: &sharedtypes::ScraperDataReturn,
) {
    let mut files = HashSet::new();
    let mut jobs = HashSet::new();

    // If we're dealing with a post
    if let Some(post_id) = post_id {
        let mut tag_list = Vec::new();
        tag_list.push(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "furry34_post_id".to_string(),
                description: Some("The posts id for furry34".to_string()),
            },
            tag: post_id.to_string(),
            ..Default::default()
        });
        if let Some(timestamp) = timestamp_ms(&js["created"].to_string()) {
            tag_list.push(sharedtypes::TagObject {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "furry34_post_creation_date".to_string(),
                    description: Some(
                        "When the post was created on the furry34 website".to_string(),
                    ),
                },
                tag: timestamp.to_string(),
                ..Default::default()
            });
        }

        for source in js["data"]["sources"].members() {
            tag_list.push(sharedtypes::TagObject {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "furry34_post_sources".to_string(),
                    description: Some("Additional sources for a post.".to_string()),
                },
                tag: source.to_string(),
                ..Default::default()
            });
        }

        for tag in js["tags"].members() {
            if !tag["value"].is_empty()
                && !tag["type"].is_empty()
                && tag["type"].is_number()
                && let Some(type_num) = tag["type"].as_i32()
            {
                if let Some(namespace) = match_tag_type_to_namespace(type_num) {
                    tag_list.push(sharedtypes::TagObject {
                        namespace,
                        tag: tag["value"].to_string(),
                        ..Default::default()
                    });
                } else {
                    panic!("{post_id}, missing tag type");
                }
            }
        }

        if let Some(url) = scraperdata.job.user_data.get("file_url") {
            let file = sharedtypes::FileObject {
                source: Some(sharedtypes::FileSource::Url(url.to_string())),
                tag_list,
                ..Default::default()
            };

            files.insert(file);
        }
    } else {
        for member in js["items"].members() {
            if let Some(post_id) = member["id"].as_u32() {
                let post_url = format!("{SITE_BASE}post/{}", post_id);
                let mut scraperdata = scraperdata.clone();
                scraperdata.job.user_data.clear();
                scraperdata
                    .job
                    .user_data
                    .insert("post_id".to_string(), post_id.to_string());
                scraperdata.job = sharedtypes::DbJobsObj {
                    priority: *POST_PRIORITY,
                    site: SITE_NAME.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(post_url.to_string())],
                    jobmanager: sharedtypes::DbJobsManager {
                        jobtype: sharedtypes::DbJobType::Scraper,
                        ..Default::default()
                    },
                    user_data: scraperdata.job.user_data,
                    ..Default::default()
                };
                jobs.insert(scraperdata);
            }
        }
        if let Some(has_more) = js["hasMore"].as_bool()
            && let Some(cursor_text) = js["cursor"].as_str()
            && has_more
        {
            let mut job_user_data = scraperdata.job.user_data.clone();

            job_user_data.insert("cursor".to_string(), cursor_text.to_string());
            let mut data_skip = job_user_data
                .get("search_skip")
                .unwrap()
                .parse::<u32>()
                .unwrap();
            let data_take = job_user_data
                .get("search_take")
                .unwrap()
                .parse::<u32>()
                .unwrap();

            data_skip += data_take;
            job_user_data.insert("search_skip".to_string(), data_skip.to_string());

            let search_terms_string = job_user_data.get("search_terms").unwrap();
            let search_terms = json::parse(search_terms_string).unwrap();

            let data = match scraperdata.job.user_data.get("cursor") {
                None => {
                    json::object! {
                        checkHasMore: true,
                        countTotal: true,
                        filterAi: false,
                        includeTags: search_terms,
                        skip: data_skip,
                        take: data_take,
                        sortBy: 0
                    }
                }
                Some(cursor_text) => {
                    json::object! {
                            checkHasMore: true,
                            countTotal: true,
                            filterAi: false,
                            includeTags: search_terms,
                            skip: data_skip,
                            take: data_take,
                            sortBy: 0,
                    cursor: cursor_text.to_string()
                        }
                }
            };

            let url = format!("{SITE_BASE}api/v2/post/search/root");

            jobs.insert(sharedtypes::ScraperDataReturn {
                job: sharedtypes::DbJobsObj {
                    priority: sharedtypes::DEFAULT_PRIORITY,
                    site: SITE_NAME.to_string(),
                    param: vec![sharedtypes::ScraperParam::UrlPost(sharedtypes::UrlPost {
                        url,
                        post_data: data.to_string(),
                        modifiers: vec![sharedtypes::TargetModifiers {
                            target: sharedtypes::ModifierTarget::Text,
                            modifier: sharedtypes::ScraperModifiers::Header((
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )),
                        }],
                    })],
                    jobmanager: sharedtypes::DbJobsManager {
                        jobtype: sharedtypes::DbJobType::Scraper,
                        ..Default::default()
                    },
                    user_data: job_user_data,
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }

    out.push(sharedtypes::ScraperReturn::Data(
        sharedtypes::ScraperObject {
            files,
            jobs,
            ..Default::default()
        },
    ));
}

/// Fixes media links for the urls
fn fix_url_to_media(url: url::Url) -> url::Url {
    let mut url = url.clone();

    url.set_path(
        &url.path()
            .replace("picsmall", "pic")
            .replace("mov480", "mov")
            .replace("//", "/"),
    );

    url
}

/// Extracts out the image or videos url
fn parse_post_html(
    html_input: &str,
    post_id: String,
    out: &mut Vec<sharedtypes::ScraperReturn>,
    _source_url: &str,
    _scraperdata: &sharedtypes::ScraperDataReturn,
) {
    let mut jobs = HashSet::new();

    let document = Html::parse_document(html_input);
    let selector =
        Selector::parse(r#"img[class="img ng-star-inserted"], source[type="video/mp4"]"#).unwrap();

    for element in document.select(&selector) {
        if let Some(src) = element.attr("src") {
            let url = match url::Url::parse(src) {
                Ok(url) => url,
                Err(err) => match err {
                    url::ParseError::RelativeUrlWithoutBase => {
                        if let Ok(url) = url::Url::parse(&format!("{SITE_BASE}{}", src)) {
                            url
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                },
            };
            let url = fix_url_to_media(url);

            let mut user_data = BTreeMap::new();

            user_data.insert("post_id".to_string(), post_id.to_string());
            user_data.insert("file_url".to_string(), url.to_string());

            jobs.insert(sharedtypes::ScraperDataReturn {
                job: sharedtypes::DbJobsObj {
                    priority: *POST_PRIORITY,
                    site: SITE_NAME.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(format!(
                        "{SITE_BASE}api/v2/post/{}",
                        post_id
                    ))],
                    jobmanager: sharedtypes::DbJobsManager {
                        jobtype: sharedtypes::DbJobType::Scraper,
                        ..Default::default()
                    },
                    user_data: user_data.clone(),
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }
    out.push(sharedtypes::ScraperReturn::Data(
        sharedtypes::ScraperObject {
            jobs,
            ..Default::default()
        },
    ));
}

///
/// Parses return from download.
///
#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    source_url: &str,
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut out = Vec::new();

    //println!("{}", html_input);
    if let Ok(js) = json::parse(html_input) {
        if scraperdata.job.user_data.contains_key("search_offset")
            | scraperdata.job.user_data.contains_key("post_id")
        {
            parse_post(
                js,
                scraperdata.job.user_data.get("post_id"),
                &mut out,
                scraperdata,
            );
        }
    } else if let Some(_post_id) = scraperdata.job.user_data.get("post_id")
        && let Some(_file_url) = scraperdata.job.user_data.get("file_url")
    {
        let jobs = HashSet::new();
        /*jobs.insert(sharedtypes::ScraperDataReturn {
            job: sharedtypes::DbJobsObj {
                priority: sharedtypes::DEFAULT_PRIORITY - 2,
                site: SITE_NAME.to_string(),
                param: vec![sharedtypes::ScraperParam::Url(format!("{SITE_BASE}api/v2/post/{}", post_id))],
                jobmanager: sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::Scraper,
                    ..Default::default()
                },
                user_data: scraperdata.user_data.clone(),
                ..Default::default()
            },
            ..Default::default()
        });*/
        out.push(sharedtypes::ScraperReturn::Data(
            sharedtypes::ScraperObject {
                jobs,
                ..Default::default()
            },
        ));
    } else if let Some(post_id) = scraperdata.job.user_data.get("post_id") {
        parse_post_html(
            html_input,
            post_id.to_string(),
            &mut out,
            source_url,
            scraperdata,
        );
    }
    out
}
