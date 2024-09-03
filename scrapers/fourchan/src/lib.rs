use chrono::DateTime;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::io::BufRead;
use std::str::FromStr;
use std::time::Duration;

//use ahash::HashSet;
//use ahash::HashSet;

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

///
/// List of all board codes.
/// Only exepction is /3/ because of enum restrictions
///
#[derive(Debug, PartialEq, Eq)]
enum BoardCodes {
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
            write!(f, "{}", "3")
        } else {
            write!(f, "{}", format!("{:?}", self).to_lowercase())
        }
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}
impl FromStr for BoardCodes {
    type Err = ();

    fn from_str(input: &str) -> Result<BoardCodes, Self::Err> {
        let inp = input.to_uppercase();
        match inp.as_str() {
            "B" => Ok(BoardCodes::B),
            "3" => Ok(BoardCodes::THREE),
            _ => Err(()),
        }
    }
}

pub struct InternalScraper {
    _version: usize,
    _name: String,
    _sites: Vec<String>,
    _ratelimit: (u64, Duration),
    _type: sharedtypes::ScraperType,
}

impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0,
            _name: "4chan".to_string(),
            _sites: vec_of_strings!("4ch", "4chan", "4channel"),
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
    println!("This scraper pulls info from 4chan. I'm not affiliated with them lol");
    InternalScraper::new()
}
///
/// Returns one url from the parameters.
///
#[no_mangle]
pub fn url_get(params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
    let mut out = Vec::new();

    dbg!("URL_GET", params);
    out
}
///
/// Dumps a list of urls to scrape
///
#[no_mangle]
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> (Vec<String>, sharedtypes::ScraperData) {
    if scraperdata.user_data.contains_key("Stop") {
        return (Vec::new(), scraperdata.clone());
    }

    let mut scraper_data = scraperdata.clone();
    //let mut out = Vec::new();
    dbg!("URL_DUMP", params);

    if let Some((board_codes, search_term)) = filter_boardcodes(&params) {
        for cnt in 0..board_codes.len() {
            scraper_data.user_data.insert(
                format!("key_board_{cnt}"),
                format!("{}", board_codes.get(cnt).unwrap()),
            );
            scraper_data.user_data.insert(
                format!("key_search_{cnt}"),
                format!("{}", search_term.get(cnt).unwrap()),
            );
        }
    }

    (gen_url_catalog(params), scraper_data)
}
fn filter_boardcodes(
    params: &Vec<sharedtypes::ScraperParam>,
) -> Option<(Vec<BoardCodes>, Vec<String>)> {
    let mut params_boardcodes = Vec::new();
    //let mut params_query = Vec::new();
    let mut params_storage = Vec::new();
    for each in params.iter() {
        match each.param_type {
            sharedtypes::ScraperParamType::Normal => {
                if let Ok(boardcode) = BoardCodes::from_str(&each.param_data) {
                    params_boardcodes.push(boardcode);
                } else {
                    params_storage.push(each.param_data.to_string());
                }
            }
            _ => {}
        }
    }

    if params_boardcodes.len() == params_storage.len() {
        return Some((params_boardcodes, params_storage));
    } else {
        None
    }
}

///
/// Generates a catalog url
///
fn gen_url_catalog(params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
    let catalog_base = "https://a.4cdn.org/";
    let catalog_end = "/catalog.json";
    let mut params = params.clone();
    params.pop();

    let mut out = Vec::new();
    if let Some((params_boardcodes, _)) = filter_boardcodes(&params) {
        for each in params_boardcodes.iter() {
            out.push(format!("{catalog_base}{}{catalog_end}", each));
        }
    }

    dbg!(&out);

    out
}

///
/// Returns bool true or false if a cookie is needed. If so return the cookie name in storage
///
#[no_mangle]
pub fn cookie_needed() -> (sharedtypes::ScraperType, String) {
    return (sharedtypes::ScraperType::Manual, format!(""));
}
///
/// Gets url to query cookie url.
/// Not used or implemented corrently. :D
///
#[no_mangle]
pub fn cookie_url() -> String {
    "e6scraper_cookie".to_string()
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(
    params: &String,
    actual_params: &sharedtypes::ScraperData,
) -> Result<(sharedtypes::ScraperObject, sharedtypes::ScraperData), sharedtypes::ScraperReturn> {
    let mut scraper_data = actual_params.clone();
    let mut out = sharedtypes::ScraperObject {
        file: HashSet::new(),
        tag: HashSet::new(),
    };

    dbg!("PARSER", actual_params);
    if let Ok(chjson) = json::parse(params) {
        for each in chjson.members() {
            for thread in each["threads"].members() {
                let mut cnt = 0;
                while let Some(key) = scraper_data.user_data.get(&format!("key_search_{cnt}")) {
                    if thread["com"].to_string().contains(key) {
                        scraper_data
                            .user_data
                            .insert("Stop".to_string(), "Stop".to_string());
                        //dbg!(&thread["com"]);
                        let threadurl = format!(
                            "https://a.4cdn.org/b/thread/{}.json",
                            thread["no"].to_string()
                        );
                        if !scraper_data.user_data.contains_key("Stop") {
                            out.tag.insert(sharedtypes::TagObject {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "DO NOT ADD".to_string(),
                                    description: Some("DO NOT PARSE".to_string()),
                                },
                                tag: threadurl.clone(),
                                tag_type: sharedtypes::TagType::ParseUrl((
                                    sharedtypes::ScraperData {
                                        job: sharedtypes::JobScraper {
                                            site: "4ch".to_string(),
                                            param: Vec::new(),
                                            original_param: threadurl,
                                            job_type: sharedtypes::DbJobType::Scraper,
                                        },
                                        system_data: scraper_data.system_data.clone(),
                                        user_data: scraper_data.user_data.clone(),
                                    },
                                    sharedtypes::SkipIf::None,
                                )),
                                relates_to: None,
                            });
                        }
                    }

                    cnt += 1;
                }
                //dbg!(thread);
            }
        }
    }
    dbg!(&out, &scraper_data);
    Ok((out, scraper_data))
}
///
/// Should this scraper handle anything relating to downloading.
///
#[no_mangle]
pub fn scraper_download_get() -> bool {
    false
}
