use chrono::DateTime;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;

//use ahash::HashSet;
//use ahash::HashSet;

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

use crate::sharedtypes::DEFAULT_PRIORITY;
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

#[derive(Debug)]
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
    Contributor,
    Species,
    General,
    Director,
    Franchise,
}

#[derive(Debug)]
pub enum Site {
    E6,
    E6AI,
}

///
/// Converts Site into Strings
///
fn site_to_string(site: &Site) -> String {
    match site {
        Site::E6 => "E621".to_string(),
        Site::E6AI => "E6AI".to_string(),
    }
}

///
/// Converts String into site
///
fn string_to_site(site: &str) -> Option<Site> {
    match site {
        "E621" => Some(Site::E6),
        "E6AI" => Some(Site::E6AI),
        _ => None,
    }
}

///
/// Converts a site to a preferred site prefix
///
fn site_to_string_prefix(site: &Site) -> String {
    match site {
        Site::E6 => "e6".to_string(),
        Site::E6AI => "e6ai".to_string(),
    }
}

#[no_mangle]
fn scraper_file_regen() -> sharedtypes::ScraperFileRegen {
    sharedtypes::ScraperFileRegen {
        hash: sharedtypes::HashesSupported::Md5("".to_string()),
    }
}

fn subgen(
    name: &NsIdent,
    tag: String,
    ttype: sharedtypes::TagType,
    limit_to: Option<sharedtypes::Tag>,
    site: &Site,
) -> sharedtypes::SubTag {
    sharedtypes::SubTag {
        namespace: nsobjplg(name, site),
        tag,
        tag_type: ttype,
        limit_to,
    }
}

fn nsobjplg(name: &NsIdent, site: &Site) -> sharedtypes::GenericNamespaceObj {
    match name {
        NsIdent::Franchise => {
            sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: format!("{}_Franchise", site_to_string(site)),
                description: Some("Franchise that this item came from.".to_string()),
            }
        }

        NsIdent::Director => {
            sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: format!("{}_Director", site_to_string(site)),
                description: Some("The director of the ai filth.".to_string()),
            }
        }

        NsIdent::PoolUpdatedAt => {
            sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: format!("{}_Pool_Updated_At", site_to_string(site)),
                description: Some("Pool When the pool was last updated.".to_string()),
            }
        }
        NsIdent::PoolCreatedAt => {
            sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: format!("{}_Created_At", site_to_string(site)),
                description: Some("Pool When the pool was created.".to_string()),
            }
        }
        NsIdent::PoolId => {
            sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: format!("{}_Pool_Id", site_to_string(site)),
                description: Some("Pool identifier unique id.".to_string()),
            }
        }
        NsIdent::PoolCreator => {
            sharedtypes::GenericNamespaceObj {
                //tag: tag,
                name: format!("{}_Pool_Creator", site_to_string(site)),
                description: Some("Person who made a pool.".to_string()),
            }
        }
        NsIdent::PoolCreatorId => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Pool_Creator_ID", site_to_string(site)),
                description: Some(format!(
                    "Person's id for {} who made a pool.",
                    site_to_string(site)
                )),
       }
        }
        NsIdent::PoolName => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Pool_Name", site_to_string(site)),
                description: Some("Name of a pool.".to_string()),
            }
        }

        NsIdent::PoolDescription => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Pool_Description", site_to_string(site)),
                description: Some("Description for a pool.".to_string()),
            }
        }
        NsIdent::PoolPosition => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Pool_Position", site_to_string(site)),
                description: Some("Position of an id in a pool.".to_string()),
            }
        }
        NsIdent::General => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_General", site_to_string(site)),
                description: Some(format!("General namespace for {}.", site_to_string(site))),
            }
        }

        NsIdent::Species => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Species", site_to_string(site)),
                description: Some(format!("Species namespace for {}.", site_to_string(site))),
            }
        }

        NsIdent::Character => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Character", site_to_string(site)),
                description: Some("What character's are in an image.".to_string()),
            }
        }
        NsIdent::Contributor => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Contributor", site_to_string(site)),
                description: Some("For those who helped make a piece of art not directly the artist think of VA's and such.".to_string()),
            }
        }

        NsIdent::Copyright => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Copyright", site_to_string(site)),
                description: Some("Who holds the copyright info".to_string()),
            }
        }
        NsIdent::Artist => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Artist", site_to_string(site)),
                description: Some("Individual who drew the filth.".to_string()),
            }
        }

        NsIdent::Lore => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Lore", site_to_string(site)),
                description: Some("Youre obviously here for the plot. :X".to_string()),
            }
        }

        NsIdent::Meta => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Meta", site_to_string(site)),
                description: Some(
                    "Additional information not relating directly to the file".to_string(),
                ),
            }
        }
        NsIdent::Sources => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Sources", site_to_string(site)),
                description: Some("Additional sources for a file.".to_string()),
            }
        }

        NsIdent::Children => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Children", site_to_string(site)),
                description: Some(
                    "Files that have a sub relationship to the current file.".to_string(),
                ),
            }
        }
        NsIdent::Parent => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Parent_id", site_to_string(site)),
                description: Some("Files that are dom or above the current file.".to_string()),
            }
        }
        NsIdent::Description => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Description", site_to_string(site)),
                description: Some("The description of a file.".to_string()),
            }
        }

        NsIdent::Rating => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Rating", site_to_string(site)),
                description: Some("The rating of the file.".to_string()),
            }
        }
        NsIdent::FileId => {
            sharedtypes::GenericNamespaceObj {
                name: format!("{}_Id", site_to_string(site)),
                description: Some(format!(
                    "File id used by {} to uniquly identify a file.",
                    site_to_string(site)
                )),
            }
        }
    }
}

