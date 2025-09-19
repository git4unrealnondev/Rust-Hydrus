//use chrono::{DateTime, Utc};
//use json::JsonValue;
use scraper::{Html, Selector};
use std::{collections::HashSet, time::Duration};
use url::Url;

use crate::sharedtypes::DEFAULT_PRIORITY;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

pub const LOCAL_NAME: &str = "FxEmbed";

// Replacment url to put everything on. IE x.com to fxtwitter.com
pub const DEFAULT_REPLACEMENT_NAME_TWIT: &str = "fxtwitter.com";
pub const DEFAULT_REPLACEMENT_NAME_BSKY: &str = "fxbsky.app";

pub const DEFAULT_SOURCE_TWIT: &str = "x.com";
pub const DEFAULT_SOURCE_BSKY: &str = "bsky.app";

// Regex to match a url when a tag comes in
pub const REGEX_COLLECTIONS: &str = r"(http(s)?://)?(fxtwitter|skibidix|fixupx|fxbsky|x|xbsky|bsky|t)\.(com|app|co)[.a-zA-Z///_0-9]+";

// Sources for a url to attach data to
const SOURCE_FILTER_ARRAY: [(&str, &str, Option<&str>); 1] =
    [("og:url", "FxEmbed_Url", Some("FxEmbed source url"))];

// Sources for an image
const IMAGE_FILTER_ARRAY: [(&str, &str, Option<&str>); 1] =
    [("refresh", "FxEmbed_ImageSource", None)];

// Items that can be extracted from the returned fxembed data
const FILTER_ARRAY: [(&str, &str, Option<&str>); 3] = [
    (
        "twitter:creator",
        "FxEmbed_Twitter_Creator",
        Some("FxEmbed post creator"),
    ),
    (
        "twitter:title",
        "FxEmbed_Twitter_Title",
        Some("FxEmbed twitter post title"),
    ),
    (
        "og:description",
        "FxEmbed_Description",
        Some("FxEmbed description for a post"),
    ),
];

const SOURCE_REPLACEMENTS_TWTR: [&str; 5] = [
    "fxtwitter.com",
    "skibidix.com",
    "fixupx.com",
    "fixvx.com",
    "cunnyx.com",
];
const SOURCE_REPLACEMENTS_BSKY: [&str; 2] = ["fxbsky.app", "bskyx.app"];

const DEBUG: bool = false;

///
/// Returns the info needed to scrape an fxembed system
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut defaultscraper = sharedtypes::return_default_globalpluginparser();

    defaultscraper.name = LOCAL_NAME.into();
    defaultscraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec![
                "fxtwitter.com".into(),
                "fixupx.com".into(),
                "fxbsky.app".into(),
                "x.com".into(),
                "bsky.app".into(),
                LOCAL_NAME.into(),
                LOCAL_NAME.to_lowercase(),
            ],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![
                // Emulates discord when reaching out to fxembed service to pull info
                sharedtypes::ScraperModifiers::TextUseragent(
                    "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)".to_string(),
                ),
                sharedtypes::ScraperModifiers::MediaUseragent(
                    "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)".to_string(),
                ),
            ],
        },
    ));

    let tag_1 = (
        Some(sharedtypes::SearchType::Regex(REGEX_COLLECTIONS.into())),
        vec![],
        vec!["source_url".to_string(), "FxEmbed_SourceUrl".into()],
    );

    let callbackvec = vec![
        sharedtypes::GlobalCallbacks::Tag(tag_1),
        sharedtypes::GlobalCallbacks::Start(sharedtypes::StartupThreadType::SpawnInline),
    ];

    let mut plugin = sharedtypes::return_default_globalpluginparser();
    plugin.name = format!("{} Regex Parser", LOCAL_NAME);
    plugin.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_channel: true,
            redirect: Some(LOCAL_NAME.into()),
        },
    ));
    plugin.callbacks = callbackvec;

    vec![defaultscraper, plugin]
}

