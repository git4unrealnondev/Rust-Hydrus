use scraper::{Html, Selector, node::Text};
use std::{collections::HashSet, time::Duration};
use url::Url;

use crate::sharedtypes::DEFAULT_PRIORITY;
use std::collections::BTreeMap;

use chrono::{NaiveDateTime, TimeZone, Utc};
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

pub const LOCAL_NAME: &str = "YChan";
pub const SITE_LINK: &str = "https://ychan.net/";

///
/// Returns the info needed to scrape an fxembed system
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut defaultscraper = sharedtypes::return_default_globalpluginparser();

    defaultscraper.name = LOCAL_NAME.into();
    defaultscraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (2, Duration::from_secs(1)),
            sites: vec![LOCAL_NAME.into(), LOCAL_NAME.to_lowercase()],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![],
        },
    ));

    vec![defaultscraper]
}

///
/// Takes the raw user input and outputs a url to download
///
#[unsafe(no_mangle)]
pub fn url_dump(
    _params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = vec![];
    dbg!(&scraperdata);
    for param in scraperdata.job.param.iter() {
        match param {
            sharedtypes::ScraperParam::Normal(input) => {
                let url = Url::parse(&input);
                if url.is_err() {
                    continue;
                }
                let url = url.unwrap();

                let url_segments = url.path_segments();
                if url_segments.is_none() {
                    continue;
                }
                let mut url_segments = url_segments.unwrap();
                let board = url_segments.next();
                let sub_board = url_segments.next();

                if let Some(board) = board
                    && let Some(sub_board) = sub_board
                {
                    let mut scraperdata = scraperdata.clone();
                    scraperdata.user_data.insert("board".into(), board.into());
                    scraperdata
                        .user_data
                        .insert("sub_board".into(), sub_board.into());

                    let results = url
                        .query_pairs()
                        .find(|(key, _)| key == "results")
                        .map(|(_, value)| value.to_string());

                    if let Some(results) = results {
                        scraperdata.user_data.insert("result".into(), results);
                    }

                    out.push((input.clone(), scraperdata));
                } else {
                    // Gets the raw site
                    if input.contains(SITE_LINK) {
                        out.push((input.to_string(), scraperdata.clone()));
                    }
                }
            }
            sharedtypes::ScraperParam::Url(_input) => {}
            _ => {}
        }
    }

    out
}

