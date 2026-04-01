use chrono::{DateTime, Utc};
use json::JsonValue;
use regex::Regex;
use std::{collections::HashSet, time::Duration};

#[path = "../../../src/client.rs"]
mod client;
#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut defaultscraper = sharedtypes::return_default_globalpluginparser();

    defaultscraper.name = "Kemono".into();
    defaultscraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(2)),
            sites: vec![
                "kemono.cr".into(),
                "kemono.cr".into(),
                "kemono".into(),
                "Kemono".into(),
            ],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![
                sharedtypes::TargetModifiers {
                    target: sharedtypes::ModifierTarget::Media,
                    modifier: sharedtypes::ScraperModifiers::Useragent(
                        "Mozilla/5.0 (X11; Linux x86_64; rv:147.0) Gecko/20100101 Firefox/147.0"
                            .to_string(),
                    ),
                },
                sharedtypes::TargetModifiers {
                    target: sharedtypes::ModifierTarget::Media,
                    modifier: sharedtypes::ScraperModifiers::Header((
                        "Accept".to_string(),
                        "image/avif,image/webp,image/png,image/svg+xml,image/*;q=0.8,*/*;q=0.5"
                            .to_string(),
                    )),
                },
                sharedtypes::TargetModifiers {
                    target: sharedtypes::ModifierTarget::Media,
                    modifier: sharedtypes::ScraperModifiers::Timeout(None),
                },
            ],
        },
    ));

    vec![defaultscraper]
}

