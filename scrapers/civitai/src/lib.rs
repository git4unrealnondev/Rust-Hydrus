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

#[path = "../../../src/sharedtypes.rs"]
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
    CivitModelId,
    CivitModelVers,
    CivitModelVersName,
    CivitModelName,
    CivitModelDescription,
    CivitModelType,
    CivitModelTags,
    CivitModelUploadTimestamp,
    CivitModelBaseModelName,
    CivitModelTrainedWords,

    CivitPostId,
    CivitPostTitle,

    CivitImageId,
    CivitImagePosition,
    CivitImageMD5,
    CivitImageTags,
    CivitImagePrompt,
    CivitImagePromptNegative,
    CivitImageMetadata,
    CivitImageResources,

    CivitCommentId,
    CivitCommentCreationTimestamp,
    CivitCommentContent,

    CivitUserId,
    CivitUserName,
    CivitUserLinks,
    DONOTPARSE,
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

fn nsobjplg(name: &NsIdent) -> sharedtypes::GenericNamespaceObj {
    match name {
        NsIdent::DONOTPARSE => sharedtypes::GenericNamespaceObj {
            name: "DONOTPARSE".to_string(),
            description: Some(
                "Should never be parsed used only for passing new urls to scrape".to_string(),
            ),
        },
        NsIdent::CivitImageId => sharedtypes::GenericNamespaceObj {
            name: "CivitImageId".to_string(),
            description: Some(
                "Used by civitai to identify the image. Unique per upload to site.".to_string(),
            ),
        },
        NsIdent::CivitImageMD5 => sharedtypes::GenericNamespaceObj {
            name: "CivitImageMd5".to_string(),
            description: Some("Hash of file uploaded stored in compressed MD5.".to_string()),
        },
        NsIdent::CivitImageTags => sharedtypes::GenericNamespaceObj {
            name: "CivitFileTags".to_string(),
            description: Some("Tags used by civit to describe a file".to_string()),
        },
        NsIdent::CivitImagePrompt => sharedtypes::GenericNamespaceObj {
            name: "CivitImagePrompt".to_string(),
            description: Some("Civit: Prompt used to generate an image.".to_string()),
        },
        NsIdent::CivitImagePromptNegative => sharedtypes::GenericNamespaceObj {
            name: "CivitImagePromptNegative".to_string(),
            description: Some(
                "Civit: Negative items to help generate an image properly.".to_string(),
            ),
        },
        NsIdent::CivitImageMetadata => sharedtypes::GenericNamespaceObj {
            name: "CivitImageMetadata".to_string(),
            description: Some("Civit: Metadata about image".to_string()),
        },
        NsIdent::CivitImageResources => sharedtypes::GenericNamespaceObj {
            name: "CivitImageResources".to_string(),
            description: Some("Resources that were used to generate an image".to_string()),
        },
        NsIdent::CivitCommentId => sharedtypes::GenericNamespaceObj {
            name: "CivitCommentId".to_string(),
            description: Some("The ID of a comment from civitai".to_string()),
        },
        NsIdent::CivitCommentContent => sharedtypes::GenericNamespaceObj {
            name: "CivitCommentContent".to_string(),
            description: Some("The text from an comment".to_string()),
        },
        NsIdent::CivitCommentCreationTimestamp => sharedtypes::GenericNamespaceObj {
            name: "CivitCommentCreationTimestamp".to_string(),
            description: Some("The timestamp of when the comment was posted".to_string()),
        },
        NsIdent::CivitUserId => sharedtypes::GenericNamespaceObj {
            name: "CivitUserId".to_string(),
            description: Some("The unique ID of a user".to_string()),
        },
        NsIdent::CivitUserName => sharedtypes::GenericNamespaceObj {
            name: "CivitUserName".to_string(),
            description: Some("The Username of a user from civit".to_string()),
        },
        NsIdent::CivitUserLinks => sharedtypes::GenericNamespaceObj {
            name: "CivitUserLinks".to_string(),
            description: Some("Links to other user's page".to_string()),
        },
        NsIdent::CivitModelId => sharedtypes::GenericNamespaceObj {
            name: "CivitModelId".to_string(),
            description: Some("Civit unique model of ai gen".to_string()),
        },
        NsIdent::CivitModelTrainedWords => sharedtypes::GenericNamespaceObj {
            name: "CivitModelTrainedWords".to_string(),
            description: Some("unique words that change the output? of the image".to_string()),
        },
        NsIdent::CivitModelTags => sharedtypes::GenericNamespaceObj {
            name: "CivitModelTags".to_string(),
            description: Some("Tags used by civit to describe a AI Model".to_string()),
        },

        NsIdent::CivitModelBaseModelName => sharedtypes::GenericNamespaceObj {
            name: "CivitModelBaseModelName".to_string(),
            description: Some("The base models name. IE SDXL 1.0 points to SDXL".to_string()),
        },

        NsIdent::CivitModelDescription => sharedtypes::GenericNamespaceObj {
            name: "CivitModelDescription".to_string(),
            description: Some("The Desicription of the model".to_string()),
        },
        NsIdent::CivitModelUploadTimestamp => sharedtypes::GenericNamespaceObj {
            name: "CivitModelUploadTimestamp".to_string(),
            description: Some("The timestamp in which the model was uploaded to civit".to_string()),
        },

        NsIdent::CivitModelVers => sharedtypes::GenericNamespaceObj {
            name: "CivitModelVers".to_string(),
            description: Some("The version of the model being used".to_string()),
        },
        NsIdent::CivitModelVersName => sharedtypes::GenericNamespaceObj {
            name: "CivitModelVersName".to_string(),
            description: Some("The versions name".to_string()),
        },

        NsIdent::CivitModelName => sharedtypes::GenericNamespaceObj {
            name: "CivitModelName".to_string(),
            description: Some("Civit Name of model".to_string()),
        },
        NsIdent::CivitModelType => sharedtypes::GenericNamespaceObj {
            name: "CivitModelType".to_string(),
            description: Some("Civit type of model".to_string()),
        },
        NsIdent::CivitPostId => sharedtypes::GenericNamespaceObj {
            name: "CivitPostId".to_string(),
            description: Some("The post id of a set of images".to_string()),
        },
        NsIdent::CivitPostTitle => sharedtypes::GenericNamespaceObj {
            name: "CivitPostTitle".to_string(),
            description: Some("The post's title.".to_string()),
        },
        NsIdent::CivitImagePosition => sharedtypes::GenericNamespaceObj {
            name: "CivitImagePosition".to_string(),
            description: Some("The image's position in the pool".to_string()),
        }, /*
                       => sharedtypes::GenericNamespaceObj {
           name: "".to_string(),
                           description: Some("".to_string())
                       }*/
    }
}

