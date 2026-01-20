use crate::{Site, sharedtypes};
use base64::Engine;
use json::JsonValue;
use rand::distributions::{Alphanumeric, DistString};
use std::str::FromStr;
use strum_macros::EnumString; // 0.8use

impl Site for BoardCodes {
    fn gen_fileurl(&self, boardcode: String, filename: String, fileext: String) -> String {
        match BoardCodes::from_str(&boardcode.to_uppercase()).unwrap() {
            BoardCodes::B => {
                format!("https://i.4cdn.org/{}/{}{}", boardcode, filename, fileext)
            }
            _ => format!(
                "https://i.4cdn.org/{}/{}{}?{}={}",
                boardcode,
                filename,
                fileext,
                Alphanumeric.sample_string(&mut rand::thread_rng(), 32),
                Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
            ),
        }
    }
    fn filter_board(&self, inp: &str, _params_cnt: &usize) -> Option<String> {
        if let Ok(_) = BoardCodes::from_str(&inp.to_uppercase()) {
            Some(inp.to_lowercase())
        } else {
            None
        }
    }
    fn gen_catalog(&self, boardcode: &str) -> String {
        let catalog_base = "https://a.4cdn.org/";
        let catalog_end = "/catalog.json";

        format!("{catalog_base}{boardcode}{catalog_end}")
    }
    fn gen_thread(&self, boardcode: &str, thread_number: &str) -> String {
        format!(
            "https://a.4cdn.org/{}/thread/{}.json",
            boardcode, thread_number
        )
    }
    fn json_getfiles(&self, inp: &JsonValue, boardcode: &str) -> Option<Vec<crate::ChanFile>> {
        if let Some(name) = inp["tim"].as_usize() {
            let mut out = Vec::new();
            let attachment_md5 = hex::encode(
                base64::prelude::BASE64_STANDARD
                    .decode(inp["md5"].as_str().unwrap())
                    .unwrap(),
            );

            let hash = match BoardCodes::from_str(&boardcode.to_uppercase()).unwrap() {
                BoardCodes::B => sharedtypes::HashesSupported::None,
                _ => sharedtypes::HashesSupported::Md5(attachment_md5.clone()),
            };
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
        "4chan"
    }
}

///
/// List of all board codes.
/// Only exepction is /3/ because of enum restrictions
///
#[derive(Debug, PartialEq, Eq, EnumString)]
pub enum BoardCodes {
    THREE,
    A,
    ACO,
    ADV,
    AN,
    B,
    BANT,
    BIZ,
    C,
    CGL,
    CK,
    CO,
    D,
    DIY,
    E,
    F,
    FA,
    FIT,
    G,
    GD,
    GIF,
    H,
    HC,
    HIS,
    HM,
    HR,
    I,
    IC,
    INT,
    J,
    JP,
    K,
    LGBT,
    LIT,
    M,
    MLP,
    MU,
    N,
    NEWS,
    O,
    OUT,
    P,
    POL,
    PW,
    QST,
    R,
    R9K,
    S,
    S4S,
    SCI,
    SOC,
    SP,
    T,
    TEST,
    TG,
    TOY,
    TRASH,
    TRV,
    TV,
    U,
    V,
    VG,
    VIP,
    VM,
    VMG,
    VP,
    VR,
    VRPG,
    VST,
    VT,
    W,
    WG,
    WSG,
    X,
    XS,
    Y,
}
///
/// Quick and dirty hack for getting the board code as a String
///
impl std::fmt::Display for BoardCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self == &BoardCodes::THREE {
            write!(f, "3")
        } else {
            write!(f, "{}", format!("{:?}", self).to_lowercase())
        }
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}