#[unsafe(no_mangle)]
pub fn url_dump(
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperDataReturn> {
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
    object: &mut Vec<sharedtypes::ScraperReturn>,
) {
    let mut tags = HashSet::new();
    let mut files = HashSet::new();

    // --- Parse timestamp ---
    let real_time = match input_post["added"]
        .as_str()
        .and_then(|t| string_to_unixms(t))
    {
        Some(t) => t,
        None => {
            client::log("Kemono Cannot parse time".to_string());
            return;
        }
    };

    // --- Core IDs ---
    let post_id_str = input_post["id"].as_str().unwrap_or("").to_string();
    let version_id = format!("{}_{}", post_id_str, real_time);

    // --- Version anchor ---
    let version_subtag = Some(sharedtypes::SubTag {
        namespace: get_genericnamespaceobj(Returntype::PostId, sitetype),
        tag: post_id_str.clone(),
        tag_type: sharedtypes::TagType::Normal,
        limit_to: Some(sharedtypes::Tag {
            namespace: get_genericnamespaceobj(Returntype::PostVersion, sitetype),
            tag: version_id.to_string(),
        }),
    });

    // --- Timestamp (metadata now) ---
    tags.insert(sharedtypes::TagObject {
        namespace: get_genericnamespaceobj(Returntype::PostAdded, sitetype),
        tag: real_time.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: version_subtag.clone(),
    });

    // --- Title ---
    if let Some(title) = input_post["title"].as_str()
        && !title.is_empty()
    {
        tags.insert(sharedtypes::TagObject {
            namespace: get_genericnamespaceobj(Returntype::PostTitle, sitetype),
            tag: title.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: version_subtag.clone(),
        });
    }

    // --- Content ---
    if let Some(content) = input_post["content"].as_str()
        && !content.is_empty()
    {
        tags.insert(sharedtypes::TagObject {
            namespace: get_genericnamespaceobj(Returntype::PostContent, sitetype),
            tag: content.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: version_subtag.clone(),
        });
    }

    // --- User ---
    if let Some(user) = input_post["user"].as_str() {
        tags.insert(sharedtypes::TagObject {
            namespace: get_genericnamespaceobj(Returntype::UserId, sitetype),
            tag: user.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: version_subtag.clone(),
        });
    }

    // --- Embed ---
    if input_post["embed"].is_object() {
        if let Some(url) = input_post["embed"]["url"].as_str() {
            tags.insert(sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::EmbedUrl, sitetype),
                tag: url.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: version_subtag.clone(),
            });
        }

        if let Some(subject) = input_post["embed"]["subject"].as_str() {
            tags.insert(sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::EmbedSubject, sitetype),
                tag: subject.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: version_subtag.clone(),
            });
        }

        if let Some(desc) = input_post["embed"]["description"].as_str() {
            tags.insert(sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::EmbedDescription, sitetype),
                tag: desc.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: version_subtag.clone(),
            });
        }
    }

    // --- Post tags ---
    for item in input_post["tags"].members() {
        if let Some(tag_str) = item.as_str() {
            tags.insert(sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::PostTags, sitetype),
                tag: tag_str.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: version_subtag.clone(),
            });
        }
    }

    let mut offset = 0;

    if input_post["file"].is_object() {
        let name = input_post["file"]["name"].as_str().unwrap_or("");
        let path = input_post["file"]["path"].as_str().unwrap_or("");

        if !path.is_empty() {
            let subtag = Some(sharedtypes::SubTag {
                namespace: get_genericnamespaceobj(Returntype::PostAttachments, sitetype),
                tag: offset.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                limit_to: Some(sharedtypes::Tag {
                    tag: version_subtag.clone().unwrap().limit_to.unwrap().tag,
                    namespace: version_subtag.clone().unwrap().limit_to.unwrap().namespace,
                }),
            });

            let file_name = sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::PostAttachmentName, sitetype),
                tag: format!("{}_{}", version_id, name),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: subtag.clone(),
            };

            let url = format!("https://{site}/data{}", path);

            files.insert(sharedtypes::FileObject {
                source: Some(sharedtypes::FileSource::Url(vec![url])),
                hash: sharedtypes::HashesSupported::None,
                tag_list: vec![sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Add,
                    tags: vec![file_name],
                }],
                ..Default::default()
            });
            offset += 1;
        }
    }

    // --- Helper for files ---
    let mut handle_attachments = |members: json::iterators::Members| {
        for (cnt, attachment) in members.enumerate() {
            let index = cnt + offset;
            let name = attachment["name"].as_str().unwrap_or("").to_string();
            let path = attachment["path"].as_str().unwrap_or("");

            let subtag = Some(sharedtypes::SubTag {
                namespace: get_genericnamespaceobj(Returntype::PostAttachments, sitetype),
                tag: index.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                limit_to: Some(sharedtypes::Tag {
                    tag: version_subtag.clone().unwrap().limit_to.unwrap().tag,
                    namespace: version_subtag.clone().unwrap().limit_to.unwrap().namespace,
                }),
            });

            let file_name = sharedtypes::TagObject {
                namespace: get_genericnamespaceobj(Returntype::PostAttachmentName, sitetype),
                tag: format!("{}_{}", version_id, name),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: subtag.clone(),
            };

            let url = format!("https://{site}/data{}", path);

            files.insert(sharedtypes::FileObject {
                source: Some(sharedtypes::FileSource::Url(vec![url])),
                hash: sharedtypes::HashesSupported::None,
                tag_list: vec![sharedtypes::FileTagAction {
                    operation: sharedtypes::TagOperation::Add,
                    tags: vec![file_name],
                }],
                ..Default::default()
            });
        }
    };

    // --- Files ---
    handle_attachments(input_post["file"].members());
    handle_attachments(input_post["attachments"].members());

    // --- Final push ---
    object.push(sharedtypes::ScraperReturn::Data(
        sharedtypes::ScraperObject {
            tags,
            files,
            ..Default::default()
        },
    ));
}
#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    source_url: &str,
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut out = Vec::new();
    let mut jobs = HashSet::new();

    // Handles when we're getting creators posts
    if let Some(name) = scraperdata.job.user_data.get("confirmed user") {
        if let Some(jobtype) = scraperdata.job.user_data.get("jobtype") {
            if jobtype == "posts" {
                if let Ok(parsed_json) = json::parse(html_input) {
                    if parsed_json.members().len() == 50 {
                        let page_offset = match scraperdata.job.user_data.get("page_offset") {
                            None => 50,
                            Some(offset) => {
                                let num: Result<u64, _> = offset.parse::<u64>();

                                match num {
                                    Ok(number) => number + 50,
                                    Err(_) => 50,
                                }
                            }
                        };

                        let url = if page_offset == 50 {
                            format!("{}?o={}", source_url, page_offset)
                        } else {
                            source_url.replace(
                                &{ page_offset - 50 }.to_string(),
                                &page_offset.to_string(),
                            )
                        };

                        let mut scraperdata = scraperdata.clone();

                        scraperdata
                            .job
                            .user_data
                            .insert("page_offset".into(), page_offset.to_string());

                        jobs.insert(sharedtypes::ScraperDataReturn {
                            job: sharedtypes::DbJobsObj {
                                priority: sharedtypes::DEFAULT_PRIORITY,
                                site: scraperdata.job.site.to_string(),
                                param: vec![sharedtypes::ScraperParam::Url(url)],
                                jobmanager: sharedtypes::DbJobsManager {
                                    jobtype: sharedtypes::DbJobType::Scraper,
                                    ..Default::default()
                                },
                                system_data: scraperdata.job.system_data,
                                user_data: scraperdata.job.user_data,
                                ..Default::default()
                            },
                            skip_conditions: vec![],
                        });
                    }

                    for item in parsed_json.members() {
                        let mut scraperdata = scraperdata.clone();

                        scraperdata.job.user_data.clear();

                        scraperdata
                            .job
                            .user_data
                            .insert("confirmed user".into(), name.into());
                        scraperdata
                            .job
                            .user_data
                            .insert("confirmed user id".into(), item["id"].to_string());
                        scraperdata
                            .job
                            .user_data
                            .insert("confirmed service".into(), item["service"].to_string());

                        let url = format!(
                            "https://kemono.cr/api/v1/{}/user/{}/post/{}",
                            item["service"], item["user"], item["id"]
                        );
                        scraperdata
                            .job
                            .user_data
                            .insert("jobtype".into(), "post".to_string());

                        jobs.insert(sharedtypes::ScraperDataReturn {
                            job: sharedtypes::DbJobsObj {
                                priority: sharedtypes::DEFAULT_PRIORITY - 2,
                                site: scraperdata.job.site.to_string(),
                                param: vec![sharedtypes::ScraperParam::Url(url.clone())],
                                jobmanager: sharedtypes::DbJobsManager {
                                    jobtype: sharedtypes::DbJobType::Scraper,
                                    ..Default::default()
                                },
                                system_data: scraperdata.job.system_data,
                                user_data: scraperdata.job.user_data,
                                ..Default::default()
                            },
                            skip_conditions: vec![],
                        });
                    }
                }
            } else if jobtype == "post"
                && let Ok(parsed_json) = json::parse(html_input)
            {
                if parsed_json.is_empty() {
                    return vec![sharedtypes::ScraperReturn::Nothing];
                }
                parse_post(
                    &parsed_json["post"],
                    &"kemono.cr".into(),
                    &Sitetype::Kemono,
                    &mut out,
                );
            }
        }
        out.push(sharedtypes::ScraperReturn::Data(
            sharedtypes::ScraperObject {
                jobs,
                ..Default::default()
            },
        ));

        return out;
    }

    // Handles the main creators page scraping. After we determine if theirs a user here
    if let Some(user) = scraperdata.job.user_data.get("Potiential User")
        && let Ok(parsed_json) = json::parse(html_input)
        && parsed_json.is_array()
        && scraperdata.job.param.len() == 1
    {
        for item in parsed_json.members() {
            let name = item["name"].to_string();
            if name.contains(user) {
                let mut scraperdata = scraperdata.clone();

                scraperdata.job.user_data.clear();

                scraperdata
                    .job
                    .user_data
                    .insert("confirmed user".into(), name);
                scraperdata
                    .job
                    .user_data
                    .insert("confirmed user id".into(), item["id"].to_string());
                scraperdata
                    .job
                    .user_data
                    .insert("confirmed service".into(), item["service"].to_string());

                let url = format!(
                    "https://kemono.cr/api/v1/{}/user/{}/posts",
                    item["service"], item["id"]
                );
                jobs.insert(sharedtypes::ScraperDataReturn {
                    job: sharedtypes::DbJobsObj {
                        priority: sharedtypes::DEFAULT_PRIORITY - 2,
                        site: scraperdata.job.site.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(url.clone())],
                        jobmanager: sharedtypes::DbJobsManager {
                            jobtype: sharedtypes::DbJobType::Scraper,
                            ..Default::default()
                        },
                        system_data: scraperdata.job.system_data,
                        user_data: scraperdata.job.user_data,
                        ..Default::default()
                    },
                    skip_conditions: vec![],
                });
            }
        }
    }
    out.push(sharedtypes::ScraperReturn::Data(
        sharedtypes::ScraperObject {
            jobs,
            ..Default::default()
        },
    ));

    out
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
    EmbedUrl,
    EmbedSubject,
    EmbedDescription,
    PostTags,
    PostVersion,
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
            description: Some(format!(
                "Any attachments added to a post on {site}. Records their position relative to a post"
            )),
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
        Returntype::EmbedUrl => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Embed_Url",),
            description: Some(format!("Something that was embedded into the site: {site}")),
        },
        Returntype::EmbedSubject => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Embed_Subject",),
            description: Some(format!("An embeds title for {site}... Normally")),
        },
        Returntype::EmbedDescription => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Embed_Description",),
            description: Some(format!("A description of the embed posted to {site}")),
        },
        Returntype::PostTags => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Tag",),
            description: Some(format!("A tag associated with a post on {site}")),
        },
        Returntype::PostVersion => sharedtypes::GenericNamespaceObj {
            name: format!("{site}_Post_Version",),
            description: Some(format!("A unique version of the post {site}")),
        },
    }
}