///
/// Standardizes the urls that we get or send
/// Will return an "alternative source" for the x or bsky links
///
fn fix_urls(parsed_url: &Url, original_url: &str) -> (String, String) {
    let mut url = original_url.to_string();

    let mut stripped = original_url.to_string();
    if let Some(host) = parsed_url.host_str() {
        if host == DEFAULT_SOURCE_TWIT || host == DEFAULT_SOURCE_BSKY {
            if host == DEFAULT_SOURCE_TWIT {
                stripped = stripped.replace(host, DEFAULT_SOURCE_TWIT);
                url = url.replace(host, DEFAULT_REPLACEMENT_NAME_TWIT);
            } else {
                stripped = stripped.replace(host, DEFAULT_SOURCE_BSKY);
                url = url.replace(host, DEFAULT_REPLACEMENT_NAME_BSKY);
            }
        } else {
            for each in SOURCE_REPLACEMENTS_TWTR {
                stripped = stripped.replace(each, DEFAULT_SOURCE_TWIT);
                url = url.replace(each, DEFAULT_REPLACEMENT_NAME_TWIT);
            }
            for each in SOURCE_REPLACEMENTS_BSKY {
                stripped = stripped.replace(each, DEFAULT_SOURCE_BSKY);
                url = url.replace(each, DEFAULT_REPLACEMENT_NAME_BSKY);
            }
        }
    }
    (stripped, url)
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

    if scraperdata.job.param.len() == 1 {
        for param in scraperdata.job.param.iter() {
            if let sharedtypes::ScraperParam::Normal(temp) = param {
                // Replaces any x or bsky links with the alternatives from fxembed
                let mut url = temp.clone();

                let mut stripped = temp.clone();
                if let Ok(parsed_url) = url::Url::parse(temp) {
                    (stripped, url) = fix_urls(&parsed_url, temp);
                }

                if DEBUG {
                    dbg!(&url, &stripped, &temp);
                }

                let mut scraperdata = scraperdata.clone();
                scraperdata.user_data.insert("post_source".into(), stripped);
                scraperdata
                    .user_data
                    .insert("post_source_edited".into(), url.clone());
                out.push((url.clone(), scraperdata.clone()));
            }
        }
    }

    out
}

///
/// After the text download step. Parses the response from the text download
///
#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    source_url: &str,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut out = sharedtypes::ScraperObject {
        file: HashSet::new(),
        tag: HashSet::new(),
        flag: vec![],
    };

    if DEBUG {
        dbg!(&html_input, scraperdata, source_url);
    }
    let fragment = Html::parse_fragment(html_input);
    let selector = Selector::parse("meta").unwrap();
    let url_source_ns = sharedtypes::GenericNamespaceObj {
        name: "FxEmbed_SourceUrl".into(),
        description: Some("The original source of a post for something".into()),
    };

    let mut urlsource = None;

    let source_urledited = match scraperdata.user_data.get("post_source_edited") {
        Some(out) => out.to_string(),
        None => return Ok(out),
    };

    let source_url = match scraperdata.user_data.get("post_source") {
        Some(out) => out.to_string(),
        None => return Ok(out),
    };

    // Filters for the source urls
    for element in fragment.select(&selector) {
        // Just checking that we're recieving 2 elements in our meta tag
        let attlen = element.value().attrs.len();
        if attlen != 2 {
            continue;
        }

        let mut attrs = element.value().attrs();
        let (_, val) = attrs.next().unwrap();
        let (_, key) = attrs.next().unwrap();

        'sourceloop: for (mat, _ns_name, _ns_description) in SOURCE_FILTER_ARRAY {
            if mat == key {
                let urlinp;
                if source_url.contains("bsky.app") {
                    urlinp = source_url.clone();
                } else {
                    urlinp = val.to_string();
                }
                urlsource = Some(sharedtypes::SubTag {
                    namespace: url_source_ns.clone(),
                    tag: urlinp.to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                    limit_to: None,
                });
                break 'sourceloop;
            }
        }
    }

    // Everything else for filtering
    for element in fragment.select(&selector) {
        // Just checking that we're recieving 2 elements in our meta tag
        let attlen = element.value().attrs.len();
        if attlen != 2 {
            continue;
        }

        let mut attrs = element.value().attrs();
        let (_, val) = attrs.next().unwrap();
        let (_, key) = attrs.next().unwrap();

        for (mat, ns_name, ns_description) in FILTER_ARRAY {
            if mat == key {
                let description = ns_description.map(|str| str.to_string());

                out.tag.insert(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: ns_name.into(),
                        description,
                    },
                    tag: val.into(),
                    tag_type: sharedtypes::TagType::Normal,
                    relates_to: urlsource.clone(),
                });
            }
        }
        for (mat, _ns_name, _ns_description) in IMAGE_FILTER_ARRAY {
            if mat == key {
                // Should get 2 items from the sources. a position and a source post
                let arrayvec: Vec<&str> = val.split(';').collect();
                if !arrayvec.len() == 2 {
                    continue;
                }

                // Extracts the position number of the images inside of the source
                let pos: usize = match arrayvec[0].parse() {
                    Err(_) => {
                        continue;
                    }
                    Ok(out) => out,
                };

                //pos += 1;

                if let Some(sourceurl) = &urlsource {
                    let indivimg_vec: Vec<&str> = arrayvec[1].split("url=").collect();

                    if indivimg_vec.len() != 2 {
                        continue;
                    }

                    if DEBUG {
                        dbg!(&indivimg_vec);
                    }

                    let mut indivimg;
                    if indivimg_vec[1].contains("x.com") {
                        indivimg = indivimg_vec[1]
                            .replace("x.com", &format!("d.{}", DEFAULT_REPLACEMENT_NAME_TWIT));
                        indivimg.push_str(&format!("/photo/{}", pos));
                    } else if indivimg_vec[1].contains("bsky.app") {
                        indivimg = source_urledited.clone();
                        indivimg = indivimg.replace(
                            "fxbsky.app",
                            &format!("d.{}", DEFAULT_REPLACEMENT_NAME_BSKY),
                        );
                        //indivimg = "https://d.fxbsky.app/profile/kadlifal.bsky.social/post/3ljzvg5xek22g".into();
                    } else {
                        continue;
                    }

                    let position_sub = Some(sharedtypes::SubTag {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "FxEmbed_Position".into(),
                            description: Some("The position of an item inside of a post".into()),
                        }
                        .clone(),
                        tag: pos.to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                        limit_to: Some(sharedtypes::Tag {
                            tag: source_url.to_string(),
                            namespace: sourceurl.namespace.clone(),
                        }),
                    });

                    let pos_tag = sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "source_url".into(),
                            description: None,
                        },
                        tag: indivimg.to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: position_sub,
                    };

                    out.file.insert(sharedtypes::FileObject {
                        source: Some(sharedtypes::FileSource::Url(indivimg)),
                        hash: sharedtypes::HashesSupported::None,
                        tag_list: vec![pos_tag],
                        skip_if: vec![],
                    });
                }
            }
        }
    }

    if DEBUG {
        dbg!(&out);
    }
    Ok(out)
}

