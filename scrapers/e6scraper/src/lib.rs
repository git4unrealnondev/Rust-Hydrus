use chrono::DateTime;
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

pub enum NsIdent {
    PoolCreatedAt,
    PoolCreator,
    PoolCreatorId,
    PoolDescription,
    PoolName,
    PoolUpdatedAt,
    PoolId,
    PoolPosition,
    FileId,
    Sources,
    Description,
    Parent,
    Children,
    Rating,
    Meta,
    Lore,
    Artist,
    Copyright,
    Character,
    Species,
    General,
}

#[no_mangle]
fn scraper_file_regen() -> sharedtypes::ScraperFileRegen {
    sharedtypes::ScraperFileRegen {
        hash: sharedtypes::HashesSupported::Md5("".to_string()),
    }
}

#[no_mangle]
fn scraper_file_return(inp: &sharedtypes::ScraperFileInput) -> sharedtypes::SubTag {
    let base = "https://static1.e621.net/data";
    let md5 = inp.hash.clone().unwrap();
    let ext = inp.ext.clone().unwrap();
    let url = format!("{}/{}/{}/{}.{}", base, &md5[0..2], &md5[2..4], &md5, ext);
    sharedtypes::SubTag {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "source_url".to_string(),
            description: None,
        },
        tag: url,
        tag_type: sharedtypes::TagType::Normal,
        limit_to: None,
    }
}

fn subgen(
    name: &NsIdent,
    tag: String,
    ttype: sharedtypes::TagType,
    limit_to: Option<sharedtypes::Tag>,
) -> sharedtypes::SubTag {
    sharedtypes::SubTag {
        namespace: nsobjplg(name),
        tag,
        tag_type: ttype,
        limit_to,
    }
}