fn md5_decode(inp: &str) -> String {
    let attachment_md5 = hex::encode(base64::prelude::BASE64_STANDARD.decode(inp).unwrap());
    attachment_md5
}

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[no_mangle]
pub fn new() -> Vec<sharedtypes::SiteStruct> {
    let out: Vec<sharedtypes::SiteStruct> = vec![sharedtypes::SiteStruct {
        name: "civitai".to_string(),
        sites: vec_of_strings!("civitai", "civitai.com"),
        version: 0,
        ratelimit: (1, Duration::from_secs(1)),
        should_handle_file_download: false,
        should_handle_text_scraping: false,
        login_type: vec![],
        stored_info: None,
    }];
    out
}
#[no_mangle]
pub fn test() -> u8 {
    0
}

#[derive(Serialize, Deserialize)]
pub enum Id {
    id(usize),
}
#[derive(Serialize, Deserialize)]
pub enum ModelById {
    json(Id),
}

#[derive(Serialize, Deserialize)]
pub struct GetInfinite {
    modelVersionId: usize,
    period: String,
    sort: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<usize>,
    pending: bool,
    cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub enum Cursor {
    cursor(String),
}
#[derive(Serialize, Deserialize)]
pub enum MetaCursor {
    cursor(Vec<String>),
}

#[derive(Serialize, Deserialize)]
pub enum Values {
    values(Cursor),
}
#[derive(Serialize, Deserialize)]
pub enum MetaValues {
    values(MetaCursor),
}

#[derive(Serialize, Deserialize)]
pub enum Meta {
    meta(Values),
}

#[derive(Serialize, Deserialize)]
pub struct ImageGetInfinite {
    json: GetInfinite,
    meta: Values,
}
#[derive(Serialize, Deserialize)]
pub struct MetaImageGetInfinite {
    json: GetInfinite,
    meta: MetaValues,
}

#[derive(Serialize, Deserialize)]
pub struct GetVotableTagsStorage {
    id: usize,
    r#type: String,
    authed: bool,
}
#[derive(Serialize, Deserialize)]
pub struct GetVotableTags {
    json: GetVotableTagsStorage,
}
#[derive(Serialize, Deserialize)]
pub enum VotableTagsType {
    Image,
    Model,
}

pub fn model_by_id(modelnumber: &usize) -> String {
    let jsonmodel = ModelById::json(Id::id(*modelnumber));
    let jsondata = serde_json::to_string(&jsonmodel).unwrap();
    format!(
        "https://civitai.com/api/trpc/model.getById?input={}",
        jsondata
    )
}

///
/// Formats the Url
/// Had to do a hacky work around due to the meta cursor being a vector
///
pub fn image_get_infinite(modelnumber: &usize, cursor: Option<String>) -> String {
    let out = match cursor {
        None => {
            let metavalues =
                MetaValues::values(MetaCursor::cursor(["undefined".to_string()].to_vec()));
            let imageinfinite = MetaImageGetInfinite {
                json: GetInfinite {
                    modelVersionId: *modelnumber,
                    period: "AllTime".to_owned(),
                    sort: "Most Reactions".to_owned(),
                    limit: Some(20),
                    pending: true,
                    cursor: None,
                },
                meta: metavalues,
            };
            let jsondata = serde_json::to_string(&imageinfinite).unwrap();
            format!(
                "https://civitai.com/api/trpc/image.getInfinite?input={}",
                jsondata
            )
        }
        Some(cursor) => {
            let values = Values::values(Cursor::cursor(cursor.clone()));
            let imageinfinite = ImageGetInfinite {
                json: GetInfinite {
                    modelVersionId: *modelnumber,
                    period: "AllTime".to_owned(),
                    sort: "Most Reactions".to_owned(),
                    limit: Some(20),
                    pending: true,
                    cursor: Some(cursor),
                },
                meta: values,
            };
            let jsondata = serde_json::to_string(&imageinfinite).unwrap();
            format!(
                "https://civitai.com/api/trpc/image.getInfinite?input={}",
                jsondata
            )
        }
    };

    return out;
}

///
/// Generates a image url from a name and a "url" looks like it's just a storage hash but that's
/// what they call it in civits internal generator it looks like
///
pub fn image_gen_url(name: &String, url: &String) -> String {
    format!(
        "https://image.civitai.com/xG1nkqKTMzGDvpLrqFT7WA/{}/original=true/{}",
        url, name
    )
}

///
/// Handles individual images
///
pub fn image_parsing(
    inp: &String,
    files: &mut HashSet<sharedtypes::FileObject>,
    tags: &mut HashSet<sharedtypes::TagObject>,
    scraperdata: &sharedtypes::ScraperData,
) {
    let unchecked = json::parse(inp);
    if unchecked.is_err() {
        return;
    }
    let data = &unchecked.unwrap()["result"]["data"]["json"];

    let file_id_temp = data["id"].as_usize().unwrap();

    let file_source_url = image_gen_url(
        &data["name"].as_str().unwrap().to_string(),
        &data["url"].as_str().unwrap().to_string(),
    );

    let user = &data["user"];

    let user_idtemp = user["id"].as_number().unwrap().to_string();
    let user_id = user_idtemp.as_str();
    let user_username = user["username"].as_str().unwrap();

    let mut scraperdatafile = scraperdata.clone();
    scraperdatafile
        .user_data
        .insert("file_source_url".to_string(), file_source_url.clone());
    scraperdatafile
        .user_data
        .insert("task".to_string(), "tagsgetimage".to_string());
    scraperdatafile
        .user_data
        .insert("user_id".to_string(), user_id.to_string());
    scraperdatafile
        .user_data
        .insert("user_username".to_string(), user_username.to_string());

    scraperdatafile.job = sharedtypes::JobScraper {
        site: scraperdata.job.site.clone(),
        param: Vec::new(),
        //original_param: get_votable_tags_url(file_id_temp, VotableTagsType::Image),
        job_type: sharedtypes::DbJobType::Scraper,
    };
    let tag = sharedtypes::TagObject {
        namespace: nsobjplg(&NsIdent::DONOTPARSE),
        tag: get_votable_tags_url(file_id_temp, VotableTagsType::Image),
        tag_type: sharedtypes::TagType::ParseUrl((scraperdatafile, None)),
        relates_to: None,
    };
    tags.insert(tag);
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
    panic!();
    dbg!(params, scraperdata);
    let urltype = Some("pass");
    //let urltype = get_urltype(&scraperdata.job.original_param);
    if let Some(urltype) = urltype {
        match urltype {
            UrlType::Models((model_number, model_version)) => {
                //if let Some(model_version) = model_version {
                //    ret.push(model_by_id(&model_version));
                //}
                //ret.push(model_by_id(&model_number));
                //println!("{}", image_get_infinite(&model_number, None));

                let active_id = if let Some(version) = model_version {
                    version
                } else {
                    model_number
                };

                // Setup for getting model data.
                let mut scraperdataclone = scraperdata.clone();
                let mut userdata = BTreeMap::new();
                userdata.insert("task".to_string(), "modelbyid".to_string());
                userdata.insert("model_number".to_string(), model_number.to_string());
                scraperdataclone.user_data = userdata;
                ret.push((model_by_id(&model_number), scraperdataclone));

                // Setup for infinite images
                let mut scraperdataclone = scraperdata.clone();
                let mut userdata = BTreeMap::new();
                userdata.insert("task".to_string(), "imagegetinfinite".to_string());
                userdata.insert("model_number".to_string(), active_id.to_string());
                scraperdataclone.user_data = userdata;
                ret.push((image_get_infinite(&active_id, None), scraperdataclone));
            }
            UrlType::Images(image_number) => {
                dbg!(&image_number);
                let mut scraperdataclone = scraperdata.clone();
                let mut userdata = BTreeMap::new();
                let mut scraperdataclone = scraperdata.clone();
                userdata.insert("task".to_string(), "imagegetone".to_string());
                userdata.insert("image_number".to_string(), image_number.to_string());
                scraperdataclone.user_data = userdata;

                ret.push((image_gen_url_one(&image_number), scraperdataclone))
            }
        }
    }
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

pub fn get_urltype(inp: &String) -> Option<UrlType> {
    // Captures the models directory
    let re = Regex::new(r"(?:(?:(?:http:\/\/|https:\/\/|www\.)civitai\.com\/models\/)([0-9]*))(?:.*(?:/?modelVersionId=)([0-9]*))?").unwrap();
    if let Some(caps) = re.captures(inp) {
        let modelno;
        if let Some(cap) = caps.get(1) {
            if let Ok(num) = cap.as_str().parse::<usize>() {
                modelno = num;
            } else {
                return None;
            }
        } else {
            return None;
        };

        let modelver = if let Some(cap) = caps.get(2) {
            if let Ok(num) = cap.as_str().parse::<usize>() {
                Some(num)
            } else {
                None
            }
        } else {
            None
        };
        return Some(UrlType::Models((modelno, modelver)));
    }

    let re = Regex::new(r"(?:(?:(?:http:\/\/|https:\/\/|www\.)civitai\.com\/images\/)([0-9]*))")
        .unwrap();
    if let Some(caps) = re.captures(inp) {
        let modelno;
        if let Some(cap) = caps.get(1) {
            if let Ok(num) = cap.as_str().parse::<usize>() {
                modelno = num;
            } else {
                return None;
            }
        } else {
            return None;
        };

        return Some(UrlType::Images(modelno));
    }

    None
}
///
/// Generates the image info for one image
///
pub fn image_gen_url_one(id: &usize) -> String {
    let modelbyid = ModelById::json(Id::id(*id));
    format!(
        "https://civitai.com/api/trpc/image.get?input={}",
        serde_json::to_string(&modelbyid).unwrap()
    )
}

pub fn tags_image_parsing(
    inp: &String,
    files: &mut HashSet<sharedtypes::FileObject>,
    tags: &mut HashSet<sharedtypes::TagObject>,
    scraperdata: &sharedtypes::ScraperData,
) {
    let unchecked = json::parse(inp);
    if unchecked.is_err() {
        return;
    }
    let data = unchecked.unwrap();

    let mut tags_vec = Vec::new();
    for each in data["result"]["data"]["json"].members() {
        if let Some(tagname) = each["name"].as_str() {
            tags_vec.push(sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::CivitImageTags),
                tag: tagname.to_string(),
                relates_to: None,
                tag_type: sharedtypes::TagType::Normal,
            });
        }
    }