///
/// Builds the URL for scraping activities.
///
fn build_url(params: &[sharedtypes::ScraperParam], pagenum: u64, site: &Site) -> String {
    let lowercase_site = site_to_string(site).to_lowercase();
    let url_base = format!("https://{}.net/posts.json", lowercase_site);
    let page = "&page=";
    let mut params_normal: Vec<&String> = Vec::new();

    let mut string_builder = String::new();
    let mut first_term = true;

    if params.is_empty() {
        return string_builder;
    }

    string_builder += &url_base;

    // Adds api login info into the url
    for each in params.iter() {
        if let sharedtypes::ScraperParam::Login(sharedtypes::LoginType::ApiNamespaced(
            _,
            username,
            val,
        )) = each
        {
            if let (Some(username), Some(val)) = (username, val) {
                string_builder += &format!("?login={}&api_key={}", username, val);
                first_term = false;
            }
            break;
        }
        /*if let sharedtypes::ScraperParam::Login(each) = each {
            if let sharedtypes::LoginType::ApiNamespaced(_, username, val) = each {
                if let (Some(username), Some(val)) = (username, val) {
                    string_builder += &format!("?login={}&api_key={}", username, val);
                    first_term = false;
                }
                break;
            }
        }*/
    }

    // Gets params into db.
    for each in params {
        if let sharedtypes::ScraperParam::Normal(inp) = each {
            params_normal.push(inp);
        }
    }

    string_builder += if first_term { "?tags=" } else { "&tags=" };
    match params_normal.len() {
        0 => return "".to_string(),
        1 => {
            string_builder += params_normal.pop().unwrap();
        }
        _ => {
            let last_searched_tag = params_normal.pop().unwrap();
            for each in params_normal {
                string_builder += each;
                string_builder += "+";
            }
            string_builder += last_searched_tag;
        }
    }

    string_builder += page;
    string_builder += &pagenum.to_string();

    // Returns url
    string_builder
}

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut e6 = sharedtypes::return_default_globalpluginparser();

    e6.name = "E621.net".to_string();
    e6.version = 0;
    e6.login_type = vec![(
        "E621".to_string(),
        sharedtypes::LoginType::ApiNamespaced("User_Api_Login".to_string(), None, None),
        sharedtypes::LoginNeed::Optional,
        Some("Username and API key goes in here.".to_string()),
        false,
    )];
    e6.stored_info = Some(sharedtypes::StoredInfo::Storage(vec![(
        "loaded_site".to_string(),
        "E621".to_string(),
    )]));
    e6.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec_of_strings!("e6", "e621", "e621.net"),
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![],
        },
    ));
    e6.callbacks = vec![sharedtypes::GlobalCallbacks::Start(
        sharedtypes::StartupThreadType::Spawn,
    )];

    let mut e6ai = sharedtypes::return_default_globalpluginparser();
    e6ai.name = "E6AI.net".to_string();
    e6ai.version = 0;
    e6ai.login_type = vec![(
        "E6AI".to_string(),
        sharedtypes::LoginType::ApiNamespaced("User_Api_Login".to_string(), None, None),
        sharedtypes::LoginNeed::Optional,
        Some("Username and API key goes in here.".to_string()),
        false,
    )];
    e6ai.stored_info = Some(sharedtypes::StoredInfo::Storage(vec![(
        "loaded_site".to_string(),
        "E6AI".to_string(),
    )]));
    e6ai.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec_of_strings!("e6ai", "e6ai.net"),
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![],
        },
    ));
    e6ai.callbacks = vec![sharedtypes::GlobalCallbacks::Start(
        sharedtypes::StartupThreadType::Spawn,
    )];

    vec![e6, e6ai]
}

