use chrono::{DateTime, Utc};
use json::JsonValue;
use regex::Regex;
use std::{collections::HashSet, time::Duration};

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

pub const LOCAL_NAME: &str = "FxEmbed";
pub const REGEX_COLLECTIONS: &str = r"(http(s)?://)?(fxtwitter|skibidix|fixupx|fxbsky)[.a-zA-Z///_0-9]+";

use crate::sharedtypes::DEFAULT_PRIORITY;
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
                LOCAL_NAME.to_lowercase().into(),
            ],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![
                // Emulates discord when reaching out to fxembed service to pull info
                sharedtypes::ScraperModifiers::TextUseragent("Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)".to_string())
            ]
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

#[unsafe(no_mangle)]
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = vec![];


    dbg!(params, scraperdata);

    if scraperdata.job.param.len() == 1 {
    for param in scraperdata.job.param.iter() {
        if let sharedtypes::ScraperParam::Normal(url) = param {
        out.push((url.clone(), scraperdata.clone()));
    }}}
    
    out
}



#[unsafe(no_mangle)]
pub fn parser(
    html_input: &String,
    params: &Vec<sharedtypes::ScraperParam>,
    actual_params: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut out = sharedtypes::ScraperObject {
        file: HashSet::new(),
        tag: HashSet::new(),
        flag: vec![],
    };

    dbg!(html_input);
    
    Err(sharedtypes::ScraperReturn::Nothing)
}