    if scraperdata.user_data.get("user_id").is_some()
        && scraperdata.user_data.get("user_username").is_some()
    {
        let user_id = scraperdata.user_data.get("user_id").unwrap();
        let user_username = scraperdata.user_data.get("user_username").unwrap();
        dbg!(&user_id, &user_username);
        // Gets the user id as a tag
        let useridsubtag = sharedtypes::SubTag {
            namespace: nsobjplg(&NsIdent::CivitUserId),
            tag: user_id.to_string(),
            limit_to: None,
            tag_type: sharedtypes::TagType::Normal,
        };

        // User Username Tag
        let userusernametag = sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::CivitUserName),
            tag: user_username.to_string(),
            relates_to: Some(useridsubtag.clone()),
            tag_type: sharedtypes::TagType::Normal,
        };
        tags_vec.push(userusernametag);
    }

    let file = sharedtypes::FileObject {
        source: Some( sharedtypes::FileSource::Url(
            scraperdata
                .user_data
                .get("file_source_url")
                .unwrap()
                .to_string(),
        )),
        hash: sharedtypes::HashesSupported::None,
        tag_list: tags_vec,
        skip_if: Vec::new(),
    };

    files.insert(file);
}

///
/// Manages parsing data from the models field
///
pub fn model_data_parsing(
    inp: &String,
    files: &mut HashSet<sharedtypes::FileObject>,
    tags: &mut HashSet<sharedtypes::TagObject>,
    scraperdata: &sharedtypes::ScraperData,
) {
    let unchecked = json::parse(inp);
    if unchecked.is_err() {
        return;
    }
    let data = unchecked.unwrap();
    let djson = &data["result"]["data"]["json"];
    let modelidtemp = match djson["id"].as_usize() {
        Some(modelid) => modelid,
        None => {
            return;
        }
    };
    let modelid = &modelidtemp.to_string();

    let modelloop = [
        ("description", nsobjplg(&NsIdent::CivitModelDescription)),
        ("type", nsobjplg(&NsIdent::CivitModelType)),
        ("publishedAt", nsobjplg(&NsIdent::CivitModelUploadTimestamp)),
    ];

    let modelidsubtag = sharedtypes::SubTag {
        namespace: nsobjplg(&NsIdent::CivitModelId),
        tag: modelid.to_string(),
        limit_to: None,
        tag_type: sharedtypes::TagType::Normal,
    };

    for (jsonsearch, genericnamespaceobject) in modelloop {
        let item = djson[jsonsearch].as_str();
        if item.is_none() {
            continue;
        }
        let item = item.unwrap();

        let tag = sharedtypes::TagObject {
            namespace: genericnamespaceobject,
            tag: item.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: Some(modelidsubtag.clone()),
        };
        tags.insert(tag);
    }

    let user = &djson["user"];

    let user_idtemp = user["id"].as_number().unwrap().to_string();
    let user_id = user_idtemp.as_str();
    let user_username = user["username"].as_str().unwrap();

    // Gets the user id as a tag
    let useridsubtag = sharedtypes::SubTag {
        namespace: nsobjplg(&NsIdent::CivitUserId),
        tag: user_id.to_string(),
        limit_to: None,
        tag_type: sharedtypes::TagType::Normal,
    };

    // User Username Tag
    let userusernametag = sharedtypes::TagObject {
        namespace: nsobjplg(&NsIdent::CivitUserName),
        tag: user_username.to_string(),
        relates_to: Some(useridsubtag.clone()),
        tag_type: sharedtypes::TagType::Normal,
    };
    tags.insert(userusernametag);

    // Associates the model with the user that uploaded it
    let modelidtag = sharedtypes::TagObject {
        namespace: nsobjplg(&NsIdent::CivitModelId),
        tag: modelid.to_string(),
        relates_to: Some(useridsubtag),
        tag_type: sharedtypes::TagType::Normal,
    };
    tags.insert(modelidtag);

    let userloop = [
        ("modelId", nsobjplg(&NsIdent::CivitModelId)),
        ("name", nsobjplg(&NsIdent::CivitModelName)),
        ("description", nsobjplg(&NsIdent::CivitModelDescription)),
        ("baseModel", nsobjplg(&NsIdent::CivitModelBaseModelName)),
    ];

    // Loops through JS items and if they exist then add them as a tag.
    for model in djson["modelVersions"].members() {
        for (jsonsearch, genericnamespaceobject) in userloop.clone() {
            let item = model[jsonsearch].as_str();
            if item.is_none() {
                continue;
            }
            let item = item.unwrap();

            let tag = sharedtypes::TagObject {
                namespace: genericnamespaceobject,
                tag: item.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(modelidsubtag.clone()),
            };
            tags.insert(tag);
            if let Some(modelid) = model["id"].as_usize() {
                let next_infinite_url = image_get_infinite(&modelid, None);

                let mut scraperdatafile = scraperdata.clone();
                scraperdatafile.job = sharedtypes::JobScraper {
                    site: scraperdata.job.site.clone(),
                    param: Vec::new(),
           //         original_param: next_infinite_url.clone(),
                    job_type: sharedtypes::DbJobType::Scraper,
                };
                scraperdatafile
                    .user_data
                    .insert("task".to_string(), "imagegetinfinite".to_string());

                let tag = sharedtypes::TagObject {
                    namespace: nsobjplg(&NsIdent::DONOTPARSE),
                    tag: next_infinite_url.clone(),
                    tag_type: sharedtypes::TagType::ParseUrl((scraperdatafile, None)),
                    relates_to: None,
                };
                tags.insert(tag);
            }
        }
    }
}

