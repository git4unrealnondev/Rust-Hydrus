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
    let site_names = ["createaifurry", "createhentai", "createporn"];
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
                    sharedtypes::TargetModifiers {
                        target: sharedtypes::ModifierTarget::Media,
                        modifier: sharedtypes::ScraperModifiers::Timeout(10),
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
    _: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut ret = Vec::new();
    for param in scraperdata.job.param.iter() {
        if let crate::sharedtypes::ScraperParam::Url(temp) = param
            && let Some(url) = parse_url(temp)
        {
            ret.push((url.clone(), scraperdata.clone()));
        }
        if let crate::sharedtypes::ScraperParam::Normal(temp) = param
            && let Some(url) = parse_url(temp)
        {
            ret.push((url.clone(), scraperdata.clone()));
        }
    }
    ret
}

/*
fn parse_url(temp: &str) -> Option<String> {
    let url = Url::parse(temp).ok()?;

    let path = url.path();
    let query = url.query().unwrap_or("");

    let is_searching = path.contains("search");
    let is_animated = path.contains("/gif") || query.contains("type=gif");
    let is_user = path.contains("/user");

    // Early exit for direct posts
    if (temp.contains("/post/") || temp.contains("/gif/")) && (!is_searching && !is_user) {
        return Some(temp.to_string());
    }

    let mut filter = None;
    let mut kind = None;
    let mut style = None;
    let mut search = None;
    let mut limit = is_searching.then(|| "20".to_string());

    for (key, value) in url.query_pairs() {
        let value = value.into_owned();
        match key.as_ref() {
            "filter" => filter = Some(value),
            "type" => kind = Some(value),
            "style" => style = Some(value),
            "search" => search = Some(value),
            "limit" => limit = Some(value),
            _ => {}
        }
    }

    let mut api_url = Url::parse("https://api.createporn.com/post/").unwrap();


        dbg!(&kind, &is_searching, &is_animated, &is_user);

// Hack preserved

  if is_searching && is_animated {
        kind = Some("gif".to_string());
    }


    // Default kind
    if kind.is_none() {
        kind = Some(filter.clone().unwrap_or_else(|| "hot".to_string()));
            }

    match (is_user, is_searching, is_animated) {
        (true, _, true) => api_url.set_path("post/profile-gifs"),
        (true, _, false) => api_url.set_path("post/profile-images"),
        (_, true, _) => api_url.set_path("post/search"),
        (_, false, true) => api_url.set_path("post/gifs"),
        _ => api_url.set_path("post/feed"),
    }

    if is_user && let Some(user_id) = url.path_segments()?.next_back() {
        api_url
            .query_pairs_mut()
            .append_pair("user", user_id)
            .append_pair("sort", filter.as_deref().unwrap_or("top"));
    }

        {
        let mut qp = api_url.query_pairs_mut();

        if is_searching {
            if let Some(l) = &limit {
                qp.append_pair("limit", l);
            }
            if let Some(s) = &search {
                qp.append_pair("searchQuery", s);
            }
            if let Some(f) = &filter {
                qp.append_pair("sort", f);
            }
            if is_animated && let Some(k) = &kind {
                qp.append_pair("type", k);
            }
        } else if !is_user && let Some(k) = &kind {
            qp.append_pair("type", k);
        }

        if let Some(s) = &style
            && s != "all"
        {
            qp.append_pair(if is_animated { "style" } else { "generatorId" }, s);
        }
    }

    Some(api_url.to_string())
}*/

