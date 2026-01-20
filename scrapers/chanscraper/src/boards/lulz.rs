use crate::{ChanFile, Site, sharedtypes};
use base64::Engine;
use json::JsonValue;
use strum_macros::EnumString; // 0.8use

impl Site for BoardCodes {
    fn site_get(&self) -> &str {
        "lulz.net"
    }
    fn gen_thread(&self, boardcode: &str, thread_number: &str) -> String {
        format!("https://lulz.net/{boardcode}/res/{thread_number}.json")
    }
    fn gen_fileurl(&self, boardcode: String, filename: String, fileext: String) -> String {
        //https://lulz.net/furi/src/1723579580299-1.png
        format!("https://lulz.net/{boardcode}/src/{filename}{fileext}")
    }
    fn gen_catalog(&self, boardcode: &str) -> String {
        format!("https://lulz.net/{boardcode}/catalog.json")
    }
    fn filter_board(&self, inp: &str, _params_cnt: &usize) -> Option<String> {
        if inp == "furi" {
            return Some("furi".to_string());
        }
        None
    }
    fn json_getfiles(&self, inp: &JsonValue, _: &str) -> Option<Vec<ChanFile>> {
        if inp["tim"].is_null() {
            return None;
        }
        let mut out = Vec::new();
        let attachment_md5 = hex::encode(
            base64::prelude::BASE64_STANDARD
                .decode(inp["md5"].as_str().unwrap())
                .unwrap(),
        );
        out.push(ChanFile {
            attachment_filename: Some(inp["filename"].as_str().unwrap().to_owned()),
            attachment_hash: sharedtypes::HashesSupported::Md5(attachment_md5.clone()),
            attachment_hash_string: Some(attachment_md5),
            attachment_ext: inp["ext"].as_str().unwrap().to_owned(),
            attachment_name: inp["tim"].as_str().unwrap().to_string(),
        });

        for member in inp["extra_files"].members() {
            let attachment_extra_md5 = hex::encode(
                base64::prelude::BASE64_STANDARD
                    .decode(member["md5"].as_str().unwrap())
                    .unwrap(),
            );

            out.push(ChanFile {
                attachment_filename: Some(member["filename"].as_str().unwrap().to_string()),
                attachment_hash: sharedtypes::HashesSupported::Md5(attachment_extra_md5.clone()),
                attachment_hash_string: Some(attachment_extra_md5),
                attachment_ext: member["ext"].as_str().unwrap().to_string(),
                attachment_name: member["tim"].as_str().unwrap().to_string(),
            })
        }

        //Some(inp["tim"].as_str().unwrap().to_string())

        Some(out)
    }
}

#[derive(Debug, PartialEq, Eq, EnumString)]
pub enum BoardCodes {
    FURI,
}
