use chrono::{DateTime, Utc};
use json::JsonValue;
use regex::Regex;
use std::{collections::HashSet, time::Duration};

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut defaultscraper = sharedtypes::return_default_globalpluginparser();

    defaultscraper.name = "Kemono".into();
    defaultscraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec!["kemono.cr".into(), "kemono.cr".into()],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
        },
    ));

    vec![defaultscraper]
}

#[unsafe(no_mangle)]
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = vec![];

    for parsed in filter_params(params, scraperdata) {
        out.push(parsed);
    }

    out
}

///
/// Converts a string into unix epoch ms if it parses
///
fn string_to_unixms(inp: &str) -> Option<i64> {
    let inp = format!("{}Z", inp);
    if let Ok(temp) = &inp.parse::<DateTime<Utc>>() {
        return Some(temp.timestamp_millis());
    }
    None
}

///
/// Parses a post from kemono or coomer into a scraperobject
///
fn parse_post(
    input_post: &JsonValue,
    site: &String,
    sitetype: &Sitetype,
    object: &mut sharedtypes::ScraperObject,
) {
    let real_time;

    if let Some(time) = input_post["added"].as_str() {
        if let Some(temp_time) = string_to_unixms(time) {
            real_time = temp_time;
        } else {
            client::log(format!("Kemono Cannot parse time: {}", time));
            return;
        }
    } else {
        client::log("Kemono Cannot parse time".to_string());
        return;
    }

    let post_id = sharedtypes::TagObject {
        namespace: get_genericnamespaceobj(Returntype::PostAdded, sitetype),
        tag: real_time.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: Some(sharedtypes::SubTag {
            namespace: get_genericnamespaceobj(Returntype::PostId, sitetype),
            tag: input_post["id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
            limit_to: Some(sharedtypes::Tag {
                namespace: get_genericnamespaceobj(Returntype::Service, sitetype),
                tag: format!("{}_{}", site_to_string(sitetype), input_post["service"]),
            }),
        }),
    };

    let post_subtag = Some(sharedtypes::SubTag {
        namespace: get_genericnamespaceobj(Returntype::PostAdded, sitetype),
        tag: real_time.to_string(),
        limit_to: Some(sharedtypes::Tag {
            namespace: get_genericnamespaceobj(Returntype::PostId, sitetype),
            tag: input_post["id"].to_string(),
        }),

        tag_type: sharedtypes::TagType::Normal,
    });

    let title = sharedtypes::TagObject {
        namespace: get_genericnamespaceobj(Returntype::PostTitle, sitetype),
        tag: input_post["title"].to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: post_subtag.clone(),
    };
    if let Some(str_comment) = input_post["content"].as_str() {
        if !str_comment.is_empty() {
            let content = sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::PostContent, sitetype),
                tag: input_post["content"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: post_subtag.clone(),
            };

            object.tag.insert(content);
        }
    }
    let userid = sharedtypes::TagObject {
        namespace: get_genericnamespaceobj(Returntype::UserId, sitetype),
        tag: input_post["user"].to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: post_subtag.clone(),
    };

    object.tag.insert(post_id);
    object.tag.insert(title);
    object.tag.insert(userid);

    for (cnt, attachments) in input_post["attachments"].members().enumerate() {
        let file_position = sharedtypes::TagObject {
            namespace: get_genericnamespaceobj(Returntype::PostAttachments, sitetype),
            tag: cnt.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: post_subtag.clone(),
        };
        let file_name = sharedtypes::TagObject {
            namespace: get_genericnamespaceobj(Returntype::PostAttachmentName, sitetype),
            tag: attachments["name"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: post_subtag.clone(),
        };

        let url = format!("https://{site}/data{}", attachments["path"]);
        object.file.insert(sharedtypes::FileObject {
            source_url: Some(url),
            hash: sharedtypes::HashesSupported::None,
            tag_list: vec![file_name, file_position],
            skip_if: vec![],
        });
    }
}

#[unsafe(no_mangle)]
pub fn parser(
    html_input: &String,
    params: &Vec<sharedtypes::ScraperParam>,
    actual_params: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    // Handles when we're getting creators posts
    if let Some(name) = actual_params.user_data.get("confirmed user") {
        let mut out = sharedtypes::ScraperObject {
            file: HashSet::new(),
            tag: HashSet::new(),
        };
        if let Ok(parsed_json) = json::parse(html_input) {
            if parsed_json.is_empty() {
                return Err(sharedtypes::ScraperReturn::Nothing);
            }

            for post in parsed_json.members() {
                parse_post(post, &"kemono.cr".into(), &Sitetype::Kemono, &mut out);
            }
        }

        return Ok(out);
    }

    // Handles the main creators page scraping. After we determine if theirs a user here
    if let Some(user) = actual_params.user_data.get("Potiential User") {
        if let Ok(parsed_json) = json::parse(html_input) {
            if parsed_json.is_array() && params.len() == 1 {
                for item in parsed_json.members() {
                    let name = item["name"].to_string();
                    if name.contains(user) {
                        let mut scraperdata = actual_params.clone();

                        scraperdata.user_data.clear();

                        scraperdata.user_data.insert("confirmed user".into(), name);
                        scraperdata
                            .user_data
                            .insert("confirmed user id".into(), item["id"].to_string());
                        scraperdata
                            .user_data
                            .insert("confirmed service".into(), item["service"].to_string());

                        let mut tag = HashSet::new();
                        let site = scraperdata.job.site;

                        for offset in (0..=2147483647).step_by(50) {
                            let url = format!(
                                "https://kemono.cr/api/v1/{}/user/{}?o={}",
                                item["service"], item["id"], offset
                            );

                            scraperdata.job = sharedtypes::JobScraper {
                                site: site.clone(),
                                param: vec![sharedtypes::ScraperParam::Url(url.clone())],
                                job_type: sharedtypes::DbJobType::Scraper,
                            };

                            tag.insert(sharedtypes::TagObject {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "do not parse".into(),
                                    description: None,
                                },
                                tag: url.clone(),
                                tag_type: sharedtypes::TagType::ParseUrl((
                                    scraperdata.clone(),
                                    None,
                                )),
                                relates_to: None,
                            });
                        }
                        return Ok(sharedtypes::ScraperObject {
                            file: HashSet::new(),
                            tag,
                        });
                    }
                }
            }
        }
    }

    Err(sharedtypes::ScraperReturn::Nothing)
}

