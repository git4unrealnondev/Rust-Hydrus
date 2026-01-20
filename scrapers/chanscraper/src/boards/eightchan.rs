use crate::{Site, sharedtypes};
use base64::Engine;
use json::JsonValue;
use rand::distributions::{Alphanumeric, DistString};
use std::str::FromStr;
use strum_macros::EnumString; // 0.8use

const BASEURL: &str = "https://8chan.moe/";

pub enum Holder {
    Base,
}

impl Site for Holder {
    fn gen_fileurl(&self, boardcode: String, filename: String, fileext: String) -> String {
        format!("{BASEURL}.media/{filename}.{fileext}")
    }
    fn filter_board(&self, inp: &str, params_cnt: &usize) -> Option<String> {
        if params_cnt == &0 {
            return Some(inp.to_lowercase());
        }
        None
    }
    fn gen_catalog(&self, boardcode: &str) -> String {
        let catalog_end = "/catalog.json";

        format!("{BASEURL}{boardcode}{catalog_end}")
    }
    fn gen_thread(&self, boardcode: &str, thread_number: &str) -> String {
        format!("{BASEURL}/{}/res/{}.json", boardcode, thread_number)
    }
    fn json_getfiles(&self, inp: &JsonValue, boardcode: &str) -> Option<Vec<crate::ChanFile>> {
        dbg!(inp);
        if let Some(name) = inp["tim"].as_usize() {
            let mut out = Vec::new();
            let attachment_md5 = hex::encode(
                base64::prelude::BASE64_STANDARD
                    .decode(inp["md5"].as_str().unwrap())
                    .unwrap(),
            );

            let hash = sharedtypes::HashesSupported::None;

            out.push(crate::ChanFile {
                attachment_name: name.to_string(),
                attachment_filename: Some(inp["filename"].as_str().unwrap().to_string()),
                attachment_hash: hash,
                attachment_hash_string: Some(attachment_md5),
                attachment_ext: inp["ext"].as_str().unwrap().to_string(),
            });

            Some(out)
        } else {
            None
        }
    }
    fn site_get(&self) -> &str {
        "8chan"
    }
}