///
/// Function runs when the system matches our regex
///
#[unsafe(no_mangle)]
pub fn on_regex_match(
    tag_name: &str,
    tag_namespace: &sharedtypes::GenericNamespaceObj,
    regex_match: &str,
    _plugin_callback: &Option<sharedtypes::SearchType>,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut job = sharedtypes::return_default_jobsobj();
    job.site = "fxembed".into();
    job.jobmanager = sharedtypes::DbJobsManager {
        jobtype: sharedtypes::DbJobType::Params,
        recreation: None,
    };
    let url_source_ns = sharedtypes::GenericNamespaceObj {
        name: "FxEmbed_SourceUrl".into(),
        description: Some("The original source of a post for something".into()),
    };

    let mut fixed_url;
    if let Ok(parsed_url) = url::Url::parse(tag_name) {
        (_, fixed_url) = fix_urls(&parsed_url, tag_name);
    } else {
        client::log(format!("FxEmbed could not parse the url for: {}", tag_name));
        return vec![];
    }

    fixed_url = fixed_url.replace(DEFAULT_REPLACEMENT_NAME_TWIT, DEFAULT_SOURCE_TWIT);
    fixed_url = fixed_url.replace(DEFAULT_REPLACEMENT_NAME_BSKY, DEFAULT_SOURCE_BSKY);

    job.param = vec![sharedtypes::ScraperParam::Normal(fixed_url.clone())];

    let tag = sharedtypes::TagObject {
        namespace: url_source_ns,
        tag: fixed_url.to_string(),
        tag_type: sharedtypes::TagType::NormalNoRegex,
        relates_to: Some(sharedtypes::SubTag {
            namespace: tag_namespace.clone(),
            tag: tag_name.to_string(),
            limit_to: None,
            tag_type: sharedtypes::TagType::NormalNoRegex,
        }),
    };

    if DEBUG {
        dbg!(&tag_name, tag_namespace, regex_match);
    }

    let out = vec![sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: vec![tag],
            setting: vec![],
            relationship: vec![],
            jobs: vec![job],
            file: vec![],
        },
    ])];

    out
}
