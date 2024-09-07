use base64::Engine;
use std::collections::HashSet;
use std::str::FromStr;
use std::time::Duration;
use strum_macros::EnumString;

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
#[derive(Debug, PartialEq, Eq, EnumString)]
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
            write!(f, "3")
        } else {
            write!(f, "{}", format!("{:?}", self).to_lowercase())
        }
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

pub struct InternalScraper {
    _version: usize,
    _name: String,
    _sites: Vec<String>,
    _ratelimit: (u64, Duration),
    _type: sharedtypes::ScraperType,
}

impl Default for InternalScraper {
    fn default() -> Self {
        Self::new()
    }
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
/// TODO Not implemented yet
///
#[no_mangle]
pub fn url_get(_params: &Vec<sharedtypes::ScraperParam>) -> Vec<String> {
    Vec::new()
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

    if let Some((board_codes, search_term)) = filter_boardcodes(params) {
        for cnt in 0..board_codes.len() {
            scraper_data.user_data.insert(
                format!("key_board_{cnt}"),
                format!("{}", board_codes.get(cnt).unwrap()),
            );
            scraper_data.user_data.insert(
                format!("key_search_{cnt}"),
                search_term.get(cnt).unwrap().to_string(),
            );
        }
    }

    (gen_url_catalog(params), scraper_data)
}
fn filter_boardcodes(
    params: &[sharedtypes::ScraperParam],
) -> Option<(Vec<BoardCodes>, Vec<String>)> {
    let mut params_boardcodes = Vec::new();
    //let mut params_query = Vec::new();
    let mut params_storage = Vec::new();
    for each in params.iter() {
        if each.param_type == sharedtypes::ScraperParamType::Normal {
            if let Ok(boardcode) = BoardCodes::from_str(&each.param_data.to_uppercase()) {
                params_boardcodes.push(boardcode);
            } else {
                params_storage.push(each.param_data.to_string());
            }
        }
    }

    if params_boardcodes.len() == params_storage.len() {
        Some((params_boardcodes, params_storage))
    } else {
        None
    }
}

///
/// Generates a catalog url
///
fn gen_url_catalog(params: &[sharedtypes::ScraperParam]) -> Vec<String> {
    let catalog_base = "https://a.4cdn.org/";
    let catalog_end = "/catalog.json";
    let mut params = params.to_owned();
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
    (sharedtypes::ScraperType::Manual, String::new())
}
///
/// Gets url to query cookie url.
/// Not used or implemented corrently. :D
///
#[no_mangle]
pub fn cookie_url() -> String {
    "e6scraper_cookie".to_string()
}

enum Nsid {
    PostId,
    PostComment,
    PostTimestamp,
    ThreadId,
    AttachmentName,
    OriginalMD5,
}

fn nsout(inp: &Nsid) -> sharedtypes::GenericNamespaceObj {
    match inp {
        Nsid::PostId => sharedtypes::GenericNamespaceObj {
            name: "4chan_Post_Id".to_string(),
            description: Some("A 4chan's post id, is unique".to_string()),
        },
        Nsid::PostTimestamp => sharedtypes::GenericNamespaceObj {
            name: "4chan_Post_Timestamp".to_string(),
            description: Some("A 4chan's post's timestamp UNIX style".to_string()),
        },
        Nsid::ThreadId => sharedtypes::GenericNamespaceObj {
            name: "4chan_Thread_ID".to_string(),
            description: Some("The thread ID from 4chan".to_string()),
        },
        Nsid::PostComment => sharedtypes::GenericNamespaceObj {
            name: "4chan_Post_Comment".to_string(),
            description: Some("A comment attached to a post".to_string()),
        },
        Nsid::AttachmentName => sharedtypes::GenericNamespaceObj {
            name: "4chan_Attachment_Name".to_string(),
            description: Some("The original name of an atachment that was uploaded".to_string()),
        },Nsid::OriginalMD5 => sharedtypes::GenericNamespaceObj {
            name: "4chan_Polish_Original_MD5".to_string(),
            description: Some("The original MD5 of the image before CF Polish tampered with this. I cannot find a way to bypass or to do other naughty things to it to get the original image".to_string()),
        },

    }
}

fn gen_fileurl(boardcode: String, filename: String, fileext: String) -> String {
    format!("https://i.4cdn.org/{}/{}{}", boardcode, filename, fileext)
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

    if let Some(jobtype) = actual_params.user_data.get("JobType") {
        let jobtype_str: &str = jobtype;
        match jobtype_str {
            "Thread" => {
                if let Ok(chjson) = json::parse(params) {
                    let thread = sharedtypes::TagObject {
                        namespace: nsout(&Nsid::ThreadId),
                        tag: actual_params.user_data.get("ThreadID").unwrap().to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: None,
                    };
                    out.tag.insert(thread);
                    let subthread = sharedtypes::SubTag {
                        namespace: nsout(&Nsid::ThreadId),
                        tag: actual_params.user_data.get("ThreadID").unwrap().to_string(),
                        tag_type: sharedtypes::TagType::Normal,
                        limit_to: None,
                    };

                    for each in chjson["posts"].members() {
                        // Gets information about post associates it to the thread
                        if let Some(comment) = each["com"].as_str() {
                            out.tag.insert(sharedtypes::TagObject {
                                namespace: nsout(&Nsid::PostComment),
                                tag: comment.to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: Some(subthread.clone()),
                            });
                        }
                        if let Some(comment) = each["no"].as_usize() {
                            out.tag.insert(sharedtypes::TagObject {
                                namespace: nsout(&Nsid::PostId),
                                tag: comment.to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: Some(subthread.clone()),
                            });
                        }
                        if let Some(comment) = each["time"].as_usize() {
                            out.tag.insert(sharedtypes::TagObject {
                                namespace: nsout(&Nsid::PostTimestamp),
                                tag: comment.to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: Some(subthread.clone()),
                            });
                        }

                        // If we have a file name then we should download it
                        if let Some(attachment_filename) = each["tim"].as_usize() {
                            let post_tag = sharedtypes::SubTag {
                                namespace: nsout(&Nsid::PostId),
                                tag: each["no"].as_usize().unwrap().to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                limit_to: None,
                            };

                            let mut tag_list = Vec::new();
                            tag_list.push(sharedtypes::TagObject {
                                namespace: nsout(&Nsid::AttachmentName),
                                tag: attachment_filename.to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: Some(post_tag.clone()),
                            });

                            let attachment_md5 = hex::encode(
                                base64::prelude::BASE64_STANDARD
                                    .decode(each["md5"].as_str().unwrap())
                                    .unwrap(),
                            );
                            tag_list.push(sharedtypes::TagObject {
                                namespace: nsout(&Nsid::OriginalMD5),
                                tag: attachment_md5.to_string(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: Some(post_tag.clone()),
                            });

                            let skip = sharedtypes::TagObject {
                                namespace: sharedtypes::GenericNamespaceObj {
                                    name: "FileHash-MD5".to_string(),
                                    description: None,
                                },
                                tag: attachment_md5.clone(),
                                tag_type: sharedtypes::TagType::Normal,
                                relates_to: None,
                            };

                            let bcode: BoardCodes = BoardCodes::from_str(
                                &actual_params
                                    .user_data
                                    .get("key_board_0")
                                    .unwrap()
                                    .to_uppercase(),
                            )
                            .unwrap();

                            // /b is protected by cloudflare polish cannot pull down "unoptimized"
                            // images from it
                            let hash = match bcode {
                                BoardCodes::B => None,
                                _ => Some(sharedtypes::HashesSupported::Md5(attachment_md5)),
                            };

                            let source_url = gen_fileurl(
                                bcode.to_string(),
                                attachment_filename.to_string(),
                                each["ext"].as_str().unwrap().to_string(),
                            );
                            let file = sharedtypes::FileObject {
                                tag_list,
                                skip_if: vec![skip],
                                source_url: Some(source_url),
                                hash,
                            };
                            out.file.insert(file);
                        }
                    }
                }
            }
            _ => {
                dbg!("CANNOT FIND JOBTYPE");
            }
        }
    }

    if let Ok(chjson) = json::parse(params) {
        for each in chjson.members() {
            for thread in each["threads"].members() {
                let mut cnt = 0;
                while let Some(key) = scraper_data.user_data.get(&format!("key_search_{cnt}")) {
                    if thread["com"].to_string().contains(key) {
                        //dbg!(&thread["com"]);
                        let threadurl =
                            format!("https://a.4cdn.org/b/thread/{}.json", thread["no"]);
                        if !scraper_data.user_data.contains_key("Stop") {
                            let mut usr_data = scraper_data.user_data.clone();
                            usr_data.insert("JobType".to_string(), "Thread".to_string());
                            usr_data.insert("ThreadID".to_string(), format!("{}", thread["no"]));
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
                                        user_data: usr_data,
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
    scraper_data
        .user_data
        .insert("Stop".to_string(), "Stop".to_string());
    Ok((out, scraper_data))
}
///
/// Should this scraper handle anything relating to downloading.
///
#[no_mangle]
pub fn scraper_download_get() -> bool {
    false
}
