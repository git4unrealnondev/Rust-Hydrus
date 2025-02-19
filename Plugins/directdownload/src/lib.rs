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
    let mut tag_vec = Vec::new();

    tag_vec.push((Some(sharedtypes::SearchType::Regex(r"[(http(s)?):(www\.)?a-zA-Z0-9@:%._\+~#=]{2,256}\.[a-z]{2,6}\b([-a-zA-Z0-9@:%_\+.~#?&//=]*)".to_string())), None));

    let callbackvec = vec![sharedtypes::PluginCallback::OnTag(tag_vec)];
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