#[no_mangle]
pub fn test() -> u8 {
    0
}

/*///
/// Returns one url from the parameters.
///
#[no_mangle]
pub fn url_get(params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
    vec![build_url(params, 1)]
}*/
///
/// Dumps a list of urls to scrape
///
#[no_mangle]
pub fn url_dump(
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let site_a = scraperdata.user_data.get("loaded_site");
    let site = match site_a {
        Some(sitename) => match string_to_site(sitename) {
            Some(out) => out,
            None => {
                return vec![];
            }
        },
        None => {
            return vec![];
        }
    };

    let mut ret = Vec::new();
    let hardlimit = 751;
    for i in 1..hardlimit {
        let a = build_url(params, i, &site);
        ret.push((a, scraperdata.clone()));
    }
    ret
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
    name_search: &str,
    relates: Option<sharedtypes::SubTag>,
    tagtype: sharedtypes::TagType,
) {
    //println!("jsonsubtag {:?}, {}", jso, &ns.name);

    match relates {
        None => {
            for each in jso[name_search].members() {
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
            for each in jso[name_search].members() {
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
fn parse_pools(
    js: &json::JsonValue,
    scraperdata: &sharedtypes::ScraperData,
    site: &Site,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let mut files: HashSet<sharedtypes::FileObject> = HashSet::default();
    let mut tag: HashSet<sharedtypes::TagObject> = HashSet::default();

    // For each in tag pools pulled.
    for multpool in js.members() {
        if multpool["id"].is_null() {
            continue;
        }

        let tags_list: Vec<sharedtypes::TagObject> = Vec::new();

        // Add poolid if not exist
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolId, site),
            relates_to: None,
            tag: multpool["id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool creator
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolCreator, site),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                site,
            )),
            tag: multpool["creator_name"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool creator id
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolCreatorId, site),
            relates_to: Some(subgen(
                &NsIdent::PoolCreator,
                multpool["creator_name"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                site,
            )),
            tag: multpool["creator_id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool name
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolName, site),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                site,
            )),
            tag: multpool["name"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        // Add pool description
        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::Description, site),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                site,
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
            namespace: nsobjplg(&NsIdent::PoolCreatedAt, site),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                site,
            )),
            tag: created_at,
            tag_type: sharedtypes::TagType::Normal,
        });

        tag.insert(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::PoolUpdatedAt, site),
            relates_to: Some(subgen(
                &NsIdent::PoolId,
                multpool["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                site,
            )),
            tag: updated_at,
            tag_type: sharedtypes::TagType::Normal,
        });
        for (cnt, postids) in multpool["post_ids"].members().enumerate() {
            if let Some(recursion) = scraperdata.system_data.get("recursion") {
                if recursion == "false" {
                    tag.insert(sharedtypes::TagObject {
                        namespace: nsobjplg(&NsIdent::PoolId, site),
                        relates_to: None,
                        tag: multpool["id"].to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                    });
                } else {
                    let lowercase_site = site_to_string(site).to_lowercase();
                    tag.insert(sharedtypes::TagObject {
                        namespace: nsobjplg(&NsIdent::PoolId, site),
                        relates_to: None,
                        tag: multpool["id"].to_string(),
                        tag_type: sharedtypes::TagType::ParseUrl((
                            (sharedtypes::ScraperData {
                                job: sharedtypes::JobScraper {
                                    site: site_to_string_prefix(site),
                                    param: vec![sharedtypes::ScraperParam::Url(format!(
                                        "https://{}.net/posts.json?tags=id:{}+status:any",
                                        lowercase_site, postids
                                    ))],
                                    job_type: sharedtypes::DbJobType::Scraper,
                                },
                                system_data: BTreeMap::new(),
                                user_data: BTreeMap::new(),
                            }),
                            Some(sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                                tag: postids.to_string(),
                                namespace: nsobjplg(&NsIdent::FileId, site),
                            })),
                        )),
                    });
                }
            } else {
                let lowercase_site = site_to_string(site).to_lowercase();
                tag.insert(sharedtypes::TagObject {
                    namespace: nsobjplg(&NsIdent::PoolId, site),
                    relates_to: None,
                    tag: multpool["id"].to_string(),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        (sharedtypes::ScraperData {
                            job: sharedtypes::JobScraper {
                                site: site_to_string_prefix(site),
                                param: vec![sharedtypes::ScraperParam::Url(format!(
                                    "https://{}.net/posts.json?tags=id:{}+status:any",
                                    lowercase_site, postids
                                ))],
                                job_type: sharedtypes::DbJobType::Scraper,
                            },
                            system_data: BTreeMap::new(),
                            user_data: BTreeMap::new(),
                        }),
                        Some(sharedtypes::SkipIf::FileTagRelationship(sharedtypes::Tag {
                            tag: postids.to_string(),
                            namespace: nsobjplg(&NsIdent::FileId, site),
                        })),
                    )),
                });
            }

            // Relates fileid to position in table with the restriction of the poolid
            tag.insert(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::PoolPosition, site),
                relates_to: Some(subgen(
                    &NsIdent::FileId,
                    postids.to_string(),
                    sharedtypes::TagType::Normal,
                    Some(sharedtypes::Tag {
                        tag: multpool["id"].to_string(),
                        namespace: nsobjplg(&NsIdent::PoolId, site),
                    }),
                    site,
                )),
                tag: cnt.to_string(),
                tag_type: sharedtypes::TagType::Normal,
            });
        }
        files.insert(sharedtypes::FileObject {
            source: None,
            hash: sharedtypes::HashesSupported::None,
            tag_list: tags_list,
            skip_if: Vec::new(),
        });
    }

    Ok(sharedtypes::ScraperObject {
        file: files,
        tag,
        flag: vec![],
    })
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(
    html_input: &str,
    _: &str,
    scraperdata: &sharedtypes::ScraperData,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    //let vecvecstr: AHashMap<String, AHashMap<String, Vec<String>>> = AHashMap::new();

    let site_a = scraperdata.user_data.get("loaded_site");
    let site = match site_a {
        Some(sitename) => match string_to_site(sitename) {
            Some(out) => out,
            None => {
                return Err(sharedtypes::ScraperReturn::Nothing);
            }
        },
        None => {
            return Err(sharedtypes::ScraperReturn::Nothing);
        }
    };

    let mut files: HashSet<sharedtypes::FileObject> = HashSet::default();
    let js = match json::parse(html_input) {
        Err(err) => {
            if html_input.contains("Please confirm you are not a robot.") {
                return Err(sharedtypes::ScraperReturn::Timeout(20));
            } else if html_input.contains("502: Bad gateway")
                | html_input.contains("SSL handshake failed")
            {
                return Err(sharedtypes::ScraperReturn::Timeout(10));
            } else if html_input.contains(&format!(
                "{} Maintenance",
                site_to_string(&site).to_lowercase()
            )) {
                return Err(sharedtypes::ScraperReturn::Timeout(240));
            }
            return Err(sharedtypes::ScraperReturn::Stop(format!(
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

    if js["posts"].is_empty() & js["posts"].is_array() {
        return Err(sharedtypes::ScraperReturn::Nothing);
    } else if js["posts"].is_null() {
        let pool = parse_pools(&js, scraperdata, &site);
        return pool;
    }

    for inc in 0..js["posts"].len() {
        let search_list = [
            ("general", &NsIdent::General),
            ("contributor", &NsIdent::Contributor),
            ("franchise", &NsIdent::Franchise),
            ("director", &NsIdent::Director),
            ("species", &NsIdent::Species),
            ("character", &NsIdent::Character),
            ("copyright", &NsIdent::Copyright),
            ("artist", &NsIdent::Artist),
            ("lore", &NsIdent::Lore),
            ("meta", &NsIdent::Meta),
        ];

        let mut tags_list: Vec<sharedtypes::TagObject> = Vec::new();
        for (search, nsident) in search_list {
            json_sub_tag(
                &mut tags_list,
                &js["posts"][inc]["tags"],
                nsobjplg(nsident, &site),
                search,
                None,
                sharedtypes::TagType::Normal,
            );
        }
        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc],
            nsobjplg(&NsIdent::Sources, &site),
            "sources",
            None,
            sharedtypes::TagType::Normal,
        );

        if !js["posts"][inc]["pools"].is_null() {
            for each in js["posts"][inc]["pools"].members() {
                tags_list.push(sharedtypes::TagObject {
                    namespace: nsobjplg(&NsIdent::PoolId, &site),
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
                let shouldparse = if let Some(recursion) = scraperdata.system_data.get("recursion")
                {
                    recursion != "false"
                } else {
                    true
                };
                if shouldparse {
                    let lowercase_site = site_to_string(&site).to_lowercase();
                    let parse_url = format!(
                        "https://{}.net/pools.json?search[id]={}",
                        lowercase_site, each
                    );
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
                                    site: site_to_string_prefix(&site).to_string(),
                                    param: vec![sharedtypes::ScraperParam::Url(parse_url)],
                                    job_type: sharedtypes::DbJobType::Scraper,
                                },

                                system_data: BTreeMap::new(),
                                user_data: BTreeMap::new(),
                            },
                            None,
                        )),
                    });
                }
            }
        }

        json_sub_tag(
            &mut tags_list,
            &js["posts"][inc]["relationships"],
            nsobjplg(&NsIdent::Children, &site),
            "children",
            Some(subgen(
                &NsIdent::FileId,
                js["posts"][inc]["id"].to_string(),
                sharedtypes::TagType::Normal,
                None,
                &site,
            )),
            sharedtypes::TagType::Normal,
        );
        if !js["posts"][inc]["description"].is_empty() {
            tags_list.push(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::Description, &site),
                relates_to: None,
                tag: js["posts"][inc]["description"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            });
        }

        tags_list.push(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::Rating, &site),
            relates_to: None,
            tag: js["posts"][inc]["rating"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        tags_list.push(sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::FileId, &site),
            relates_to: None,
            tag: js["posts"][inc]["id"].to_string(),
            tag_type: sharedtypes::TagType::Normal,
        });

        if !js["posts"][inc]["relationships"]["parent_id"].is_null() {
            tags_list.push(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::Parent, &site),
                relates_to: Some(subgen(
                    &NsIdent::FileId,
                    js["posts"][inc]["id"].to_string(),
                    sharedtypes::TagType::Normal,
                    None,
                    &site,
                )),

                tag: js["posts"][inc]["relationships"]["parent_id"].to_string(),
                tag_type: sharedtypes::TagType::Normal,
            });
            json_sub_tag(
                &mut tags_list,
                &js["posts"][inc]["relationships"],
                nsobjplg(&NsIdent::Parent, &site),
                "parent_id",
                Some(subgen(
                    &NsIdent::FileId,
                    js["posts"][inc]["id"].to_string(),
                    sharedtypes::TagType::Normal,
                    None,
                    &site,
                )),
                sharedtypes::TagType::Normal,
            );
        }

        let url = match js["posts"][inc]["file"]["url"].is_null() {
            false => js["posts"][inc]["file"]["url"].to_string(),
            true => {
                let md5 = js["posts"][inc]["file"]["md5"].to_string();
                let ext = js["posts"][inc]["file"]["ext"].to_string();
                gen_source_from_md5_ext(&md5, &ext, &site)
            }
        };
        let file: sharedtypes::FileObject = sharedtypes::FileObject {
            source: Some(sharedtypes::FileSource::Url(url)),
            hash: sharedtypes::HashesSupported::Md5(js["posts"][inc]["file"]["md5"].to_string()),
            tag_list: tags_list,
            skip_if: Vec::new(),
        };
        files.insert(file);
    }
    Ok(sharedtypes::ScraperObject {
        file: files,
        tag: HashSet::new(),
        flag: vec![],
    })
    //return Ok(vecvecstr);
}
///
/// Should this scraper handle anything relating to downloading.
///
#[no_mangle]
pub fn scraper_download_get() -> bool {
    false
}