fn parse_from_root_page(
    fragment: &Html,
    url_base: &Url,
    tag: &mut HashSet<sharedtypes::TagObject>,
) {
    // On the root of the site this manages to loop through the thread list and first page
    let selector = Selector::parse(r#"td[class="threadtitle"]"#).unwrap();
    for element in fragment.select(&selector) {
        if let Some(a) = element.child_elements().next() {
            if let Some(url_relative) = a.value().attr("href") {
                let url = url_base.join(url_relative);
                if url.is_err() {
                    continue;
                }
                let url = url.unwrap();
                tag.insert(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "".into(),
                        description: None,
                    },
                    tag: "".to_string(),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        sharedtypes::ScraperData {
                            job: sharedtypes::JobScraper {
                                site: LOCAL_NAME.to_string(),
                                param: vec![sharedtypes::ScraperParam::Url(
                                    url.as_str().to_string(),
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
            }
        }
    }
}

fn parse_from_pool_page(
    fragment: &Html,
    url_base: &Url,
    tag: &mut HashSet<sharedtypes::TagObject>,
    source_url: &str,
) {
    // Gets all thumb posts
    let selector = Selector::parse(r#"div[class="thumb_wrapper"]"#).unwrap();

    // Parses everything from the main page.
    for element in fragment.select(&selector) {
        if let Some(link) = element.children().next() {
            if let Some(rel) = link.value().as_element() {
                if let Some(url_relative) = rel.attr("href") {
                    let url_finished = url_base.join(url_relative);
                    if url_finished.is_err() {
                        continue;
                    }
                    let url_finished = url_finished.clone().unwrap();
                    let url_segments = url_finished.path_segments();
                    if url_segments.is_none() {
                        continue;
                    }
                    let mut url_segments = url_segments.unwrap();

                    // view handler for root
                    let post_id = if url_relative.contains("/view/") {
                        let _ = url_segments.next();
                        url_segments.next()
                    } else {
                        let _ = url_segments.next();
                        let _ = url_segments.next();
                        url_segments.next()
                    };

                    let url_finished = url_finished.as_str().to_string();

                    // Catch to stop adding the same url back into the db
                    if url_finished == source_url {
                        continue;
                    }

                    let early_stop = match post_id {
                        None => None,
                        Some(post_id) => {
                            Some(sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "ychan_post_id".into(),
                                    description: Some("A unique post id from ychan".into()),
                                },
                                tag: post_id.into(),
                            }))
                        }
                    };

                    tag.insert(sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "".into(),
                            description: None,
                        },
                        tag: "".to_string(),
                        tag_type: sharedtypes::TagType::ParseUrl((
                            sharedtypes::ScraperData {
                                job: sharedtypes::JobScraper {
                                    site: LOCAL_NAME.to_string(),
                                    param: vec![sharedtypes::ScraperParam::Url(url_finished)],
                                    job_type: sharedtypes::DbJobType::Scraper,
                                },
                                system_data: BTreeMap::new(),
                                user_data: BTreeMap::new(),
                            },
                            early_stop,
                        )),
                        relates_to: None,
                    });
                }
            }
        }
    }

    // Gets the other results for other pages when passing x
    let selector = Selector::parse(r#"td[class="firstpages"]"#).unwrap();
    for element in fragment.select(&selector) {
        for href in element.child_elements() {
            if let Some(url_relative) = href.value().attr("href") {
                let url_finished = url_base.join(url_relative);

                if url_finished.is_err() {
                    continue;
                }
                let url_finished = url_finished.unwrap().as_str().to_string();
                tag.insert(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "".into(),
                        description: None,
                    },
                    tag: "".to_string(),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        sharedtypes::ScraperData {
                            job: sharedtypes::JobScraper {
                                site: LOCAL_NAME.to_string(),
                                param: vec![sharedtypes::ScraperParam::Url(url_finished)],
                                job_type: sharedtypes::DbJobType::Scraper,
                            },
                            system_data: BTreeMap::new(),
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

fn parse_from_file_page<'a>(
    fragment: &'a Html,
    file_tags: &mut Vec<sharedtypes::TagObject>,
    file_url: &mut Option<&'a str>,
) {
    // Should only ever be one file per page but this works gets the raw file url for the image
    let selector = Selector::parse(r#"meta[property="og:image"]"#).unwrap();
    for element in fragment.select(&selector) {
        if element.attr("content") != Some("//ychan.net/img/ychan_default_thumb.png") {
            *file_url = element.attr("content");
        }
    }
    // Extracts tags from a post
    let selector = Selector::parse(r#"meta[name="keywords"]"#).unwrap();
    for element in fragment.select(&selector) {
        if let Some(keywords) = element.attr("content") {
            // When the shitassed site returns blank tags it actually returns this
            if keywords
                == "yiffy, yiff, yiffing, furry, fur, furr, furre, anthro, anthropomorphic, art, ychan, imageboard, art, hentai, gay, lesbian, straight, bisexual, fursuit, bodypaint, plush, plushie, plushy, scaly, scaley, avian, feathery"
            {
                continue;
            }

            let keyword_vec: Vec<&str> = keywords.split(", ").collect();

            for keyword in keyword_vec {
                let tag = sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "ychan_tag".into(),
                        description: Some("A file has x tag from ychan".into()),
                    },
                    tag: keyword.to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: None,
                };
                file_tags.push(tag);
            }
        }
    }

    // Will get author, submitted date
    let selector = Selector::parse(r#"table[class="info"]"#).unwrap();
    for info_frag in fragment.select(&selector) {
        if let Some(tbody) = info_frag.first_child() {
            if let Some(tr) = tbody.next_sibling() {
                if let Some(td) = tr.first_child() {
                    if let Some(main_info) = td.first_child() {
                        let text_list: Vec<Text> = main_info
                            .children()
                            .filter_map(|noderef| noderef.value().as_text().map(|t| t.clone()))
                            .collect();
                        if let Some(author) = text_list.get(1) {
                            let tag = sharedtypes::TagObject {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "ychan_author".into(),
                                    description: Some(
                                        "A person on ychan who uploaded the image".into(),
                                    ),
                                },
                                tag: author.trim().to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: None,
                            };

                            file_tags.push(tag);
                        }
                        if let Some(post_time) = text_list.get(3) {
                            // The garbage format that ychan uses for their date
                            let fmt = "%b%d/%y, %H:%M";

                            if let Ok(post_timestamp) =
                                NaiveDateTime::parse_from_str(post_time.trim(), fmt)
                            {
                                let dt_utc = Utc.from_utc_datetime(&post_timestamp);

                                // Get UNIX timestamp (seconds since epoch)
                                let unix_timestamp = dt_utc.timestamp();
                                let tag = sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: "ychan_post_timestamp".into(),
                                        description: Some(
                                            "A timestamp of when the image was uploaded to ychan"
                                                .into(),
                                        ),
                                    },
                                    tag: unix_timestamp.to_string(),
                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: None,
                                };

                                file_tags.push(tag);
                            }
                        }
                    }
                }
            }
        }
    }

    // Should only ever get one board and subboard and the post id
    let selector = Selector::parse(r#"meta[property="og:url"]"#).unwrap();
    for element in fragment.select(&selector) {
        if let Some(page_url) = element.attr("content") {
            if let Ok(url) = Url::parse(&page_url) {
                let url_segments = url.path_segments();
                if url_segments.is_none() {
                    continue;
                }
                let mut url_segments = url_segments.unwrap();
                let board = url_segments.next();
                let sub_board = url_segments.next();
                let post_id = url_segments.next();
                if let Some(board) = board
                    && let Some(sub_board) = sub_board
                    && let Some(post_id) = post_id
                {
                    let tag = sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "ychan_post_id".into(),
                            description: Some("A unique post id from ychan".into()),
                        },
                        tag: post_id.to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: Some(sharedtypes::SubTag {
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: "ychan_subboard".to_string(),
                                description: Some(
                                    "The subboard that the image was posted to".into(),
                                ),
                            },
                            tag: sub_board.to_string(),
                            tag_type: sharedtypes::TagType::Normal,
                            limit_to: Some(sharedtypes::Tag {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "ychan_board".into(),
                                    description: Some(
                                        "The mainboard that the image was posted to.".into(),
                                    ),
                                },
                                tag: board.into(),
                            }),
                        }),
                    };

                    file_tags.push(tag);
                }
            }
        }
    }
}

///
/// After the text download step. Parses the response from the text download
///
#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    source_url: &str,
    _scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let fragment = Html::parse_fragment(html_input);

    // Should never fail
    let url_base = Url::parse(SITE_LINK).unwrap();

    let mut file = HashSet::new();
    let mut tag = HashSet::new();

    parse_from_root_page(&fragment, &url_base, &mut tag);

    parse_from_pool_page(&fragment, &url_base, &mut tag, source_url);

    let mut file_url = None;
    let mut file_tags = Vec::new();

    parse_from_file_page(&fragment, &mut file_tags, &mut file_url);

    if let Some(file_url) = file_url {
        file.insert(sharedtypes::FileObject {
            source: Some(sharedtypes::FileSource::Url(file_url.into())),
            hash: sharedtypes::HashesSupported::None,
            tag_list: file_tags,
            skip_if: vec![],
        });
    }

    if file_url.is_none() && file.is_empty() && tag.is_empty() {
        Err(sharedtypes::ScraperReturn::Nothing)
    } else {
        Ok(sharedtypes::ScraperObject {
            file,
            tag,
            flag: vec![],
        })
    }
}
