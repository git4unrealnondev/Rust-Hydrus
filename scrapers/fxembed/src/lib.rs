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
pub const DEFAULT_REPLACEMENT_NAME_BSKY: &str = "fxbsky.app";

// Regex to match a url when a tag comes in
pub const REGEX_COLLECTIONS: &str =
    r"(http(s)?://)?(fxtwitter|skibidix|fixupx|fxbsky|x)[.a-zA-Z///_0-9]+";

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

const SOURCE_REPLACEMENTS_TWTR: [&str; 3] = ["fxtwitter", "skibidix", "fixupx"];

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
    _params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = vec![];

    if scraperdata.job.param.len() == 1 {
        for param in scraperdata.job.param.iter() {
            if let sharedtypes::ScraperParam::Normal(temp) = param {
                // Replaces any x or bsky links with the alternatives from fxembed
                let url = temp
                    .replace("x.com", DEFAULT_REPLACEMENT_NAME_TWIT)
                    .replace("/twitter.com", DEFAULT_REPLACEMENT_NAME_TWIT)
                    .replace("/bsky.app", DEFAULT_REPLACEMENT_NAME_BSKY);

                let mut stripped = temp.clone();
                for each in SOURCE_REPLACEMENTS_TWTR {
                    stripped = stripped.replace(each, "x");
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
    html_input: &String,
    _params: &Vec<sharedtypes::ScraperParam>,
    actual_params: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut out = sharedtypes::ScraperObject {
        file: HashSet::new(),
        tag: HashSet::new(),
        flag: vec![],
    };

    if DEBUG {
        dbg!(&html_input, actual_params);
    }
    let fragment = Html::parse_fragment(html_input);
    let selector = Selector::parse("meta").unwrap();
    let url_source_ns = sharedtypes::GenericNamespaceObj {
        name: "FxEmbed_Source_Url".into(),
        description: Some("The original source of a post for something".into()),
    };

    let mut urlsource = None;

    let postsource = match actual_params.user_data.get("post_source") {
        Some(out) => out.to_string(),
        None => return Ok(out),
    };

    let postsourceedited = match actual_params.user_data.get("post_source_edited") {
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
                if postsource.contains("bsky.app") {
                    urlinp = postsource.clone();
                } else {
                    urlinp = val.into();
                }
                urlsource = Some(sharedtypes::SubTag {
                    namespace: url_source_ns.clone(),
                    tag: urlinp,
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
                        indivimg = postsourceedited.clone();
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
                            tag: postsource.clone(),
                            namespace: sourceurl.namespace.clone(),
                        }),
                    });

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
                        skip_if: vec![sharedtypes::SkipIf::DownloadedFileExtension((
                            "html".into(),
                            true,
                        ))],
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