pub struct Componenttype {
    site: Sitetype,
    site_str: Option<String>,
    service: Option<String>,
    user_id: Option<String>,
    post_id: Option<String>,
}

///
/// Returns a type based on an input string.
///
fn parse_type(inp: &str) -> Option<Componenttype> {
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

    if let Some(reg_match) = site_regex.captures(inp)
        && let Some(reg) = reg_match.get(0)
    {
        site_str = Some(reg.as_str().to_string());
    }

    let mut component = Componenttype {
        site,
        site_str,
        service: None,
        user_id: None,
        post_id: None,
    };
    // matches both the service and the user id and post id
    let user_regex = Regex::new(r"/([a-z]+)/user/([0-9]+)/post/([0-9]+)").unwrap();

    if let Some(reg_match) = user_regex.captures(inp) {
        if let Some(service) = &reg_match.get(1) {
            component.service = Some(service.as_str().to_string());
        }
        if let Some(userid) = &reg_match.get(2) {
            component.user_id = Some(userid.as_str().to_string());
        }
        if let Some(userid) = &reg_match.get(3) {
            component.post_id = Some(userid.as_str().to_string());
        }

        return Some(component);
    }

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
    match (&inp.site_str, &inp.service, &inp.user_id, &inp.post_id) {
        (Some(site), Some(service), Some(user_id), None) => {
            Some(format!("https://{site}/api/v1/{service}/user/{user_id}"))
        }
        (Some(site), Some(service), Some(user_id), Some(post_id)) => Some(format!(
            "https://{site}/api/v1/{service}/user/{user_id}/post/{post_id}"
        )),
        _ => None,
    }
}

