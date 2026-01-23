#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;
use chrono::DateTime;
use chrono::Utc;
use json::JsonValue;
use scraper::Selector;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::time::Duration;
use unescape::unescape;
use url::Url;

pub const SITE: &str = "createporn";

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let site_names = ["createaifurry"];
    let mut out = Vec::new();
    for site_name in site_names {
        let site_url = format!("https://www.{}.com", site_name);
        let mut createporn = sharedtypes::return_default_globalpluginparser();
        createporn.name = format!("createporn_{}", &site_name);
        createporn.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
            sharedtypes::ScraperInfo {
                ratelimit: (4, Duration::from_secs(1)),
                sites: vec![site_name.to_string()],
                priority: sharedtypes::DEFAULT_PRIORITY,
                num_threads: None,
                modifiers: vec![
                    sharedtypes::TargetModifiers {
                        target: sharedtypes::ModifierTarget::Text,
                        modifier: sharedtypes::ScraperModifiers::Header((
                            "Accept".to_string(),
                            "application/json, text/plain, */*".to_string(),
                        )),
                    },
                    sharedtypes::TargetModifiers {
                        target: sharedtypes::ModifierTarget::Text,
                        modifier: sharedtypes::ScraperModifiers::Header((
                            "Origin".to_string(),
                            site_url.clone(),
                        )),
                    },
                    sharedtypes::TargetModifiers {
                        target: sharedtypes::ModifierTarget::Text,
                        modifier: sharedtypes::ScraperModifiers::Header((
                            "X-Origin".to_string(),
                            site_url.clone(),
                        )),
                    },
                    sharedtypes::TargetModifiers {
                        target: sharedtypes::ModifierTarget::Text,
                        modifier: sharedtypes::ScraperModifiers::Header((
                            "Referer".to_string(),
                            site_url,
                        )),
                    },
                ],
            },
        ));

        out.push(createporn);
    }

    out
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
    for param in scraperdata.job.param.iter() {
        if let crate::sharedtypes::ScraperParam::Normal(temp) = param {
            let url = Url::parse(temp).ok();

            // Early exit if we see a post
            if temp.contains("/post/") {
                ret.push((temp.to_string(), scraperdata.clone()));
                continue;
            }
            if let Some(url) = url {
                let is_animated = url.path() == "/gifs";

                let mut filter = None;
                let mut kind = None;
                let mut style = None;

                for (key, value) in url.query_pairs() {
                    match key.as_ref() {
                        "filter" => filter = Some(value.into_owned()),
                        "type" => kind = Some(value.into_owned()),
                        "style" => style = Some(value.into_owned()),
                        _ => {}
                    }
                }
                //Default for kind if nothing was given
                if kind.is_none() {
                    kind = Some("hot".to_string());
                }

                let mut api_url = "https://api.createporn.com/post/".to_string();
                if is_animated {
                    api_url.push_str("gifs")
                } else {
                    api_url.push_str("feed")
                }
                if filter.is_some() || kind.is_some() || style.is_some() {
                    api_url.push('?');
                }
                let mut should_add_amp = false;
                if let Some(filter) = filter {
                    if should_add_amp {
                        api_url.push('&');
                    } else {
                        should_add_amp = true;
                    }
                    api_url.push_str("filter=");
                    api_url.push_str(&filter);
                }
                if let Some(kind) = kind {
                    if should_add_amp {
                        api_url.push('&');
                    } else {
                        should_add_amp = true;
                    }

                    api_url.push_str("type=");
                    api_url.push_str(&kind);
                }
                if let Some(style) = style {
                    if should_add_amp {
                        api_url.push('&');
                    } else {
                        should_add_amp = true;
                    }
                    api_url.push_str("generatorId=");
                    api_url.push_str(&style);
                }

                ret.push((api_url, scraperdata.clone()));
            }
        }
    }
    ret
}

