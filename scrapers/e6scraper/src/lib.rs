use ahash::AHashMap;
use json;
use nohash_hasher::NoHashHasher;
use rayon::prelude::*;
use std::io;
use std::io::BufRead;
use std::time::Duration;
use std::{collections::HashMap, hash::BuildHasherDefault};

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

impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0,
            _name: "e6scraper".to_string(),
            _sites: vec_of_strings!("e6", "e621", "e621.net"),
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
/// Builds the URL for scraping activities.
///
fn build_url(params: &Vec<String>, pagenum: u64) -> String {
    let url = "https://e621.net/posts.json";
    let tag_store = "&tags=";
    let page = "&page=";
    let formatted: String = "".to_string();

    if params.is_empty() {
        return "".to_string();
    } else {
        let endint = params.len() - 1;
        let endtwo = params.len() - 2;
        let end = &params[endint];
        let mut format_string = "".to_string();
        //for each in 0..endint {
        for (each, temp) in params.iter().enumerate().take(endint) {
            format_string += &params[each].replace(' ', "+");
            if each != endtwo {
                format_string += "+";
            }
        }
        return format!(
            "{}{}{}{}{}{}",
            url, params[endint], tag_store, format_string, page, pagenum
        );
    }

    if params.len() == 1 {
        formatted = format!("{}{}{}", &url, &tag_store, &params[0].replace(' ', "+"));
        return format!("{}{}{}", formatted, page, pagenum);
    }

    if params.len() == 2 {
        formatted = format!(
            "{}{}{}{}",
            &url,
            &params[1],
            &tag_store,
            &params[0].replace(' ', "+")
        );
    }
    format!("{}{}{}", formatted, page, pagenum)
}
///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[no_mangle]
pub fn new() -> InternalScraper {
    InternalScraper::new()
}
///
/// Returns one url from the parameters.
///
#[no_mangle]
pub fn url_get(params: &Vec<String>) -> Vec<String> {
    vec![build_url(params, 1)]
}
///
/// Dumps a list of urls to scrape
///
#[no_mangle]
pub fn url_dump(params: &Vec<String>) -> Vec<String> {
    dbg!(&params);
    let mut ret = Vec::new();
    let hardlimit = 751;
    for i in 1..hardlimit {
        let a = build_url(params, i);
        ret.push(a);
    }
    ret
}
///
/// Returns bool true or false if a cookie is needed. If so return the cookie name in storage
///
#[no_mangle]
pub fn cookie_needed() -> (sharedtypes::ScraperType, String) {
    println!("Enter E6 Username");
    let user = io::stdin().lock().lines().next().unwrap().unwrap();
    println!("Enter E6 API Key");
    let api = io::stdin().lock().lines().next().unwrap().unwrap();

    return (
        sharedtypes::ScraperType::Manual,
        format!("?login={}&api_key={}", user, api),
    );
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
/// New function that inserts a tag object into the tags_list. Increments the tag_count variable.
/// relates is an option that goes : (namespace: tag) OR None
/// relates searches by the second string in the members assuming it's set.
///
fn json_sub_tag(
    tag_count: &mut u64,
    tags_list: &mut HashMap<u64, sharedtypes::TagObject, BuildHasherDefault<NoHashHasher<u64>>>,
    jso: &json::JsonValue,
    sub: &str,
    relates: Option<(String, String)>,
) {
    match relates {
        None => {
            for each in jso[sub].members() {
                tags_list.insert(
                    *tag_count,
                    sharedtypes::TagObject {
                        namespace: sub.to_string(),
                        relates_to: None,
                        tag: each.to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                    },
                );
                //dbg!(&tag_count, sharedtypes::TagObject{namespace: sub.to_string(), relates_to: None, tag:each.to_string()});
                *tag_count += 1;
            }
        }
        Some(_) => {
            //let temp = relates.unwrap().1;
            for each in jso[sub].members() {
                let temp = relates.as_ref().unwrap();
                tags_list.insert(
                    *tag_count,
                    sharedtypes::TagObject {
                        namespace: sub.to_string(),
                        relates_to: Some(temp.clone()),
                        tag: each.to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                    },
                );
                //dbg!(&tag_count, &tags_list[&tag_count]);
                *tag_count += 1;
            }
        }
    }
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(params: &String) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    //let vecvecstr: AHashMap<String, AHashMap<String, Vec<String>>> = AHashMap::new();

    let mut files: HashMap<u64, sharedtypes::FileObject, BuildHasherDefault<NoHashHasher<u64>>> =
        HashMap::with_hasher(BuildHasherDefault::default());
    if let Err(_) = json::parse(params) {
        if params.contains("Please confirm you are not a robot.") {
            return Err(sharedtypes::ScraperReturn::Timeout(5))
        }
        dbg!(params);
        return Err(sharedtypes::ScraperReturn::EMCStop("Unknown Error".to_string()))
    }
    let js = json::parse(params).unwrap();

    //let mut file = File::create("main1.json").unwrap();

    // Write a &str in the file (ignoring the result).
    //writeln!(&mut file, "{}", js.to_string()).unwrap();

    if js["posts"].is_empty() {
        return Err(sharedtypes::ScraperReturn::Nothing)
    }

    for inc in 0..js["posts"].len() {
        let mut tag_count: u64 = 0;

        let mut tags_list: HashMap<
            u64,
            sharedtypes::TagObject,
            BuildHasherDefault<NoHashHasher<u64>>,
        > = HashMap::with_hasher(BuildHasherDefault::default());
        //dbg!(&tag_count);
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "general",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "species",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "character",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "copyright",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "artist",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "lore",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "meta",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc],
            "sources",
            None,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc],
            "pools",
            Some(("id".to_string(), js["posts"][inc]["id"].to_string())),
        );

        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["relationships"],
            "children",
            Some(("id".to_string(), js["posts"][inc]["id"].to_string())),
        );
        if js["posts"][inc]["description"].is_empty() {
            tags_list.insert(
                tag_count,
                sharedtypes::TagObject {
                    namespace: "description".to_string(),
                    relates_to: None,
                    tag: js["posts"][inc]["description"].to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                },
            );
            //dbg!(js["posts"][inc]["description"].to_string());
            tag_count += 1;
        }

        /*tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "md5".to_string(),
                relates_to: None,
                tag: js["posts"][inc]["file"]["md5"].to_string(),
                tag_type: sharedtypes::TagType::Hash(sharedtypes::HashesSupported::md5),
            },
        );*/
        tag_count += 1;
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "id".to_string(),
                relates_to: None,
                tag: js["posts"][inc]["id"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
        tag_count += 1;
        if !js["posts"][inc]["relationships"]["parent_id"].is_null() {
            //dbg!(&js["posts"][inc]["relationships"]["parent_id"]);
            //dbg!(&js["posts"][inc]["relationships"]["parent_id"].to_string());
            tags_list.insert(
                tag_count,
                sharedtypes::TagObject {
                    namespace: "parent_id".to_string(),
                    relates_to: Some(("id".to_string(), js["posts"][inc]["id"].to_string())),
                    tag: js["posts"][inc]["relationships"]["parent_id"].to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                },
            );
        }

        let file: sharedtypes::FileObject = sharedtypes::FileObject {
            source_url: js["posts"][inc]["file"]["url"].to_string(),
            hash: sharedtypes::HashesSupported::Md5(js["posts"][inc]["file"]["md5"].to_string()),
            tag_list: tags_list,
        };

        files.insert(inc.try_into().unwrap(), file);

        /*//println!("{:?}", &tags_list);
        //dbg!();
        //dbg!(&tag_count);
        let mut vecstr: AHashMap<String, Vec<String>> = AHashMap::new();
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "general");
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "species");
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "character");
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "copyright");
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "artist");
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "lore");
        retvec(&mut vecstr, &js["posts"][inc]["tags"], "meta");
        retvec(&mut vecstr, &js["posts"][inc], "sources");
        retvec(&mut vecstr, &js["posts"][inc], "pools");
        retvec(&mut vecstr, &js["posts"][inc]["relationships"], "parent_id");
        retvec(&mut vecstr, &js["posts"][inc]["relationships"], "children");

        // Filtering for parents
        if js["posts"][inc]["relationships"]["parent_id"].to_string() != "null".to_string() {
            vecstr.insert(
                "parent_id".to_string(),
                [js["posts"][inc]["relationships"]["parent_id"].to_string()].to_vec(),
            );
        }
        if !&js["posts"][inc]["description"].is_empty() {
            vecstr.insert(
                "description".to_string(),
                [js["posts"][inc]["description"].to_string()].to_vec(),
            );
        }
        vecstr.insert(
            "md5".to_string(),
            [js["posts"][inc]["file"]["md5"].to_string()].to_vec(),
        );
        vecstr.insert(
            "id".to_string(),
            [js["posts"][inc]["id"].to_string()].to_vec(),
        );
        vecvecstr.insert(js["posts"][inc]["file"]["url"].to_string(), vecstr);*/
    }
    //panic!();
    Ok(sharedtypes::ScraperObject { file: files })
    //return Ok(vecvecstr);
}
///
/// Should this scraper handle anything relating to downloading.
///
#[no_mangle]
pub fn scraper_download_get() -> bool {
    false
}
