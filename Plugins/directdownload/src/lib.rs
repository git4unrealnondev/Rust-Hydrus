use base64::Engine;
use chrono::DateTime;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::io::BufRead;
use std::time::Duration;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

static PLUGIN_NAME: &str = "File Downloader";
static PLUGIN_DESCRIPTION: &str =
    "Tries to download files directly if this plugin can recognize a url.";

#[no_mangle]
pub fn return_info() -> sharedtypes::PluginInfo {
    let  tag_vec = vec![   (     Some(sharedtypes::SearchType::Regex(
            //r"(http(s)?://www.|((www.|http(s)?://)))[a-zA-Z0-9-].[a-zA-Z0-9-_.]*/[a-zA-Z0-9/_%]+\.[a-zA-Z0-9/_%\.?=&-]+"
            r"(http(s)?://www.|((www.|http(s)?://)))[a-zA-Z0-9-].[a-zA-Z0-9-_.]*/[a-zA-Z0-9/_%-]+\.[a-zA-Z0-9/_%\.?=&-]+"
            .to_string()),
        ),
        None,
        Some("source_url".to_string()),
    )];

    let callbackvec = vec![sharedtypes::PluginCallback::Tag(tag_vec)];
    sharedtypes::PluginInfo {
        name: PLUGIN_NAME.to_string(),
        description: PLUGIN_DESCRIPTION.to_string(),
        version: 1.00,
        api_version: 1.00,
        callbacks: callbackvec,
        communication: Some(sharedtypes::PluginSharedData {
            thread: sharedtypes::PluginThreadType::Inline,
            com_channel: Some(sharedtypes::PluginCommunicationChannel::Pipe(
                "beans".to_string(),
            )),
        }),
    }
}

#[no_mangle]
pub fn on_regex_match(
    tag: &String,
    tag_ns: &String,
    regex_match: &String,
    callback: sharedtypes::PluginCallback,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut out = Vec::new();
    if regex_match.contains("bsky.app") {
        return out;
    }
    dbg!(tag, tag_ns);

    let subtag = sharedtypes::SubTag {
        namespace: sharedtypes::GenericNamespaceObj {
            name: tag_ns.to_string(),
            description: None,
        },
        tag: tag.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        limit_to: None,
    };

    let taginfo = sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "source_url".to_string(),
            description: None,
        },
        tag: regex_match.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: Some(subtag),
    };

    let earlyexistag = sharedtypes::Tag {
        tag: regex_match.to_string(),
        namespace: sharedtypes::GenericNamespaceObj {
            name: "source_url".to_string(),
            description: None,
        },
    };

    let file = sharedtypes::FileObject {
        source_url: Some(regex_match.to_string()),
        hash: sharedtypes::HashesSupported::None,
        tag_list: vec![taginfo],
        skip_if: vec![sharedtypes::SkipIf::FileTagRelationship(earlyexistag)],
    };
    let ratelimit = (1, Duration::from_secs(1));

    out.push(sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: None,
            setting: None,
            relationship: None,
            parents: None,
            jobs: Some(vec![sharedtypes::DbJobsObj {
                id: 0,
                time: 0,
                reptime: Some(0),

                site: "direct download".to_string(),
                param: vec![sharedtypes::ScraperParam::Url(regex_match.to_string())],
                jobmanager: sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::FileUrl,
                    recreation: None,
                },
                committype: Some(sharedtypes::CommitType::StopOnNothing),
                isrunning: false,
                system_data: BTreeMap::new(),
                user_data: BTreeMap::new(),
            }]),
            namespace: None,
            file: None,
        },
    ]));

    /*client::job_add(
        None,
        0,
        0,
        "direct download".to_string(),
        regex_match.to_string(),
        true,
        sharedtypes::CommitType::StopOnNothing,
        sharedtypes::DbJobType::FileUrl,
        BTreeMap::new(),
        BTreeMap::new(),
        sharedtypes::DbJobsManager {
            jobtype: sharedtypes::DbJobType::FileUrl,
            recreation: None,
        },
    );*/
    out
    //client::add_file_nonblocking(file, ratelimit);
}
