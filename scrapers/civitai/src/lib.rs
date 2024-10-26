use base64::Engine;
use chrono::DateTime;
use regex::Regex;
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
    CivitModelId,
    CivitModelVers,
    CivitModelVersName,
    CivitModelName,
    CivitModelDescription,
    CivitModelType,
    CivitModelUploadTimestamp,
    CivitModelBaseModelName,
    CivitModelTrainedWords,

    CivitPostId,
    CivitPostTitle,

    CivitImageId,
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
        }, /*
                       => sharedtypes::GenericNamespaceObj {
           name: "".to_string(),
                           description: Some("".to_string())
                       }*/
    }
}

fn md5_decode(inp: &String) -> String {
    let attachment_md5 = hex::encode(base64::prelude::BASE64_STANDARD.decode(inp).unwrap());
    attachment_md5
}

impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0,
            _name: "civitai".to_string(),
            _sites: vec_of_strings!("civitai"),
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
) -> (Vec<String>, sharedtypes::ScraperData) {
    let mut ret = Vec::new();
    dbg!(params, scraperdata);
    get_urltype(&scraperdata.job.original_param);
    (ret, scraperdata.clone())
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

pub fn get_urltype(inp: &String) -> Option<String> {
    // Captures the models directory
    let re = Regex::new(r"(?:(?:(?:http:\/\/|https:\/\/|www\.)civitai\.com\/models\/)([0-9]*))(?:.*(?:/?modelVersionId=)([0-9]*))?").unwrap();
    if let Some(caps) = re.captures(inp) {
        dbg!(&caps);
        dbg!(&caps[0]);
        let modelno = if let Some(cap) = caps.get(1) {
            Some(cap.as_str())
        } else {
            None
        };

        let modelver = if let Some(cap) = caps.get(2) {
            Some(cap.as_str())
        } else {
            None
        };
        dbg!(modelno, modelver);
    }
    None
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

#[no_mangle]
pub fn db_upgrade_call(db_version: &usize) {
    match db_version {
        _ => {
            client::log_no_print(format!("Civitai No upgrade for version: {}", db_version));
        }
    }
}
