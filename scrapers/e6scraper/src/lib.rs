use std::collections::HashMap;
use std::io;
use std::io::BufRead;
use std::time::Duration;

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
            _ratelimit: (1, Duration::from_secs(2)),
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
fn build_url(params: &Vec<sharedtypes::ScraperParam>, pagenum: u64) -> String {
    let url_base = "https://e621.net/posts.json".to_string();
    let tag_store = "&tags=";
    let page = "&page=";
    let mut param_tags_string: String = "".to_string();
    let mut params_normal: Vec<String> = Vec::new();
    let mut params_database: Vec<String> = Vec::new();
    let mut params_normal_count: usize = 0;
    let mut params_database_count: usize = 0;

    if params.is_empty() {
        return "".to_string();
    }

    // Gets params into db.
    for each in params {
        match each.param_type {
            sharedtypes::ScraperParamType::Normal => {
                params_normal.push(each.param_data.to_string());
                params_normal_count += 1;
            }
            sharedtypes::ScraperParamType::Database => {
                params_database.push(each.param_data.to_string());
                params_database_count += 1;
            }
        }
    }

    // Catch for normal tags being lower then 0
    match params_normal_count {
        0 => return "".to_string(),
        _ => {}
    }

    // Catch for database tags being correct. "Sould be one"
    let param_finalize_string = match params_database_count {
        0 => "?tags=".to_string(),
        1 => params_database.pop().unwrap() + tag_store,
        _ => {
            panic!(
                "Scraper e6scraper: IS PANICING RECIEVED ONE TOO MANY SAUCY DB COUNTS : {:?} {:?}",
                params_database, params_normal
            );
        }
    };

    // Gets last item in "normal" tags
    let params_last = params_normal.pop().unwrap();

    // Loops through all normal tags and inserts it into the tag string
    for each in params_normal {
        param_tags_string += &(each + "+")
    }
    
    // Adds on teh last string to the tags
    param_tags_string = param_tags_string + &params_last;

    // Does final formatting
    let url = url_base + &param_finalize_string + &param_tags_string + page + &pagenum.to_string();

    // Returns url
    return url.to_string();
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
pub fn url_get(params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
    vec![build_url(params, 1)]
}
///
/// Dumps a list of urls to scrape
///
#[no_mangle]
pub fn url_dump(params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
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
    tags_list: &mut HashMap<u64, sharedtypes::TagObject>,
    jso: &json::JsonValue,
    sub: &str,
    relates: Option<(String, String)>,
    tagtype: sharedtypes::TagType,
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
                        tag_type: tagtype,
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
                        tag_type: tagtype,
                    },
                );
                //dbg!(&tag_count, &tags_list[&tag_count]);
                *tag_count += 1;
            }
        }
    }
}