#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    _source_url: &str,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut out = Vec::new();

    let mut tag = HashSet::new();

    if let Ok(json) = json::parse(html_input) {
        // Username page
        if json["username"].is_string() && json["userId"].is_string() {
            if let Some(url) = scraperdata.user_data.get("file_url") {
                let mut tags = Vec::new();
                tags.push(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: format!("createporn_{}_author_name", scraperdata.job.site),
                        description: Some("An Author who uploaded the image / video.".to_string()),
                    },
                    tag: json["username"].to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: Some(sharedtypes::SubTag {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: format!("createporn_{}_author_id", scraperdata.job.site),
                            description: Some(
                                "A author's unique ID as it pertains to their internal database"
                                    .to_string(),
                            ),
                        },
                        tag: json["userId"].to_string(),
                        limit_to: None,
                        tag_type: sharedtypes::TagType::Normal,
                    }),
                });
                let mut file = HashSet::new();
                file.insert(sharedtypes::FileObject {
                    source: Some(sharedtypes::FileSource::Url(url.to_string())),
                    hash: sharedtypes::HashesSupported::None,
                    tag_list: tags,
                    skip_if: vec![],
                });
                out.push(sharedtypes::ScraperReturn::Data(
                    sharedtypes::ScraperObject {
                        file,
                        tag: HashSet::new(),
                        flag: vec![],
                    },
                ));
            }
        }
        if json["info"]["next"].is_string() {
            tag.insert(sharedtypes::TagObject {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: "".to_string(),
                    description: None,
                },
                tag: json["info"]["next"].to_string(),
                tag_type: sharedtypes::TagType::ParseUrl((
                    sharedtypes::ScraperData {
                        job: sharedtypes::JobScraper {
                            site: scraperdata.job.site.to_string(),
                            param: vec![sharedtypes::ScraperParam::Url(
                                json["info"]["next"].to_string(),
                            )],
                            job_type: sharedtypes::DbJobType::Scraper,
                        },

                        system_data: BTreeMap::new(),
                        user_data: BTreeMap::new(),
                    },
                    None,
                )),
                relates_to: None,
            });
            out.push(sharedtypes::ScraperReturn::Data(
                sharedtypes::ScraperObject {
                    file: HashSet::new(),
                    tag,
                    flag: vec![],
                },
            ));
        }

        for files in json["results"].members() {
            let mut tag = HashSet::new();
            let mut tag_list = Vec::new();
            let mut file_id = None;
            if files["_id"].is_string() {
                file_id = Some(files["_id"].to_string());
                let url_to_scrape = format!(
                    "https://www.{}.com/post/{}",
                    scraperdata.job.site, files["_id"]
                );
                // Adds job for getting file details
                tag.insert(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "".to_string(),
                        description: None,
                    },
                    tag: url_to_scrape.clone(),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        sharedtypes::ScraperData {
                            job: sharedtypes::JobScraper {
                                site: scraperdata.job.site.to_string(),
                                param: vec![sharedtypes::ScraperParam::Url(url_to_scrape)],
                                job_type: sharedtypes::DbJobType::Scraper,
                            },

                            system_data: BTreeMap::new(),
                            user_data: BTreeMap::new(),
                        },
                        None,
                    )),
                    relates_to: None,
                });
                out.push(sharedtypes::ScraperReturn::Data(
                    sharedtypes::ScraperObject {
                        file: HashSet::new(),
                        tag,
                        flag: vec![],
                    },
                ));

                tag_list.push(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: format!("createporn_{}_id", scraperdata.job.site),
                        description: Some(
                            "A file's unique id inside of the createporn site".to_string(),
                        ),
                    },
                    tag: files["_id"].to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: None,
                });
            }
            if files["imageUrl"].is_string() {
                // Scrapes the username off a post if we need to
                if let Some(file_id) = file_id {
                    add_username_search(
                        files["imageUrl"].to_string(),
                        file_id,
                        scraperdata,
                        &mut tag_list,
                    );
                }

                let mut file = HashSet::new();
                file.insert(sharedtypes::FileObject {
                    source: Some(sharedtypes::FileSource::Url(files["imageUrl"].to_string())),
                    hash: sharedtypes::HashesSupported::None,
                    tag_list,
                    skip_if: vec![],
                });
                out.push(sharedtypes::ScraperReturn::Data(
                    sharedtypes::ScraperObject {
                        file,
                        tag: HashSet::new(),
                        flag: vec![],
                    },
                ));
            }
        }
    } else {
        // Hot dogpoo code that parses info from the /post/ page through an internal API call uses
        // next.js so its hydrating
        let html = scraper::Html::parse_document(html_input);
        let selector = Selector::parse("script").unwrap();
        if let Some(frag) = html.select(&selector).nth_back(2)
            && let Some(text) = frag.text().next()
        {
            let mut text = text.to_string();
            text.pop();
            let text = text.trim();
            let text = text.replace("self.__next_f.push(", "");

            let json_str = match text.find(':') {
                Some(idx) => &text[idx + 1..],
                None => &text,
            };
            let json_str = json_str.trim();
            let json_str = json_str.replace("\\\"", "\"");
            let json_str = unescape(&json_str).unwrap_or_else(|| json_str.to_string());
            let json_str = json_str
                .trim()
                // Remove trailing quote if the string ends with ']"'
                .trim_end_matches("\"]")
                // Remove trailing comma if it exists before the final bracket
                .trim_end_matches(',')
                // Optional: remove surrounding quotes if the whole thing is wrapped in quotes
                .trim_matches('"');
            if let Ok(json) = json::parse(json_str) {
                for child in json.members() {
                    // Parse JSON
                    let v = json::parse(&child.to_string()).ok();
                    if v.is_none() {
                        continue;
                    }
                    let v = v.unwrap();
                    let mut posts = Vec::new();
                    find_posts(&v, &mut posts);

                    for post in posts {
                        //let _id = post["_id"].as_str().unwrap_or("");
                        //let url = post["imageUrl"].as_str().unwrap_or("");
                        //let _prompt = post["customPrompt"].as_str().unwrap_or("");
                        let mut tags: Vec<sharedtypes::TagObject> = Vec::new();
                        // Extracts out the tags from an entry
                        for (tag_id, json_val) in post["tags"].entries() {
                            if let Some((entry, val)) = json_val.entries().next() {
                                if let Some(val) = val.as_str() {
                                    tags.push(sharedtypes::TagObject{
namespace: sharedtypes::GenericNamespaceObj{
                                                name: format!("createporn_{}_tag_name", scraperdata.job.site),
                                                description: Some("A tag's name thats directly from the site.".to_string())
                                            },
                                            tag: val.to_string(),
                                            tag_type: sharedtypes::TagType::Normal,
                                            relates_to: Some(
                                                sharedtypes::SubTag{
                                                    namespace: sharedtypes::GenericNamespaceObj{
                                                        name: format!("createporn_{}_tag_id", scraperdata.job.site),
                                                        description: Some("A tag's unique ID as it pertains to their internal database".to_string())
                                                    },
                                                    tag: tag_id.to_string(),
                                                    limit_to: None,
                                                    tag_type: sharedtypes::TagType::Normal
                                                }
                                            )
                                        });
                                }
                            }
                        }

                        let mut file_id_storage = None;
                        if let Some(id) = post["_id"].as_str() {
                            file_id_storage = Some(id.to_string());
                            if !id.is_empty() {
                                tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!("createporn_{}_id", scraperdata.job.site),
                                        description: Some(
                                            "A file's unique id inside of the createporn site"
                                                .to_string(),
                                        ),
                                    },
                                    tag: id.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                });
                            }
                        }

                        if let Some(id) = post["prompt"].as_str() {
                            if !id.is_empty() {
                                tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_prompt",
                                            scraperdata.job.site
                                        ),
                                        description: Some(
                                            "A posts prompt that was used to generate the image or video"
                                                .to_string(),
                                        ),
                                    },
                                    tag: id.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                });
                            }
                        }
                        if let Some(id) = post["customPrompt"].as_str() {
                            if !id.is_empty() {
                                tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_prompt",
                                            scraperdata.job.site
                                        ),
                                        description: Some(
                                            "A posts prompt that was used to generate the image or video"
                                                .to_string(),
                                        ),
                                    },
                                    tag: id.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                });
                                //Attempting to get the individual prompts from a custom prompt
                                for tag in id.split(&['.', ',']) {
                                    let tag = tag.trim();
                                    if tag.is_empty() {
                                        continue;
                                    }
                                    tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_prompt_individual",
                                            scraperdata.job.site
                                        ),
                                        description: Some(
                                            "An individual post's prompt that was used to generate the image or video"
                                                .to_string(),
                                        ),
                                    },
                                    tag: tag.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                });
                                }
                            }
                        }

                        if let Some(id) = post["gifPrompt"].as_str() {
                            if !id.is_empty() {
                                tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_promptgif",
                                            scraperdata.job.site
                                        ),
                                        description: Some(
                                            "A posts prompt that was used to generate the image or video gif specific??"
                                                .to_string(),
                                        ),
                                    },
                                    tag: id.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                });
                            }
                        }
                        //dbg!(&post);

                        if let Some(iso_time) = post["createdAt"].as_str() {
                            // Parse the ISO 8601 string into a DateTime<Utc>
                            let dt: DateTime<Utc> =
                                iso_time.parse().expect("Invalid datetime format");

                            // If you want milliseconds
                            let unix_ms = dt.timestamp_millis();

                            tags.push(sharedtypes::TagObject {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: format!(
                                        "createporn_{}_createdAt_timestamp",
                                        scraperdata.job.site
                                    ),
                                    description: Some(
                                        "When a post was created on the site".to_string(),
                                    ),
                                },
                                tag: unix_ms.to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: None,
                            });
                        }

                        let mut id_storage = None;
                        for (entry, val) in post["generator"].entries() {
                            if entry.contains("_id") {
                                id_storage = Some(val.to_string());
                            }
                            if val.is_empty() {
                                continue;
                            }

                            if entry.contains("shortName")
                                && let Some(ref id) = id_storage
                            {
                                tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_generator",
                                            scraperdata.job.site
                                        ),
                                        description: Some(
                                            "A posts generator that was used to make the post"
                                                .to_string(),
                                        ),
                                    },
                                    tag: val.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: Some(
  sharedtypes::SubTag{
                                                    namespace: sharedtypes::GenericNamespaceObj{
                                                        name: format!("createporn_{}_generator_id", scraperdata.job.site),
                                                        description: Some("A image/vids generator unique ID as it pertains to their internal database".to_string())
                                                    },
                                                    tag: id.to_string(),
                                                    limit_to: None,
                                                    tag_type: sharedtypes::TagType::Normal
                                                }

                                        ),
                                });
                                break;
                            }
                        }

                        if let Some(url) = post["imageUrl"].as_str() {
                            let mut tag = HashSet::new();
                            if let Some(file_id) = file_id_storage {
                                add_username_search(
                                    url.to_string(),
                                    file_id,
                                    scraperdata,
                                    &mut tags,
                                );
                            }
                            let mut file = HashSet::new();
                            file.insert(sharedtypes::FileObject {
                                source: Some(sharedtypes::FileSource::Url(url.to_string())),
                                hash: sharedtypes::HashesSupported::None,
                                tag_list: tags,
                                skip_if: vec![],
                            });
                            out.push(sharedtypes::ScraperReturn::Data(
                                sharedtypes::ScraperObject {
                                    file,
                                    tag,
                                    flag: vec![],
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    out
}

/// Adds a username to search for
fn add_username_search(
    image_url: String,
    file_id: String,
    scraperdata: &sharedtypes::ScraperData,
    tag: &mut Vec<sharedtypes::TagObject>,
) {
    let mut user_data = BTreeMap::new();
    user_data.insert("file_url".to_string(), image_url);

    let username_url = format!("https://api.createporn.com/post/username/{}", file_id);

    let recursion = scraperdata
        .system_data
        .get("recursion")
        .filter(|&tf| tf == "true")
        .map(|_| {
            sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                namespace: sharedtypes::GenericNamespaceObj {
                    name: format!("createporn_{}_id", scraperdata.job.site),
                    description: Some(
                        "A file's unique id inside of the createporn site".to_string(),
                    ),
                },
                tag: file_id.to_string(),
            })
        });


    tag.push(sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "".to_string(),
            description: None,
        },
        tag: username_url.clone(),
        tag_type: sharedtypes::TagType::ParseUrl((
            sharedtypes::ScraperData {
                job: sharedtypes::JobScraper {
                    site: scraperdata.job.site.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(username_url)],
                    job_type: sharedtypes::DbJobType::Scraper,
                },

                system_data: BTreeMap::new(),
                user_data,
            },
            recursion,
        )),
        relates_to: None,
    });
}

///
/// Rips out post values
///
fn find_posts(value: &JsonValue, posts: &mut Vec<JsonValue>) {
    match value {
        JsonValue::Array(arr) => {
            for item in arr {
                find_posts(item, posts);
            }
        }
        JsonValue::Object(_) => {
            // Check if "action" -> "post" exists
            if !value["action"]["post"].is_null() {
                posts.push(value["action"]["post"].clone());
            }

            // Recurse through all object values
            for (_, val) in value.entries() {
                find_posts(val, posts);
            }
        }
        _ => {}
    }
}
