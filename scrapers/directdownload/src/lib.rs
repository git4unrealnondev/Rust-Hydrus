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

//use ahash::HashSet;
//use ahash::HashSet;

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub struct InternalScraper {
    _version: usize,
    _name: String,
    _sites: Vec<String>,
    _ratelimit: (u64, Duration),
    _type: sharedtypes::ScraperType,
}

#[no_mangle]
fn scraper_file_regen() -> sharedtypes::ScraperFileRegen {
    sharedtypes::ScraperFileRegen {
        hash: sharedtypes::HashesSupported::Md5("".to_string()),
    }
}

#[no_mangle]
fn scraper_file_return(inp: &sharedtypes::ScraperFileInput) -> sharedtypes::SubTag {
    todo!()
}

impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0,
            _name: "direct-download".to_string(),
            _sites: vec_of_strings!("direct-download"),
            _ratelimit: (1, Duration::from_secs(1)),
            _type: sharedtypes::ScraperType::Automatic,
        }
    }
    pub fn version_get(&self) -> usize {
        self._version
    }
    pub fn name_get(&self) -> &String {
        &self._name
    }
    pub fn name_put(&mut self, inp: String) {
        self._name = inp;
    }
    pub fn sites_get(&self) -> Vec<String> {
        println!("AHAGAFAD");
        let mut vecs: Vec<String> = Vec::new();
        for each in &self._sites {
            vecs.push(each.to_string());
        }
        vecs
    }
}

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[no_mangle]
pub fn new() -> InternalScraper {
    InternalScraper::new()
}
#[no_mangle]
pub fn test() -> u8 {
    0
}

///
/// Returns one url from the parameters.
///
#[no_mangle]
pub fn url_get(params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
    todo!()
}

///
/// Dumps a list of urls to scrape
///
#[no_mangle]
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut ret = Vec::new();
    dbg!(params, scraperdata);
    ret.push((scraperdata.job.original_param.clone(), ))
    ret
}
///
/// Returns bool true or false if a cookie is needed. If so return the cookie name in storage
///
#[no_mangle]
pub fn cookie_needed() -> (sharedtypes::ScraperType, String) {
    return (sharedtypes::ScraperType::Automatic, format!(""));
}
///
/// Gets url to query cookie url.
/// Not used or implemented corrently. :D
///
#[no_mangle]
pub fn cookie_url() -> String {
    "".to_string()
}

pub enum UrlType {
    Models((usize, Option<usize>)),
    Images(usize),
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(
    inputfromreqwest: &String,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<(sharedtypes::ScraperObject, sharedtypes::ScraperData), sharedtypes::ScraperReturn> {
    //let vecvecstr: AHashMap<String, AHashMap<String, Vec<String>>> = AHashMap::new();
    if !scraperdata.user_data.contains_key("task") {
        return Err(sharedtypes::ScraperReturn::Nothing);
    }
    let mut files: HashSet<sharedtypes::FileObject> = HashSet::default();
    let mut tags: HashSet<sharedtypes::TagObject> = HashSet::default();

    dbg!(inputfromreqwest, scraperdata);

    Ok((
        sharedtypes::ScraperObject {
            file: files,
            tag: tags,
        },
        scraperdata.clone(),
    ))
    //return Ok(vecvecstr);
}
///
/// Should this scraper handle anything relating to downloading.
///
#[no_mangle]
pub fn scraper_download_get() -> bool {
    false
}

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;

#[no_mangle]
pub fn db_upgrade_call(db_version: &usize) {
    match db_version {
        _ => {
            client::log_no_print(format!("Civitai No upgrade for version: {}", db_version));
        }
    }
}