fn parse_pools(
    js: &json::JsonValue,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut files: HashMap<u64, sharedtypes::FileObject> = HashMap::default();
    let mut cnttotal = 0;    
    
    // For each in tag pools pulled.
    for multpool in js.members() {
        
        if multpool["id"].is_null() {
            continue;
        }

        let mut tag_count: u64 = 0;
        let mut tags_list: HashMap<u64, sharedtypes::TagObject> = HashMap::default();
        
        // Add poolid if not exist
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "pool_id".to_string(),
                relates_to: None,
                tag: multpool["id"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
        tag_count += 1;

        // Add pool creator
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "pool_creator".to_string(),
                relates_to: Some(("pool_id".to_string(), multpool["id"].to_string())),
                tag: multpool["creator_name"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
        tag_count += 1;

        // Add pool creator id
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "pool_creator_id".to_string(),
                relates_to: Some((
                    "pool_creator".to_string(),
                    multpool["creator_name"].to_string(),
                )),
                tag: multpool["creator_id"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
        tag_count += 1;

        // Add pool name
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "pool_name".to_string(),
                relates_to: Some(("pool_id".to_string(), multpool["id"].to_string())),
                tag: multpool["name"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
        tag_count += 1;

        // Add pool description
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "pool_description".to_string(),
                relates_to: Some(("pool_id".to_string(), multpool["id"].to_string())),
                tag: multpool["description"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
        tag_count += 1;

        let mut cnt = 0;
        //dbg!(&multpool);
         //dbg!(&multpool.entries());
        //dbg!(&multpool["pool_ids"]);
        for postids in multpool["post_ids"].members() {
            dbg!(&postids);

            
            
            // Relates the file id to pool
            tags_list.insert(
                tag_count,
                sharedtypes::TagObject {
                    namespace: "pool_id".to_string(),
                    relates_to: Some(("id".to_string(), postids.to_string())),
                    tag: multpool["id"].to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                },
            );
            tag_count += 1;
            // TODO need to fix the pool positing. Needs to relate the pool position with the ID better.
            tags_list.insert(
                tag_count,
                sharedtypes::TagObject {
                    namespace: "pool_position".to_string(),
                    relates_to: Some(("id".to_string(), postids.to_string())),
                    tag: cnt.to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                },
            );
            tag_count += 1;
            
            tags_list.insert(
                tag_count,
                sharedtypes::TagObject {
                    namespace: "pool_id".to_string(),
                    relates_to: Some(("pool_position".to_string(), cnt.to_string())),
                    tag: multpool["id"].to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                },
            );
            tag_count += 1;

            cnt += 1;
        }
        files.insert(cnttotal, sharedtypes::FileObject{source_url: None, hash: None, tag_list: tags_list});
    }
    
    
    

    Ok(sharedtypes::ScraperObject { file: files })
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(params: &String) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    //let vecvecstr: AHashMap<String, AHashMap<String, Vec<String>>> = AHashMap::new();

    let mut files: HashMap<u64, sharedtypes::FileObject> = HashMap::default();
    if let Err(_) = json::parse(params) {
        if params.contains("Please confirm you are not a robot.") {
            return Err(sharedtypes::ScraperReturn::Timeout(20));
        } else if params.contains("502: Bad gateway") {
            return Err(sharedtypes::ScraperReturn::Timeout(10));
        } else if params.contains("SSL handshake failed") {
            return Err(sharedtypes::ScraperReturn::Timeout(10));
        } else if params.contains("e621 Maintenance") {
            return Err(sharedtypes::ScraperReturn::Timeout(240));
        }
        dbg!(params);
        return Err(sharedtypes::ScraperReturn::EMCStop(
            "Unknown Error".to_string(),
        ));
    }
    let js = json::parse(params).unwrap();

    //let mut file = File::create("main1.json").unwrap();

    // Write a &str in the file (ignoring the result).
    //writeln!(&mut file, "{}", js.to_string()).unwrap();
    println!("Parsing");
    if js["posts"].is_empty() & !js["posts"].is_null() {
        dbg!(js);
        return Err(sharedtypes::ScraperReturn::Nothing);
    } else if js["posts"].is_null() {
        println!("{}", &js);
        let pool = parse_pools(&js);
        dbg!(&pool);
        return pool;
        //panic!();
    }

    for inc in 0..js["posts"].len() {
        let mut tag_count: u64 = 0;

        let mut tags_list: HashMap<u64, sharedtypes::TagObject> = HashMap::default();
        //dbg!(&tag_count);
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "general",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "species",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "character",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "copyright",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "artist",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "lore",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["tags"],
            "meta",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc],
            "sources",
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc],
            "pool_id",
            Some(("id".to_string(), js["posts"][inc]["id"].to_string())),
            sharedtypes::TagType::Normal,
        );

        if !js["posts"][inc]["pools"].is_null() {
            for each in js["posts"][inc]["pools"].members() {
                tags_list.insert(
                    tag_count,
                    sharedtypes::TagObject {
                        namespace: "".to_string(),
                        relates_to: None,
                        tag: format!("https://e621.net/pools?format=json&search[id={}]", each),
                        tag_type: sharedtypes::TagType::ParseUrl,
                    },
                );
                tag_count += 1;
            }
        }

        json_sub_tag(
            &mut tag_count,
            &mut tags_list,
            &js["posts"][inc]["relationships"],
            "children",
            Some(("id".to_string(), js["posts"][inc]["id"].to_string())),
            sharedtypes::TagType::Normal,
        );
        if !js["posts"][inc]["description"].is_empty() {
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
        tag_count += 1;
        tags_list.insert(
            tag_count,
            sharedtypes::TagObject {
                namespace: "rating".to_string(),
                relates_to: None,
                tag: js["posts"][inc]["rating"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            },
        );
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
            json_sub_tag(
                &mut tag_count,
                &mut tags_list,
                &js["posts"][inc]["relationships"],
                "parent_id",
                Some(("id".to_string(), js["posts"][inc]["id"].to_string())),
                sharedtypes::TagType::Normal,
            );
        }

        let url = match js["posts"][inc]["file"]["url"].is_null() {
            false => js["posts"][inc]["file"]["url"].to_string(),
            true => {
                //let base = "https://static1.e621.net/data/1c/a6/1ca6868a2b0f5e7129d2b478198bfa91.webm";
                let base = "https://static1.e621.net/data";
                let md5 = js["posts"][inc]["file"]["md5"].to_string();
                let ext = js["posts"][inc]["file"]["ext"].to_string();
                dbg!(format!(
                    "{}/{}/{}/{}.{}",
                    base,
                    &md5[0..2],
                    &md5[2..4],
                    &md5,
                    &ext
                ));
                format!("{}/{}/{}/{}.{}", base, &md5[0..2], &md5[2..4], &md5, ext)
            }
        };
        let file: sharedtypes::FileObject = sharedtypes::FileObject {
            source_url: Some(url),
            hash: Some(sharedtypes::HashesSupported::Md5(
                js["posts"][inc]["file"]["md5"].to_string(),
            )),
            tag_list: tags_list,
        };
        files.insert(inc.try_into().unwrap(), file);
    }
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