fn filter_params(
    item: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperDataReturn> {
    let mut out = Vec::new();
    for item in item {
        let mut vec_user_string: Vec<&str> = Vec::new();

        match item {
            sharedtypes::ScraperParam::Normal(something)
            | sharedtypes::ScraperParam::Url(something) => {
                if let Some(component) = parse_type(something) {
                    // If we extracted no post id then we need to query
                    if let Some(url) = generate_userid_search(&component) {
                        if component.post_id.is_none() {
                            let mut scraperdata = scraperdata.clone();
                            scraperdata.job.user_data.insert(
                                "confirmed user".into(),
                                component.user_id.clone().unwrap(),
                            );
                            scraperdata
                                .job
                                .user_data
                                .insert("jobtype".into(), "posts".into());

                            /*  scraperdata.job = sharedtypes::JobScraper {
                                site: site_to_string(&component.site).to_string(),
                                job_type: sharedtypes::DbJobType::Params,
                                param: vec![],
                            };
                            out.push((format!("{}/posts", url), scraperdata.clone()));*/
                            out.push(sharedtypes::ScraperDataReturn {
                                job: sharedtypes::DbJobsObj {
                                    priority: sharedtypes::DEFAULT_PRIORITY - 1,
                                    site: scraperdata.job.site.to_string(),
                                    param: vec![sharedtypes::ScraperParam::Url(format!(
                                        "{}/posts",
                                        url
                                    ))],
                                    jobmanager: sharedtypes::DbJobsManager {
                                        jobtype: sharedtypes::DbJobType::Scraper,
                                        ..Default::default()
                                    },
                                    system_data: scraperdata.job.system_data,
                                    user_data: scraperdata.job.user_data,
                                    ..Default::default()
                                },

                                ..Default::default()
                            });
                        } else {
                            let mut scraperdata = scraperdata.clone();
                            scraperdata.job.user_data.insert(
                                "confirmed user".into(),
                                component.user_id.clone().unwrap(),
                            );
                            scraperdata
                                .job
                                .user_data
                                .insert("jobtype".into(), "post".into());

                            /* scraperdata.job = sharedtypes::JobScraper {
                                site: site_to_string(&component.site).to_string(),
                                job_type: sharedtypes::DbJobType::Params,
                                param: vec![],
                            };
                            out.push((url.to_string(), scraperdata.clone()));*/

                            out.push(sharedtypes::ScraperDataReturn {
                                job: sharedtypes::DbJobsObj {
                                    priority: sharedtypes::DEFAULT_PRIORITY - 1,
                                    site: scraperdata.job.site.to_string(),
                                    param: vec![sharedtypes::ScraperParam::Url(url.clone())],
                                    jobmanager: sharedtypes::DbJobsManager {
                                        jobtype: sharedtypes::DbJobType::Scraper,
                                        ..Default::default()
                                    },
                                    system_data: scraperdata.job.system_data,
                                    user_data: scraperdata.job.user_data,
                                    ..Default::default()
                                },

                                ..Default::default()
                            });
                        }
                    }
                } else {
                    vec_user_string = something.split(' ').collect();
                }
            }
            /*  sharedtypes::ScraperParam::Url(url) => {
                out.push(sharedtypes::ScraperDataReturn {
                    job: sharedtypes::DbJobsObj {
                        priority: sharedtypes::DEFAULT_PRIORITY,
                        site: scraperdata.job.site.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(url.clone())],
                        jobmanager: sharedtypes::DbJobsManager {
                            jobtype: sharedtypes::DbJobType::Scraper,
                            ..Default::default()
                        },
                        system_data: scraperdata.job.system_data.clone(),
                        user_data: scraperdata.job.user_data.clone(),
                        ..Default::default()
                    },

                    ..Default::default()
                });
            }*/
            //sharedtypes::ScraperParam::Url(url) => out.push((url.into(), scraperdata.clone())),
            _ => {}
        }

        for user in vec_user_string.iter() {
            let mut scraperdata = scraperdata.clone();
            scraperdata
                .job
                .user_data
                .insert("Potiential User".into(), user.to_string());
            /*  scraperdata.job = sharedtypes::JobScraper {
                site: "kemono.cr".into(),
                param: vec![sharedtypes::ScraperParam::Normal(user.to_string())],
                job_type: sharedtypes::DbJobType::Scraper,
            };*/

            scraperdata
                .job
                .user_data
                .insert("action".into(), "creators".into());
            out.push(sharedtypes::ScraperDataReturn {
                job: sharedtypes::DbJobsObj {
                    priority: sharedtypes::DEFAULT_PRIORITY,
                    site: scraperdata.job.site.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(
                        "https://kemono.cr/api/v1/creators.txt".into(),
                    )],
                    jobmanager: sharedtypes::DbJobsManager {
                        jobtype: sharedtypes::DbJobType::Scraper,
                        ..Default::default()
                    },
                    system_data: scraperdata.job.system_data,
                    user_data: scraperdata.job.user_data,
                    ..Default::default()
                },

                ..Default::default()
            });

            //out.push(("https://kemono.cr/api/v1/creators.txt".into(), scraperdata));
        }
    }
    out
}
