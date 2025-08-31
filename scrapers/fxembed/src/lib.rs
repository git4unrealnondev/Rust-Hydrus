//use chrono::{DateTime, Utc};
//use json::JsonValue;
use scraper::{Html, Selector};
use std::{collections::HashSet, time::Duration};

use crate::sharedtypes::DEFAULT_PRIORITY;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

pub const LOCAL_NAME: &str = "FxEmbed";

// Replacment url to put everything on. IE x.com to fxtwitter.com
pub const DEFAULT_REPLACEMENT_NAME_TWIT: &str = "fxtwitter.com";
pub const DEFAULT_REPLACEMENT_NAME_BSKY: &str = "fxbsky.com";

// Regex to match a url when a tag comes in
pub const REGEX_COLLECTIONS: &str =
    r"(http(s)?://)?(fxtwitter|skibidix|fixupx|fxbsky)[.a-zA-Z///_0-9]+";

// Sources for a url to attach data to
const SOURCE_FILTER_ARRAY: [(&str, &str, Option<&str>); 1] =
    [("og:url", "FxEmbed_Url", Some("FxEmbed source url"))];

// Sources for an image
const IMAGE_FILTER_ARRAY: [(&str, &str, Option<&str>); 1] =
    [("refresh", "FxEmbed_Image_Source", None)];

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
        vec!["source_url".to_string(), LOCAL_NAME.into()],
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

    vec![defaultscraper]
}

///
/// Takes the raw user input and outputs a url to download
///
#[unsafe(no_mangle)]
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = vec![];

    if scraperdata.job.param.len() == 1 {
        for param in scraperdata.job.param.iter() {
            if let sharedtypes::ScraperParam::Normal(url) = param {
                //let url:String  = url.replace("fx", "d.fx");
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
    html_input: &String,
    _params: &Vec<sharedtypes::ScraperParam>,
    _actual_params: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut out = sharedtypes::ScraperObject {
        file: HashSet::new(),
        tag: HashSet::new(),
        flag: vec![],
    };

    let fragment = Html::parse_fragment(html_input);
    let selector = Selector::parse("meta").unwrap();
    let url_source_ns = sharedtypes::GenericNamespaceObj {
        name: "FxEmbed_Source_Url".into(),
        description: Some("The original source of a post for something".into()),
    };

    let mut urlsource = None;

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
                urlsource = Some(sharedtypes::SubTag {
                    namespace: url_source_ns.clone(),
                    tag: val.into(),
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
                    let position_sub = Some(sharedtypes::SubTag {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "FxEmbed_Position".into(),
                            description: Some("The position of an item inside of a post".into()),
                        }
                        .clone(),
                        tag: pos.to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                        limit_to: Some(sharedtypes::Tag {
                            tag: sourceurl.tag.clone(),
                            namespace: sourceurl.namespace.clone(),
                        }),
                    });

                    let indivimg_vec: Vec<&str> = arrayvec[1].split("url=").collect();

                    if indivimg_vec.len() != 2 {
                        continue;
                    }

                    let mut indivimg;
                    if indivimg_vec[1].contains("x.com") {
                        indivimg = indivimg_vec[1]
                            .replace("x.com", &format!("d.{}", DEFAULT_REPLACEMENT_NAME_TWIT));
                    } else if indivimg_vec[1].contains("bsky.com") {
                        indivimg = indivimg_vec[1]
                            .replace("bsky.com", &format!("d.{}", DEFAULT_REPLACEMENT_NAME_BSKY));
                    } else {
                        continue;
                    }

                    indivimg.push_str(&format!("/photo/{}", pos));

                    let pos_tag = sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "source_url".into(),
                            description: None,
                        },
                        tag: indivimg.clone(),
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

    Ok(out)
}