fn gen_source_from_md5_ext(md5: &String, ext: &String, site: &Site) -> String {
    let lowercase_site = site_to_string(site).to_lowercase();
    let base = format!("https://static1.{}.net/data", lowercase_site);

    format!("{}/{}/{}/{}.{}", base, &md5[0..2], &md5[2..4], &md5, ext)
}
#[path = "../../../src/scr/intcoms/client.rs"]
mod client;

pub fn db_upgrade_call_3(site: &Site) {
    dbg!("{} GOING TO LOCK DB DOES THIS WORKY", site_to_string(site));
    client::load_table(sharedtypes::LoadDBTable::All);

    // Loads all fileids into memory
    let mut file_ids = client::file_get_list_id();

    // Gets namespace id from poolid
    let pool_nsid = match client::namespace_get(nsobjplg(&NsIdent::PoolId, site).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::PoolId, site).name,
            nsobjplg(&NsIdent::PoolId, site).description,
        ),
    };
    // Gets namespace id from poolid
    let poolposition_nsid = match client::namespace_get(nsobjplg(&NsIdent::PoolPosition, site).name)
    {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::PoolPosition, site).name,
            nsobjplg(&NsIdent::PoolPosition, site).description,
        ),
    };

    // Gets e6id from db
    let fileid_nsid = match client::namespace_get(nsobjplg(&NsIdent::FileId, site).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::FileId, site).name,
            nsobjplg(&NsIdent::FileId, site).description,
        ),
    }; // Gets e6's parent ids from db
    let parent_nsid = match client::namespace_get(nsobjplg(&NsIdent::Parent, site).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::Parent, site).name,
            nsobjplg(&NsIdent::Parent, site).description,
        ),
    }; // Gets e6's children id's from db
    let children_nsid = match client::namespace_get(nsobjplg(&NsIdent::Children, site).name) {
        Some(id) => id,
        None => client::namespace_put(
            nsobjplg(&NsIdent::Children, site).name,
            nsobjplg(&NsIdent::Children, site).description,
        ),
    };

    // Loads all tagid's that are attached to the pool
    let pool_table = client::namespace_get_tagids(pool_nsid);
    // Gets namespace id from source urls ensures that we're only working on e621 files
    let sourceurl_nsid = match client::namespace_get("source_url".to_string()) {
        Some(id) => id,
        None => client::namespace_put("source_url".to_string(), None),
    };

    // Loads all tagid's that are attached to the e621 sources
    let sourceurl_table = client::namespace_get_tagids(sourceurl_nsid);

    // Loads all tagid's that are attached to the parents sources
    let parent_table = client::namespace_get_tagids(parent_nsid); // Loads all tagid's that are attached to the children sources
    let children_table = client::namespace_get_tagids(children_nsid); // Loads all tagid's that are attached to the position
    let position_table = client::namespace_get_tagids(poolposition_nsid);

    client::log(format!(
        "{} Scraper-Starting to strip: {} fileids from processing list",
        site_to_string(site),
        file_ids.len()
    ));
    let mut cnt = 0;
    // Removes all fileids where the source is not e621
    for each in sourceurl_table {
        if let Some(tag) = client::tag_get_id(each) {
            let lowercase_site = site_to_string(site).to_lowercase();
            if !tag.name.contains(&format!("{}.net", lowercase_site)) {
                for fid in client::relationship_get_fileid(each).iter() {
                    file_ids.remove(fid);
                    cnt += 1;
                }
            }
        }
    }

    client::log(format!(
        "{} Scraper-Stripped: {} fileids from processing list",
        site_to_string(site),
        cnt
    ));
    for each in file_ids {
        let tids = client::relationship_get_tagid(each);
        for tid in tids.intersection(&pool_table) {
            dbg!(tid);
            let mut vec_poolpos = Vec::new();
            let mut hashset_fileid = HashSet::new();
            for each in client::parents_get(crate::client::types::ParentsType::Tag, *tid) {
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

            /*for position in vec_poolpos.iter() {
                /*if let Some(tag_id) =
                    client::parents_get(crate::client::types::ParentsType::Rel, *position)
                {
                }*/
            }*/
            for fid in hashset_fileid.iter() {
                let mut tag_id = client::parents_get(crate::client::types::ParentsType::Rel, *fid);

                // Removes the parents and children from tag_ids
                for tid_iter in tag_id.clone().iter() {
                    if parent_table.contains(tid_iter) || children_table.contains(tid_iter) {
                        tag_id.remove(tid_iter);
                    }
                }

                match tag_id.len().cmp(&2) {
                    std::cmp::Ordering::Less => {
                        dbg!("LESS 2 ITEMS IN HERE", tag_id);
                        dbg!(&fid, tid);
                    }

                    std::cmp::Ordering::Equal => {
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
                            if position_table.contains(tid_iter) {
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
                    std::cmp::Ordering::Greater => {
                        // Clears sub items and add job to db to
                        // scrape
                        dbg!("MORE THEN 2 ITEMS IN HERE", &tag_id);
                        dbg!(&fid, tid);
                        for tid_iter in tag_id.iter() {
                            if pool_table.contains(tid_iter) {
                                let mut job = sharedtypes::return_default_jobsobj();
                                job.site = site_to_string_prefix(site);
                                job.param = vec![sharedtypes::ScraperParam::Url(format!(
                                    "https://{}.net/pools.json?search[id]={}",
                                    site_to_string(site).to_lowercase(),
                                    client::tag_get_id(*tid_iter).unwrap().name
                                ))];
                                job.jobmanager = sharedtypes::DbJobsManager {
                                    jobtype: sharedtypes::DbJobType::Scraper,
                                    recreation: None,
                                };
                                client::job_add(job);
                                /*client::job_add(
                                    None,
                                    0,
                                    0,
                                    site_to_string_prefix(site),
                                    vec![sharedtypes::ScraperParam::Url(format!(
                                        "https://{}.net/pools.json?search[id]={}",
                                        site_to_string(site).to_lowercase(),
                                        client::tag_get_id(*tid_iter).unwrap().name
                                    ))],
                                    BTreeMap::new(),
                                    BTreeMap::new(),
                                    sharedtypes::DbJobsManager {
                                        jobtype: sharedtypes::DbJobType::Scraper,
                                        recreation: None,
                                    },
                                );*/
                            }
                            client::parents_delete(sharedtypes::DbParentsObj {
                                tag_id: *tid_iter,
                                relate_tag_id: *fid,
                                limit_to: None,
                            });
                        }
                    }
                }
            }
        }
    }

    client::transaction_flush();
}

#[no_mangle]
pub fn db_upgrade_call(db_version: &usize, site_struct: &sharedtypes::GlobalPluginScraper) {
    let mut site_op = None;

    if let Some(ref stored_info) = site_struct.stored_info {
        match stored_info {
            sharedtypes::StoredInfo::Storage(storage) => {
                for (key, val) in storage.iter() {
                    if key == "loaded_site" {
                        match string_to_site(val) {
                            None => {}
                            Some(site) => {
                                site_op = Some(site);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(site) = site_op {
        match db_version {
            3 => {
                db_upgrade_call_3(&site);
            }
            _ => {
                client::log_no_print(format!(
                    "{} No upgrade for version: {}",
                    site_to_string(&site),
                    db_version
                ));
            }
        }
    }
}

///
/// Determines what the fileid source is.
///
fn determine_site_type(
    file_id: &usize,
    source_url_nsid: &usize,
) -> Option<(Site, sharedtypes::DbTagNNS)> {
    for tagid in client::relationship_get_tagid(*file_id) {
        if let Some(tag) = client::tag_get_id(tagid) {
            if tag.namespace != *source_url_nsid {
                continue;
            } else {
                if tag.name.contains("e621.net") {
                    return Some((Site::E6, tag));
                }
                if tag.name.contains("e6ai.net") {
                    return Some((Site::E6AI, tag));
                }
            }
        }
    }
    None
}

///
/// Runs on startup of the software before any jobs run
///
#[no_mangle]
pub fn on_start(site_struct: &sharedtypes::GlobalPluginScraper) {
    return;
    let mut site_op = None;

    if let Some(ref stored_info) = site_struct.stored_info {
        match stored_info {
            sharedtypes::StoredInfo::Storage(storage) => {
                for (key, val) in storage.iter() {
                    if key == "loaded_site" {
                        match string_to_site(val) {
                            None => {}
                            Some(site) => {
                                if val.contains("E6AI") {
                                    return;
                                }
                                site_op = Some(site);
                            }
                        }
                    }
                }
            }
        }
    }

    //This POS migrates the old generic tags into a namespaced tag system for each site
    if let Some(site) = site_op {
        client::load_table(sharedtypes::LoadDBTable::Namespace);

        let source_url_nsid = match client::namespace_get("source_url".to_string()) {
            Some(out) => out,
            None => {
                return;
            }
        };

        let mut legacy_id_id = HashMap::new();

        let mut legacy_ns = [
            //("Pool_Updated_At", NsIdent::PoolUpdatedAt, None),
            //("Pool_Created_At", NsIdent::PoolCreatedAt, None),
            //("Pool_Id", NsIdent::PoolId, None),
            //("Pool_Creator", NsIdent::PoolCreator, None),
            //("Pool_Creator_Id", NsIdent::PoolCreatorId, None),
            //("Pool_Name", NsIdent::PoolName, None),
            //("Pool_Description", NsIdent::PoolDescription, None),
            //("Pool_Position", NsIdent::PoolPosition, None),
            ("General", NsIdent::General, None),
            ("Species", NsIdent::Species, None),
            ("Character", NsIdent::Character, None),
            ("Contributor", NsIdent::Contributor, None),
            ("Copyright", NsIdent::Copyright, None),
            ("Artist", NsIdent::Artist, None),
            ("Lore", NsIdent::Lore, None),
            ("Meta", NsIdent::Meta, None),
            ("Sources", NsIdent::Sources, None),
            //("Children", NsIdent::Children, None),
            //("Parent_id", NsIdent::Parent, None),
            ("Description", NsIdent::Description, None),
            ("Rating", NsIdent::Rating, None),
            //("Id", NsIdent::FileId, None),
            ("franchise", NsIdent::Franchise, None),
            ("Director", NsIdent::Director, None),
        ];

        for (nsname, ref nsident, ref mut id) in legacy_ns.iter_mut() {
            let nso = nsobjplg(nsident, &site);
            if let Some(old_ns_id) = client::namespace_get(nsname.to_string()) {
                *id = Some(client::namespace_put(
                    nso.name.clone(),
                    nso.description.clone(),
                ));
                legacy_id_id.insert(
                    old_ns_id,
                    client::namespace_put(nso.name.clone(), nso.description.clone()),
                );
            }
        }

        let mut nsids = Vec::new();
        for (ns, nsident, main_nsid) in legacy_ns.iter() {
            if main_nsid.is_none() {
                continue;
            }

            let mut cnt = 0;

            let max_b4_commit = 3000;

            client::log(format!(
                "E6-Scraper: Will commit every {max_b4_commit} tag migrations"
            ));

            //dbg!(ns, nsident, main_nsid);
            if let Some(nsid) = client::namespace_get(ns.to_string()) {
                client::log(format!("Parsing legacy NS: {ns}"));
                client::load_table(sharedtypes::LoadDBTable::Tags);
                client::load_table(sharedtypes::LoadDBTable::All);
                for tagid in client::namespace_get_tagids(nsid) {
                    //dbg!(&tagid);
                    let fileids = client::relationship_get_fileid(tagid);
                    for fileid in fileids.iter() {
                        //dbg!(fileid);
                        if let Some((site, og_tag)) = determine_site_type(fileid, &source_url_nsid)
                        {
                            //dbg!(&site, &og_tag);
                            for tagid in client::relationship_get_tagid(*fileid) {
                                if let Some(tag) = client::tag_get_id(tagid) {
                                    if let Some(legacy_ns) = legacy_id_id.get(&tag.namespace) {
                                        cnt += 1;

                                        if cnt >= max_b4_commit {
                                            client::log(format!(
                                                "E6-Scraper: Fileid: {} Moving tag: {} from Namespace: {} to Namespace: {}",
                                                &fileid, &tag.name, tag.namespace, legacy_ns
                                            ));
                                            client::transaction_flush();
                                            cnt = 0;
                                        } else {
                                            client::log_no_print(format!(
                                                "E6-Scraper: Fileid: {} Moving tag: {} from Namespace: {} to Namespace: {}",
                                                &fileid, &tag.name, tag.namespace, legacy_ns
                                            ));
                                        }
                                        let tagid_new =
                                            client::tag_add(tag.name, *legacy_ns, true, None);

                                        client::file_relationship_migrate(
                                            *fileid, tagid, tagid_new,
                                        );
                                    }
                                }
                            }
                            /*let old_tag = client::tag_get_id(tagid).unwrap();
                            let new_tag_id =
                                client::tag_add(old_tag.name, main_nsid.unwrap(), true, None);
                            client::relationship_add(*fileid, new_tag_id);
                            client::relationship_remove(*fileid, tagid);*/
                        } else {
                            dbg!(fileid, tagid, nsid);
                        }
                    }
                }
                nsids.push(nsid);
            }
        }
        client::transaction_flush();
        client::log_no_print("E6-Scraper: Finished processing migrations".to_string());
    }
}