enum Returntype {
    PostId,
    UserId,
    Service,
    PostTitle,
    PostContent,
    PostOriginalTime,
    PostEditiedTime,
    PostAttachments,
    PostAttachmentName,
    PostAdded,
}

enum Sitetype {
    Kemono,
    Coomer,
}

fn site_to_string(site: &Sitetype) -> &'static str {
    match site {
        Sitetype::Kemono => "Kemono",
        Sitetype::Coomer => "Coomer",
    }
}
fn get_genericnamespaceobj(inp: Returntype, site: &Sitetype) -> sharedtypes::GenericNamespaceObj {
    let site = site_to_string(site);
    match inp {
        Returntype::PostId => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Id",),
            description: Some(format!("Used by {site} to mark a post id")),
        },
        Returntype::UserId => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_User_Id",),
            description: Some(format!("Used by {site} for a users unique id")),
        },
        Returntype::Service => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Service",),
            description: Some(format!("{site} is a service ")),
        },
        Returntype::PostTitle => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Title",),
            description: Some(format!("A title for a post on site: {site}")),
        },
        Returntype::PostContent => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Content",),
            description: Some(format!("Content related to a post on {site}")),
        },
        Returntype::PostOriginalTime => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Original_Time",),
            description: Some(format!(
                "A posts original post time on the paysite as recorded by {site}"
            )),
        },
        Returntype::PostEditiedTime => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Edited_Time",),
            description: Some(format!("The last time a post has been editied on {site}")),
        },
        Returntype::PostAttachments => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Attachments",),
            description: Some(format!("Any attachments added to a post on {site}. Records their position relative to a post")),
        },
        Returntype::PostAttachmentName => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Attachment_Name"),
            description: Some(format!(
                "A file's unique name as originally recorded by {site}"
            )),
        },
        Returntype::PostAdded => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Added",),
            description: Some(format!("Time when post was added to {site}")),
        },
    }
}