pub fn get_votable_tags_url(imageid: usize, datatype: VotableTagsType) -> String {
    let dtype = match datatype {
        VotableTagsType::Image => "image",
        VotableTagsType::Model => "model",
    };
    let data = GetVotableTags {
        json: GetVotableTagsStorage {
            id: imageid,
            r#type: dtype.to_string(),
            authed: true,
        },
    };

    format!(
        "https://civitai.com/api/trpc/tag.getVotableTags?input={}",
        serde_json::to_string(&data).unwrap()
    )
}

pub fn image_infinite_parsing(
    inp: &String,
    files: &mut HashSet<sharedtypes::FileObject>,
    tags: &mut HashSet<sharedtypes::TagObject>,
    scraperdata: &sharedtypes::ScraperData,
) {
    let unchecked = json::parse(inp);
    if unchecked.is_err() {
        return;
    }
    let data = &unchecked.unwrap()["result"]["data"]["json"];

    // Handles the next cursor object. Acts like a slide window
    if let Some(cursor) = data["nextCursor"].as_str() {
        let modelno: usize = scraperdata
            .user_data
            .get("model_number")
            .unwrap()
            .parse()
            .unwrap();
        let next_infinite_url = image_get_infinite(&modelno, Some(cursor.to_string()));
        let mut scraperdatafile = scraperdata.clone();
        scraperdatafile.job = sharedtypes::JobScraper {
            site: scraperdata.job.site.clone(),
            param: Vec::new(),
          //  original_param: next_infinite_url.clone(),
            job_type: sharedtypes::DbJobType::Scraper,
        };
        scraperdatafile
            .user_data
            .insert("task".to_string(), "imagegetinfinite".to_string());

        let tag = sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::DONOTPARSE),
            tag: next_infinite_url.clone(),
            tag_type: sharedtypes::TagType::ParseUrl((scraperdatafile, None)),
            relates_to: None,
        };
        //tags.insert(tag);
    }

    for file_item in data["items"].members() {
        let mut file_tags = Vec::new();
        let mut skip_if = Vec::new();
        let file_id_temp = file_item["id"].as_usize().unwrap();
        // Gets the specific model used to generate image
        // Sometimes the specific model is not given in
        let modelno = if file_item["modelVersionId"].is_null() {
            scraperdata.user_data.get("model_number").unwrap()
        } else {
            &file_item["modelVersionId"].as_usize().unwrap().to_string()
        };

        //Gets post info
        let postid_sub = sharedtypes::SubTag {
            namespace: nsobjplg(&NsIdent::CivitPostId),
            tag: file_item["postId"].as_usize().unwrap().to_string(),
            limit_to: None,
            tag_type: sharedtypes::TagType::Normal,
        };

        // Sometimes the posts don't have titles
        if let Some(postTitle) = file_item["postTitle"].as_str() {
            let posttitle = sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::CivitPostTitle),
                tag: postTitle.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(postid_sub),
            };
            tags.insert(posttitle);
        }

        let modelidvers_sub = sharedtypes::SubTag {
            namespace: nsobjplg(&NsIdent::CivitModelVers),
            tag: modelno.to_string(),
            limit_to: None,
            tag_type: sharedtypes::TagType::Normal,
        };

        let postid = sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::CivitPostId),
            tag: file_item["postId"].as_usize().unwrap().to_string(),
            tag_type: sharedtypes::TagType::Normal,
            relates_to: Some(modelidvers_sub),
        };
        let postid_tag = sharedtypes::Tag {
            namespace: nsobjplg(&NsIdent::CivitPostId),
            tag: file_item["postId"].as_usize().unwrap().to_string(),
        };

        tags.insert(postid);
        let imagepos_sub = sharedtypes::SubTag {
            namespace: nsobjplg(&NsIdent::CivitImagePosition),
            tag: file_item["index"].as_usize().unwrap().to_string(),
            limit_to: Some(postid_tag),
            tag_type: sharedtypes::TagType::Normal,
        };
        let file_id = sharedtypes::TagObject {
            namespace: nsobjplg(&NsIdent::CivitImageId),
            tag: file_id_temp.to_string(),
            relates_to: Some(imagepos_sub),
            tag_type: sharedtypes::TagType::Normal,
        };
        file_tags.push(file_id.clone());
        tags.insert(file_id);

        // Gets username info
        let userid_sub = sharedtypes::SubTag {
            namespace: nsobjplg(&NsIdent::CivitUserId),
            tag: file_item["user"]["id"].as_usize().unwrap().to_string(),
            limit_to: None,
            tag_type: sharedtypes::TagType::Normal,
        };
        if let Some(username_st) = file_item["user"]["username"].as_str() {
            let username = sharedtypes::TagObject {
                namespace: nsobjplg(&NsIdent::CivitUserName),
                tag: username_st.to_string(),
                tag_type: sharedtypes::TagType::Normal,
                relates_to: Some(userid_sub),
            };
            file_tags.push(username);
        }

        let file_name = match file_item["name"].as_str() {
            Some(name) => name,
            None => {
                let tmep = &file_item["id"].as_usize().unwrap();
                &tmep.to_string()
            }
        };

        let file_source_url = image_gen_url(
            &file_name.to_string(),
            &file_item["url"].as_str().unwrap().to_string(),
        );

        let mut scraperdatafile = scraperdata.clone();
        scraperdatafile
            .user_data
            .insert("file_source_url".to_string(), file_source_url.clone());
        scraperdatafile
            .user_data
            .insert("task".to_string(), "tagsgetimage".to_string());

        if file_item["tagIds"].is_array() {
            dbg!(file_item["tagIds"].members().len(),);
            let limit_num = file_item["tagIds"].members().len();
            if limit_num != 0 {
                skip_if.push(sharedtypes::SkipIf::FileNamespaceNumber((
                    sharedtypes::Tag {
                        namespace: nsobjplg(&NsIdent::CivitImageId),
                        tag: file_id_temp.to_string(),
                    },
                    nsobjplg(&NsIdent::CivitImageTags),
                    limit_num,
                )));

                scraperdatafile.job = sharedtypes::JobScraper {
                    site: scraperdata.job.site.clone(),
                    param: Vec::new(),
                   // original_param: get_votable_tags_url(file_id_temp, VotableTagsType::Image),
                    job_type: sharedtypes::DbJobType::Scraper,
                };

                let tag = sharedtypes::TagObject {
                    namespace: nsobjplg(&NsIdent::DONOTPARSE),
                    tag: get_votable_tags_url(file_id_temp, VotableTagsType::Image),
                    tag_type: sharedtypes::TagType::ParseUrl((
                        scraperdatafile,
                        Some(sharedtypes::SkipIf::FileNamespaceNumber((
                            sharedtypes::Tag {
                                namespace: nsobjplg(&NsIdent::CivitImageId),
                                tag: file_id_temp.to_string(),
                            },
                            nsobjplg(&NsIdent::CivitImageTags),
                            limit_num,
                        ))),
                    )),
                    relates_to: None,
                };
                tags.insert(tag);
            }
        }

        let file = sharedtypes::FileObject {
            source: Some(sharedtypes::FileSource::Url(file_source_url)),
            hash: sharedtypes::HashesSupported::None,
            tag_list: file_tags,
            skip_if: skip_if,
        };
        files.insert(file);
    }
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

    match scraperdata.user_data.get("task").unwrap().as_str() {
        "modelbyid" => {
            model_data_parsing(inputfromreqwest, &mut files, &mut tags, scraperdata);
        }
        "imagegetinfinite" => {
            image_infinite_parsing(inputfromreqwest, &mut files, &mut tags, scraperdata);
        }
        "tagsgetimage" => {
            tags_image_parsing(inputfromreqwest, &mut files, &mut tags, scraperdata);
        }
        "imagegetone" => {
            image_parsing(inputfromreqwest, &mut files, &mut tags, scraperdata);
        }
        _ => {}
    }

    Ok((
        sharedtypes::ScraperObject {
            file: files,
            tag: tags,
            flag: vec![]
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

#[path = "../../../src/client.rs"]
mod client;

#[no_mangle]
pub fn db_upgrade_call(db_version: &usize) {
    match db_version {
        _ => {
            client::log_no_print(format!("Civitai No upgrade for version: {}", db_version));
        }
    }
}