fn nsobjplg(name: &NsIdent) -> sharedtypes::GenericNamespaceObj {
    match name {
        NsIdent::PoolUpdatedAt => {
            return sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: "Pool_Updated_At".to_string(),
                description: Some("Pool When the pool was last updated.".to_string()),
            };
        }
        NsIdent::PoolCreatedAt => {
            return sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: "Pool_Created_At".to_string(),
                description: Some("Pool When the pool was created.".to_string()),
            };
        }
        NsIdent::PoolId => {
            return sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: "Pool_Id".to_string(),
                description: Some("Pool identifier unique id.".to_string()),
            };
        }
        NsIdent::PoolCreator => {
            return sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: "Pool_Creator".to_string(),
                description: Some("Person who made a pool.".to_string()),
            };
        }
        NsIdent::PoolCreatorId => {
            return sharedtypes::GenericNamespaceObj {
                name: "Pool_Creator_Id".to_string(),
                description: Some("Person's id for e6 who made a pool.".to_string()),
            };
        }
        NsIdent::PoolName => {
            return sharedtypes::GenericNamespaceObj {
                name: "Pool_Name".to_string(),
                description: Some("Name of a pool.".to_string()),
            };
        }

        NsIdent::PoolDescription => {
            return sharedtypes::GenericNamespaceObj {
                name: "Pool_Description".to_string(),
                description: Some("Description for a pool.".to_string()),
            };
        }
        NsIdent::PoolPosition => {
            return sharedtypes::GenericNamespaceObj {
                name: "Pool_Position".to_string(),
                description: Some("Position of an id in a pool.".to_string()),
            };
        }
        NsIdent::General => {
            return sharedtypes::GenericNamespaceObj {
                name: "General".to_string(),
                description: Some("General namespace for e621.".to_string()),
            };
        }

        NsIdent::Species => {
            return sharedtypes::GenericNamespaceObj {
                name: "Species".to_string(),
                description: Some("Species namespace for e621.".to_string()),
            };
        }

        NsIdent::Character => {
            return sharedtypes::GenericNamespaceObj {
                name: "Character".to_string(),
                description: Some("What character's are in an image.".to_string()),
            };
        }
        NsIdent::Copyright => {
            return sharedtypes::GenericNamespaceObj {
                name: "Copyright".to_string(),
                description: Some("Who holds the copyright info".to_string()),
            };
        }
        NsIdent::Artist => {
            return sharedtypes::GenericNamespaceObj {
                name: "Artist".to_string(),
                description: Some("Individual who drew the filth.".to_string()),
            };
        }

        NsIdent::Lore => {
            return sharedtypes::GenericNamespaceObj {
                name: "Lore".to_string(),
                description: Some("Youre obviously here for the plot. :X".to_string()),
            };
        }

        NsIdent::Meta => {
            return sharedtypes::GenericNamespaceObj {
                name: "Meta".to_string(),
                description: Some(
                    "Additional information not relating directly to the file".to_string(),
                ),
            };
        }
        NsIdent::Sources => {
            return sharedtypes::GenericNamespaceObj {
                name: "Sources".to_string(),
                description: Some("Additional sources for a file.".to_string()),
            };
        }

        NsIdent::Children => {
            return sharedtypes::GenericNamespaceObj {
                name: "Children".to_string(),
                description: Some(
                    "Files that have a sub relationship to the current file.".to_string(),
                ),
            };
        }
        NsIdent::Parent => {
            return sharedtypes::GenericNamespaceObj {
                name: "Parent_id".to_string(),
                description: Some("Files that are dom or above the current file.".to_string()),
            };
        }
        NsIdent::Description => {
            return sharedtypes::GenericNamespaceObj {
                name: "Description".to_string(),
                description: Some("The description of a file.".to_string()),
            };
        }

        NsIdent::Rating => {
            return sharedtypes::GenericNamespaceObj {
                name: "Rating".to_string(),
                description: Some("The rating of the file.".to_string()),
            };
        }
        NsIdent::FileId => {
            return sharedtypes::GenericNamespaceObj {
                name: "Id".to_string(),
                description: Some("File id used by e621 to uniquly identify a file.".to_string()),
            }
        } /*"pool_id" => {

                  return sharedtypes::PluginRelatesObj {
                      id: None,
                      name: Some("pool_id".to_string()),
                      description: Some("Pool identifier unique id.".to_string()),
              }
          }
          _ => {
              panic!();
          }*/
    }
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
#[no_mangle]
pub fn test() -> u8 {
    0
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
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
) -> (Vec<String>, sharedtypes::ScraperData) {
    let mut ret = Vec::new();
    let hardlimit = 751;
    for i in 1..hardlimit {
        let a = build_url(params, i);
        ret.push(a);
    }
    (ret, scraperdata.clone())
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
    tags_list: &mut Vec<sharedtypes::TagObject>,
    jso: &json::JsonValue,
    ns: sharedtypes::GenericNamespaceObj,
    relates: Option<sharedtypes::SubTag>,
    tagtype: sharedtypes::TagType,
) {
    //println!("jsonsubtag {:?}, {}", jso, &ns.name);

    match relates {
        None => {
            for each in jso[ns.name.clone().to_lowercase()].members() {
                //println!("jsosub {}", &each);
                tags_list.push(sharedtypes::TagObject {
                    namespace: ns.clone(),
                    relates_to: None,
                    tag: each.to_string(),
                    tag_type: tagtype.clone(),
                });
            }
        }
        Some(temp) => {
            //let temp = relates.unwrap().1;
            for each in jso[ns.name.clone().to_lowercase()].members() {
                tags_list.push(sharedtypes::TagObject {
                    namespace: ns.clone(),
                    relates_to: Some(temp.clone()),
                    tag: each.to_string(),
                    tag_type: tagtype.clone(),
                });
            }
        }
    }
}
/*
*
*let url = match js["posts"][cnt]["file"]["url"].is_null() {
               false => js["posts"][cnt]["file"]["url"].to_string(),
               true => {
                   let md5 = js["posts"][cnt]["file"]["md5"].to_string();
                   let ext = js["posts"][cnt]["file"]["ext"].to_string();
                   gen_source_from_md5_ext(&md5, &ext)
               }
           };
           let mut tag_list = Vec::new();

           json_sub_tag(
               &mut tag_list,
               &js["posts"][cnt],
               nsobjplg(&NsIdent::PoolId),
               Some(subgen(
                   &NsIdent::FileId,
                   js["posts"][cnt]["id"].to_string(),
                   sharedtypes::TagType::Normal,
               )),
               sharedtypes::TagType::Normal,
           );

           let file: sharedtypes::FileObject = sharedtypes::FileObject {
               source_url: Some(url),
               hash: Some(sharedtypes::HashesSupported::Md5(
                   js["posts"][cnt]["file"]["md5"].to_string(),
               )),
               tag_list: tag_list,
           };
           files.insert(file);
*
*
*/
fn parse_pools(
    js: &json::JsonValue,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<(sharedtypes::ScraperObject, sharedtypes::ScraperData), sharedtypes::ScraperReturn> {
    let mut files: HashSet<sharedtypes::FileObject> = HashSet::default();
    let mut tag: HashSet<sharedtypes::TagObject> = HashSet::default();
    let mut cnttotal = 0;

    // For each in tag pools pulled.
    for multpool in js.members() {
        if multpool["id"].is_null() {
            continue;
        }

        let mut tag_count: u64 = 0;
        let mut tags_list: Vec<sharedtypes::TagObject> = Vec::new();

        // Add poolid if not exist
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolId),
            relates_to: None,
            tag: multpool["id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool creator
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolCreator),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            tag: multpool["creator_name"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool creator id
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolCreatorId),
            relates_to: Some(subgen(
                &NsIdent::PoolCreator,
                multpool["creator_name"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            tag: multpool["creator_id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool name
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolName),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            tag: multpool["name"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool description
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::Description),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            tag: multpool["description"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        let created_at = DateTime::parse_from_str(
            &multpool["created_at"].to_string(),
            "%Y-%m-%dT%H:%M:%S.%f%:z",
        )
        .unwrap()
        .timestamp()
        .to_string();

        let updated_at = DateTime::parse_from_str(
            &multpool["updated_at"].to_string(),
            "%Y-%m-%dT%H:%M:%S.%f%:z",
        )
        .unwrap()
        .timestamp()
        .to_string();

        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolCreatedAt),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            tag: created_at,
            tag_type: sharedtypes::TagType::Normal,
        });

        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolUpdatedAt),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            tag: updated_at,
            tag_type: sharedtypes::TagType::Normal,
        });
        let mut cnt = 0;
        for postids in multpool["post_ids"].members() {
            tag.insert(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::PoolId),
                relates_to: None,
                tag: multpool["id"].to_string(),
                tag_type: sharedtypes::TagType::ParseUrl((
                    (sharedtypes::ScraperData {
                        job: sharedtypes::JobScraper {
                            site: "e6".to_string(),
                            param: Vec::new(),
                            original_param: format!(
                                "https://e621.net/posts.json?tags=id:{}",
                                postids
                            ),
                            job_type: sharedtypes::DbJobType::Scraper,
                        },
                        system_data: BTreeMap::new(),
                        user_data: BTreeMap::new(),
                    }),
                    sharedtypes::SkipIf::Tag(sharedtypes::Tag {
                        tag: postids.to_string(),
                        namespace: nsobjplg(&NsIdent::FileId),
                        needsrelationship: true,
                    }),
                )),
            });

            // Relates fileid to position in table with the restriction of the poolid
            tag.insert(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::PoolPosition),
                relates_to: Some(subgen(
                    &NsIdent::FileId,
                    postids.to_string(),
                    sharedtypes::TagType::Normal,
                    Some(sharedtypes::Tag {
                        tag: multpool["id"].to_string(),
                        namespace: nsobjplg(&NsIdent::PoolId),
                        needsrelationship: false,
                    }),
                )),
                tag: cnt.to_string(),
                tag_type: sharedtypes::TagType::Normal,
            });

            cnt += 1;
        }
        files.insert(sharedtypes::FileObject {
            source_url: None,
            hash: sharedtypes::HashesSupported::None,
            tag_list: tags_list,
            skip_if: Vec::new(),
        });
    }

    Ok((
        sharedtypes::ScraperObject {
            file: files,
            tag: tag,
        },
        scraperdata.clone(),
    ))
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(
    params: &String,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<(sharedtypes::ScraperObject, sharedtypes::ScraperData), sharedtypes::ScraperReturn> {
    //let vecvecstr: AHashMap<String, AHashMap<String, Vec<String>>> = AHashMap::new();

    let mut files: HashSet<sharedtypes::FileObject> = HashSet::default();
    let js = match json::parse(params) {
        Err(err) => {
            if params.contains("Please confirm you are not a robot.") {
                return Err(sharedtypes::ScraperReturn::Timeout(20));
            } else if params.contains("502: Bad gateway") {
                return Err(sharedtypes::ScraperReturn::Timeout(10));
            } else if params.contains("SSL handshake failed") {
                return Err(sharedtypes::ScraperReturn::Timeout(10));
            } else if params.contains("e621 Maintenance") {
                return Err(sharedtypes::ScraperReturn::Timeout(240));
            }
            return Err(sharedtypes::ScraperReturn::EMCStop(format!(
                "Unknown Error: {}",
                err
            )));
        }
        Ok(out) => out,
    };

    //let mut file = File::create("main1.json").unwrap();

    // Write a &str in the file (ignoring the result).
    //writeln!(&mut file, "{}", js.to_string()).unwrap();
    //println!("Parsing");
    if js["posts"].is_empty() & !js["posts"].is_null() {
        return Err(sharedtypes::ScraperReturn::Nothing);
    } else if js["posts"].is_null() {
        let pool = parse_pools(&js, scraperdata);
        return pool;
    }

    for inc in 0..js["posts"].len() {
        let mut tag_count: u64 = 0;

        let mut tags_list: Vec<sharedtypes::TagObject> = Vec::new();
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::General),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::Species),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::Character),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::Copyright),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::Artist),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::Lore),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["tags"],
            nsobjplg(&NsIdent::Meta),
            None,
            sharedtypes::TagType::Normal,
        );
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc],
            nsobjplg(&NsIdent::Sources),
            None,
            sharedtypes::TagType::Normal,
        );

        if !js["posts"][inc]["pools"].is_null() {
            for each in js["posts"][inc]["pools"].members() {
                tags_list.push(sharedtypes::TagObject {
                    namespace: nsobjplg(&NsIdent::PoolId),
                    relates_to: None,
                    tag: each.to_string(),
                    tag_type: sharedtypes::TagType::Normal,
                });
                /*json_sub_tag(
                    &mut tags_list,
                    &js["posts"][inc],
                    nsobjplg(&NsIdent::PoolId),
                    None,
                    sharedtypes::TagType::Normal,
                );*/
                //let parse_url = format!("https://e621.net/pools?format=json&search[id={}]", each);
                let parse_url = format!("https://e621.net/pools.json?search[id]={}", each);
                tags_list.push(sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "Do Not Add".to_string(),
                        description: Some("DO NOT PARSE ME".to_string()),
                    },
                    relates_to: None,
                    tag: parse_url.clone(),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        sharedtypes::ScraperData {
                            job: sharedtypes::JobScraper {
                                site: "e6".to_string(),
                                param: Vec::new(),
                                original_param: parse_url,
                                job_type: sharedtypes::DbJobType::Scraper,
                            },

                            system_data: BTreeMap::new(),
                            user_data: BTreeMap::new(),
                        },
                        sharedtypes::SkipIf::None,
                    )),
                });
            }
        }

        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["relationships"],
            nsobjplg(&NsIdent::Children),
            Some(subgen(
                &NsIdent::FileId,
                js["posts"][inc]["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
            )),
            sharedtypes::TagType::Normal,
        );
        if !js["posts"][inc]["description"].is_empty() {
            tags_list.push(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::Description),
                relates_to: None,
                tag: js["posts"][inc]["description"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            });
        }

        tags_list.push(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::Rating),
            relates_to: None,
            tag: js["posts"][inc]["rating"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        tags_list.push(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::FileId),
            relates_to: None,
            tag: js["posts"][inc]["id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        if !js["posts"][inc]["relationships"]["parent_id"].is_null() {
            tags_list.push(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::Parent),
                relates_to: Some(subgen(
                    &NsIdent::FileId,
                    js["posts"][inc]["id"].to_string(),
                    sharedtypes::TagType::Normal,
                    None,
                )),

                tag: js["posts"][inc]["relationships"]["parent_id"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            });
            json_sub_tag(
                &mut tags_list,
                &js["posts"][inc]["relationships"],
                nsobjplg(&NsIdent::Parent),
                Some(subgen(
                    &NsIdent::FileId,
                    js["posts"][inc]["id"].to_string(),
                    sharedtypes::TagType::Normal,
                    None,
                )),
                sharedtypes::TagType::Normal,
            );
        }

        let url = match js["posts"][inc]["file"]["url"].is_null() {
            false => js["posts"][inc]["file"]["url"].to_string(),
            true => {
                //let base = "https://static1.e621.net/data/1c/a6/1ca6868a2b0f5e7129d2b478198bfa91.webm";
                let md5 = js["posts"][inc]["file"]["md5"].to_string();
                let ext = js["posts"][inc]["file"]["ext"].to_string();
                gen_source_from_md5_ext(&md5, &ext)
            }
        };
        let file: sharedtypes::FileObject = sharedtypes::FileObject {
            source_url: Some(url),
            hash: sharedtypes::HashesSupported::Md5(js["posts"][inc]["file"]["md5"].to_string()),
            tag_list: tags_list,
            skip_if: Vec::new(),
        };
        files.insert(file);
    }
    Ok((
        sharedtypes::ScraperObject {
            file: files,
            tag: HashSet::new(),
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

fn gen_source_from_md5_ext(md5: &String, ext: &String) -> String {
    let base = "https://static1.e621.net/data";

    format!("{}/{}/{}/{}.{}", base, &md5[0..2], &md5[2..4], &md5, ext)
}
#[path = "../../../src/scr/intcoms/client.rs"]
mod client;

pub fn db_upgrade_call_3() {
    dbg!("E6 GOING TO LOCK DB DOES THIS WORKY");
    client::load_table(sharedtypes::LoadDBTable::All);

    // Loads all fileids into memory
    let mut file_ids = client::file_get_list_all();

    // Gets namespace id from poolid
    let pool_nsid = match client::namespace_get(nsobjplg(&NsIdent::PoolId).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::PoolId).name,
            nsobjplg(&NsIdent::PoolId).description,
            true,
        ),
    };
    // Gets namespace id from poolid
    let poolposition_nsid = match client::namespace_get(nsobjplg(&NsIdent::PoolPosition).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::PoolPosition).name,
            nsobjplg(&NsIdent::PoolPosition).description,
            true,
        ),
    };

    // Gets e6id from db
    let fileid_nsid = match client::namespace_get(nsobjplg(&NsIdent::FileId).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::FileId).name,
            nsobjplg(&NsIdent::FileId).description,
            true,
        ),
    }; // Gets e6's parent ids from db
    let parent_nsid = match client::namespace_get(nsobjplg(&NsIdent::Parent).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::Parent).name,
            nsobjplg(&NsIdent::Parent).description,
            true,
        ),
    }; // Gets e6's children id's from db
    let children_nsid = match client::namespace_get(nsobjplg(&NsIdent::Children).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::Children).name,
            nsobjplg(&NsIdent::Children).description,
            true,
        ),
    };

    // Loads all tagid's that are attached to the pool
    let pool_table = match client::namespace_get_tagids(pool_nsid) {
        None => HashSet::new(),
        Some(out) => out,
    };
    // Gets namespace id from source urls ensures that we're only working on e621 files
    let sourceurl_nsid = match client::namespace_get("source_url".to_string()) {
        Some(id) => id,
        None => client::namespace_put("source_url".to_string(), None, true),
    };

    // Loads all tagid's that are attached to the e621 sources
    let sourceurl_table = match client::namespace_get_tagids(sourceurl_nsid) {
        None => HashSet::new(),
        Some(out) => out,
    };

    // Loads all tagid's that are attached to the parents sources
    let parent_table = match client::namespace_get_tagids(parent_nsid) {
        None => HashSet::new(),
        Some(out) => out,
    }; // Loads all tagid's that are attached to the children sources
    let children_table = match client::namespace_get_tagids(children_nsid) {
        None => HashSet::new(),
        Some(out) => out,
    }; // Loads all tagid's that are attached to the position
    let position_table = match client::namespace_get_tagids(poolposition_nsid) {
        None => HashSet::new(),
        Some(out) => out,
    };

    client::log(format!(
        "E6 Scraper-Starting to strip: {} fileids from processing list",
        file_ids.len()
    ));
    let mut cnt = 0;
    // Removes all fileids where the source is not e621
    for each in sourceurl_table {
        if let Some(tag) = client::tag_get_id(each) {
            if !tag.name.contains("e621.net") {
                if let Some(fids) = client::relationship_get_fileid(each) {
                    for fid in fids.iter() {
                        file_ids.remove(fid);
                        cnt += 1;
                    }
                }
            }
        }
    }

    client::log(format!(
        "E6 Scraper-Stripped: {} fileids from processing list",
        cnt
    ));
    for (each, _) in file_ids {
        if let Some(tids) = client::relationship_get_tagid(each) {
            for tid in tids.intersection(&pool_table) {
                if let Some(tag_rels) =
                    client::parents_get(crate::client::types::ParentsType::Tag, tid.clone())
                {
                    dbg!(tid, &tag_rels);
                    let mut vec_poolpos = Vec::new();
                    let mut hashset_fileid = HashSet::new();
                    for each in tag_rels {
                        if let Some(tag_nns) = client::tag_get_id(each) {
                            // Removes the spare poolid tag as a position that I added for some
                            // reason. lol
                            if tag_nns.namespace == poolposition_nsid {
                                /*client::parents_delete(sharedtypes::DbParentsObj {
                                    tag_id: *tid,
                                    relate_tag_id: each.clone(),
                                    limit_to: None,
                                });*/

                                vec_poolpos.push(each);
                            } else if tag_nns.namespace == fileid_nsid {
                                hashset_fileid.insert(each);
                            }
                        }
                    }

                    for position in vec_poolpos.iter() {
                        if let Some(tag_id) =
                            client::parents_get(crate::client::types::ParentsType::Rel, *position)
                        {
                            //dbg!("FID'S POSITION", &tag_id, tid, &tag_id.len());
                        }
                    }
                    for fid in hashset_fileid.iter() {
                        if let Some(mut tag_id) =
                            client::parents_get(crate::client::types::ParentsType::Rel, *fid)
                        {
                            // Removes the parents and children from tag_ids
                            for tid_iter in tag_id.clone().iter() {
                                if parent_table.contains(tid_iter)
                                    || children_table.contains(tid_iter)
                                {
                                    tag_id.remove(tid_iter);
                                }
                            }
                            if tag_id.len() < 2 {
                                dbg!("LESS 2 ITEMS IN HERE", tag_id);
                                dbg!(&fid, tid);
                            } else if tag_id.len() > 2 {
                                // Clears sub items and add job to db to
                                // scrape
                                dbg!("MORE THEN 2 ITEMS IN HERE", &tag_id);
                                dbg!(&fid, tid);
                                for tid_iter in tag_id.iter() {
                                    if pool_table.contains(&tid_iter) {
                                        client::job_add(
                                            None,
                                            0,
                                            0,
                                            "e6".to_string(),
                                            format!(
                                                "https://e621.net/pools.json?search[id]={}",
                                                client::tag_get_id(*tid_iter).unwrap().name
                                            ),
                                            true,
                                            sharedtypes::CommitType::StopOnNothing,
                                            sharedtypes::DbJobType::Scraper,
                                            BTreeMap::new(),
                                            BTreeMap::new(),
                                        );
                                    }
                                    client::parents_delete(sharedtypes::DbParentsObj {
                                        tag_id: *tid_iter,
                                        relate_tag_id: *fid,
                                        limit_to: None,
                                    });
                                }
                            } else {
                                let mut pos = None;
                                // Updates the pool position.
                                // Clears out relations not including children and parents
                                // Adds relation if it exists properly
                                for tid_iter in tag_id.iter() {
                                    client::parents_delete(sharedtypes::DbParentsObj {
                                        tag_id: *tid_iter,
                                        relate_tag_id: *fid,
                                        limit_to: None,
                                    });
                                    if position_table.contains(&tid_iter) {
                                        pos = Some(sharedtypes::DbParentsObj {
                                            tag_id: *tid_iter,
                                            relate_tag_id: *fid,
                                            limit_to: Some(*tid),
                                        });
                                    }
                                }
                                if let Some(pos) = pos {
                                    client::parents_put(pos);
                                }
                                dbg!("FID TO REMOVE FROM TAG_ID", &fid, tid, &tag_id);
                            }
                        }
                    }
                }
            }
        }
    }

    client::transaction_flush();
}

#[no_mangle]
pub fn db_upgrade_call(db_version: &usize) {
    match db_version {
        3 => {
            db_upgrade_call_3();
        }
        _ => {
            client::log_no_print(format!("E621 No upgrade for version: {}", db_version));
        }
    }
}