pub struct Componenttype {
    site: Sitetype,
    site_str: Option<String>,
    service: Option<String>,
    user_id: Option<String>,
}

///
/// Returns a type based on an input string.
///
fn parse_type(inp: &String) -> Option<Componenttype> {
    let site;
    if inp.contains("kemono.") {
        site = Sitetype::Kemono
    } else if inp.contains("coomer.") {
        site = Sitetype::Coomer
    } else {
        return None;
    }

    // matches a site
    let mut site_str = None;
    let site_regex = Regex::new(r"[a-z]+\.(su|party|cr)").unwrap();

    if let Some(reg_match) = site_regex.captures(inp) {
        if let Some(reg) = reg_match.get(0) {
            site_str = Some(reg.as_str().to_string());
        }
    }

    let mut component = Componenttype {
        site,
        site_str,
        service: None,
        user_id: None,
    };

    // matches both the service and the user id
    let user_regex = Regex::new(r"/([a-z]+)/user/([0-9]+)").unwrap();

    if let Some(reg_match) = user_regex.captures(inp) {
        if let Some(service) = &reg_match.get(1) {
            component.service = Some(service.as_str().to_string());
        }
        if let Some(userid) = &reg_match.get(2) {
            component.user_id = Some(userid.as_str().to_string());
        }

        return Some(component);
    }
    None
}

fn generate_userid_search(inp: &Componenttype) -> Option<String> {
    if let (Some(site_str), Some(userid), Some(service)) = (
        inp.site_str.clone(),
        inp.user_id.clone(),
        inp.service.clone(),
    ) {
        return Some(format!("https://{site_str}/api/v1/{service}/user/{userid}"));
    }
    None
}

fn filter_params(
    item: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = Vec::new();
    for item in item {
        let mut vec_user_string: Vec<&str> = Vec::new();

        match item {
            sharedtypes::ScraperParam::Normal(something) => {
                if let Some(component) = parse_type(something) {
                    if let Some(url) = generate_userid_search(&component) {
                        let mut scraperdata = scraperdata.clone();
                        scraperdata
                            .user_data
                            .insert("confirmed user".into(), component.user_id.clone().unwrap());
                        scraperdata.job = sharedtypes::JobScraper {
                            site: site_to_string(&component.site).to_string(),
                            job_type: sharedtypes::DbJobType::Params,
                            param: vec![],
                        };
                        for offset in (0..=1000000).step_by(50) {
                            out.push((
                                format!("{}{}", url, &format!("?o={}", offset)),
                                scraperdata.clone(),
                            ));
                        }
                    }
                } else {
                    vec_user_string = something.split(' ').collect();
                }
            }
            sharedtypes::ScraperParam::Url(url) => out.push((url.into(), scraperdata.clone())),
            _ => {}
        }

        for user in vec_user_string.iter() {
            let mut scraperdata = scraperdata.clone();
            scraperdata
                .user_data
                .insert("Potiential User".into(), user.to_string());
            scraperdata.job = sharedtypes::JobScraper {
                site: "kemono.cr".into(),
                param: vec![sharedtypes::ScraperParam::Normal(user.to_string())],
                job_type: sharedtypes::DbJobType::Scraper,
            };

            scraperdata
                .user_data
                .insert("action".into(), "creators".into());
            out.push(("https://kemono.cr/api/v1/creators.txt".into(), scraperdata));
        }
    }
    out
}