use std::collections::HashMap;
fn parse_url(input: &str) -> Option<String> {
    if input.is_empty() {
        return None;
    }

    let src = Url::parse(input).ok()?;
    let mut api = Url::parse("https://api.createporn.com").unwrap();

    let segments: Vec<&str> = src.path_segments().map(|s| s.collect()).unwrap_or_default();

    let params: HashMap<String, String> = src.query_pairs().into_owned().collect();

    let filter = params.get("filter").map(String::as_str);
    let style = params.get("style").map(String::as_str);
    let search = params.get("search").map(String::as_str);
    let ty = params.get("type").map(String::as_str);

    let is_user = segments.first() == Some(&"user");
    let is_search = segments.contains(&"search");
    let is_gif = segments.iter().any(|s| *s == "gif" || *s == "gifs");

    /* ---------------- PATH ---------------- */

    match (is_user, is_search, is_gif) {
        (true, _, true) => api.set_path("post/profile-gifs"),
        (true, _, false) => api.set_path("post/profile-images"),
        (_, true, _) => api.set_path("post/search"),
        (_, false, true) => api.set_path("post/gifs"),
        _ => api.set_path("post/feed"),
    }

    /* ---------------- QUERY ---------------- */

    let mut query: Vec<(&str, &str)> = vec![];

    match api.path() {
        /* -------- USER -------- */
        "/post/profile-images" | "/post/profile-gifs" => {
            let user_id = segments.get(1)?;
            query.push(("user", user_id));
            query.push(("sort", filter.unwrap_or("top")));
        }

        /* -------- SEARCH -------- */
        "/post/search" => {
            query.push(("limit", "20"));

            if let Some(q) = search {
                query.push(("searchQuery", q));
            }

            query.push(("sort", filter.unwrap_or("new")));

            // ONLY place where type=gif is valid
            if is_gif || ty == Some("gif") {
                query.push(("type", "gif"));
            }

            if let Some(s) = style
                && s != "all"
            {
                query.push(("style", s));
            }
        }

        /* -------- GIF FEED -------- */
        "/post/gifs" => {
            query.push(("type", filter.unwrap_or("hot")));

            if let Some(s) = style
                && s != "all"
            {
                query.push(("style", s));
            }
        }

        /* -------- IMAGE FEED -------- */
        "/post/feed" => {
            let sort = filter.unwrap_or("hot");

            // "new" is a PATH segment, not a query param
            if sort == "new" {
                api.set_path("post/feed/new");
            } else {
                query.push(("type", sort));
            }

            if let Some(s) = style {
                if s != "all" {
                    query.push(("generatorId", s));
                }
            }
        }

        _ => {}
    }

    api.query_pairs_mut().extend_pairs(query);
    Some(api.into())
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
            // Gets the users username and maps it to their userid
            if let Some(url) = scraperdata.user_data.get("file_url") {
                let mut tags = Vec::new();

                //add_username_post_search(json["userId"].to_string(), scraperdata, &mut tags);

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
        if json["info"]["next"].is_string() && !json["info"]["next"].is_empty() {
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

                        system_data: scraperdata.system_data.clone(),
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

            // Sets up userid for username searching
            let mut user_id = None;
            if files["author"].is_string() {
                user_id = Some(files["author"].to_string());
            }
            if files["_id"].is_string() {
                let url_to_scrape = format!(
                    "https://www.{}.com/post/{}",
                    scraperdata.job.site, files["_id"]
                );

                // if we get recursion,true as an input then we should rescrape otherwise skip

                let should_grab_post: Option<sharedtypes::SkipIf> = (scraperdata
                    .system_data
                    .get("recursion")
                    != Some(&"true".to_string()))
                .then(|| {
                    sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: format!("createporn_{}_id", scraperdata.job.site),
                            description: Some(
                                "A file's unique id inside of the createporn site".to_string(),
                            ),
                        },
                        tag: files["_id"].to_string(),
                    })
                });

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

                            system_data: scraperdata.system_data.clone(),
                            user_data: BTreeMap::new(),
                        },
                        should_grab_post,
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
                if let Some(user_id) = user_id {
                    add_username_search(
                        files["imageUrl"].to_string(),
                        user_id,
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
        for frag in html.select(&selector) {
            if let Some(text) = frag.text().next() {
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
                                if let Some((_, val)) = json_val.entries().next()
                                    && let Some(val) = val.as_str()
                                {
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

                            let mut user_id_storage = None;

                            if let Some(user_id) = post["author"].as_str() {
                                user_id_storage = Some(user_id.to_string());
                            }

                            if let Some(id) = post["_id"].as_str()
                                && !id.is_empty()
                            {
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

                            if let Some(id) = post["prompt"].as_str()
                                && !id.is_empty()
                            {
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
                            if let Some(id) = post["customPrompt"].as_str()
                                && !id.is_empty()
                            {
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
                                for tag in smart_split(id) {
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

                            if let Some(id) = post["gifPrompt"].as_str()
                                && !id.is_empty()
                            {
                                tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_prompt_gif",
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
                                }); //Attempting to get the individual prompts from a custom prompt
                                for tag in smart_split(id) {
                                    let tag = tag.trim();
                                    if tag.is_empty() {
                                        continue;
                                    }
                                    tags.push(sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: format!(
                                            "createporn_{}_prompt_gif_individual",
                                            scraperdata.job.site
                                        ),
                                        description: Some(
                                            "An individual post's prompt that was used to generate video"
                                                .to_string(),
                                        ),
                                    },
                                    tag: tag.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                });
                                }
                            }

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

                            if let Some(url) = post["imageUrl"]
                                .as_str()
                                .or_else(|| post["videoUrl"].as_str())
                            {
                                let tag = HashSet::new();
                                if let Some(file_id) = user_id_storage {
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
    }
    out
}

/// Strips out , and . if they look like delimiters
fn smart_split(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];

        match c {
            ',' => {
                push_buf(&mut out, &mut buf);
            }
            '.' => {
                let prev = chars.get(i.wrapping_sub(1));
                let next = chars.get(i + 1);

                let is_decimal = prev.is_some_and(|p| p.is_ascii_digit())
                    && next.is_some_and(|n| n.is_ascii_digit());

                if is_decimal {
                    buf.push('.');
                } else {
                    // Treat as delimiter (handles ".", ". ", " . ")
                    push_buf(&mut out, &mut buf);
                }
            }
            _ => buf.push(c),
        }
    }

    push_buf(&mut out, &mut buf);
    out
}

fn push_buf(out: &mut Vec<String>, buf: &mut String) {
    let s = buf.trim();
    if !s.is_empty() {
        out.push(s.to_string());
    }
    buf.clear();
}

/// Adds a username to search for
fn add_username_search(
    image_url: String,
    user_id: String,
    scraperdata: &sharedtypes::ScraperData,
    tag: &mut Vec<sharedtypes::TagObject>,
) {
    let mut user_data = BTreeMap::new();
    user_data.insert("file_url".to_string(), image_url);

    // Supports searching the users for their posts
    if scraperdata
        .system_data
        .get("username-search")
        .filter(|&tf| tf == "true")
        .is_some()
    {
        let user_base_url = format!("https://{}.com/user/{}", scraperdata.job.site, user_id);

        // Supports getting the users posts and animated "gifs"
        for user_change_url in ["", "?type=gif"] {
            let user_post_url = format!("{}{}", user_base_url, user_change_url);
            if let Some(scrape_url) = parse_url(&user_post_url) {
                tag.push(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "".to_string(),
                        description: None,
                    },
                    tag: scrape_url.clone(),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        sharedtypes::ScraperData {
                            job: sharedtypes::JobScraper {
                                site: scraperdata.job.site.to_string(),
                                param: vec![sharedtypes::ScraperParam::Url(scrape_url)],
                                job_type: sharedtypes::DbJobType::Scraper,
                            },

                            system_data: scraperdata.system_data.clone(),
                            user_data: BTreeMap::new(),
                        },
                        None,
                    )),
                    relates_to: None,
                });
            }
        }
    }
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
            if !value["action"]["gif"].is_null() {
                posts.push(value["action"]["gif"].clone());
            }

            // Recurse through all object values
            for (_, val) in value.entries() {
                find_posts(val, posts);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Tests the url parsing of the url extractor
    #[test]
    fn url_gen_test() {
        let urls = [
            ("".to_string(), None),
            (
                "https://www.createaifurry.com/".to_string(),
                Some("https://api.createporn.com/post/feed?type=hot".to_string()),
            ),
            //("https://www.createaifurry.com/?filter=new&type=image&style=all".to_string(), Some("https://api.createporn.com/post/feed/new".to_string())),
            //("".to_string(), Some()),
            (
                "https://www.createaifurry.com/gifs?style=all".to_string(),
                Some("https://api.createporn.com/post/gifs?type=hot".to_string()),
            ),
            (
                "https://www.createaifurry.com/?filter=top15&style=all".to_string(),
                Some("https://api.createporn.com/post/feed?type=top15".to_string()),
            ),(
                "https://www.createaifurry.com/post/search?filter=new&search=female&type=image&style=all".to_string(),
                Some("https://api.createporn.com/post/search?limit=20&searchQuery=female&sort=new".to_string()),
            ),(
                "https://www.createaifurry.com/gif/search?filter=new&search=female&type=gif&style=all".to_string(),
                Some("https://api.createporn.com/post/search?limit=20&searchQuery=female&type=gif&sort=new".to_string()),
            ),(
                "https://www.createaifurry.com/user/6860abf2415eadac56bab8b0".to_string(),
                Some("https://api.createporn.com/post/profile-images?user=6860abf2415eadac56bab8b0&sort=top".to_string()),
            ),(
                "https://www.createaifurry.com/user/6860abf2415eadac56bab8b0?filter=new&type=image".to_string(),
                Some("https://api.createporn.com/post/profile-images?user=6860abf2415eadac56bab8b0&sort=new".to_string()),
            ),(
                "https://www.createaifurry.com/user/6860abf2415eadac56bab8b0?filter=new&type=gif".to_string(),
                Some("https://api.createporn.com/post/profile-gifs?user=6860abf2415eadac56bab8b0&sort=new".to_string()),
            ),(
                "https://www.createaifurry.com/?style=lineart".to_string(),
                Some("https://api.createporn.com/post/feed?type=hot&generatorId=lineart".to_string()),
            ),(
                "https://www.createaifurry.com/gifs?style=lineart".to_string(),
                Some("https://api.createporn.com/post/gifs?type=hot&style=lineart".to_string()),
            ),(
                "https://www.createaifurry.com/gif/search?filter=top1&style=animecorev3&search=female".to_string(),
                Some("https://api.createporn.com/post/search?limit=20&style=animecorev3&searchQuery=female&type=gif&sort=top1".to_string()),
            ),(
                "https://www.createaifurry.com/user/6860abf2415eadac56bab8b0?type=gif".to_string(),
                Some("https://api.createporn.com/post/profile-gifs?user=6860abf2415eadac56bab8b0&sort=top".to_string()),
            ),(
                "https://www.createaifurry.com/gifs?filter=new&type=gif&style=all".to_string(),
                Some("https://api.createporn.com/post/gifs?type=new".to_string()),
            ), (
            "https://www.createaifurry.com/?filter=new&type=image&style=all".to_string(),
                Some("https://api.createporn.com/post/feed/new".to_string())
        )

        ];

        // Checks to see if the params are equal and if they aren't then then we check the url
        // itself
        for (ref url, ref valid) in urls {
            if valid.is_some() {
                let param1: HashMap<String, String> = Url::parse(&parse_url(url).unwrap())
                    .unwrap()
                    .query_pairs()
                    .into_owned()
                    .collect();
                let param2: HashMap<String, String> = Url::parse(&valid.clone().unwrap())
                    .unwrap()
                    .query_pairs()
                    .into_owned()
                    .collect();
                if param1 != param2 {
                    dbg!(&param1, &param2);
                    assert_eq!(parse_url(url), valid.clone());
                }
                assert!(param1 == param2);
            } else {
                assert_eq!(parse_url(url), valid.clone());
            }
        }
    }
}
