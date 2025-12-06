#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unexpected_cfgs)]

#[cfg(feature = "regex")]
use regex::Regex;

#[cfg(feature = "clap")]
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
#[cfg(feature = "clap")]
use strum_macros::EnumIter;
#[cfg(feature = "clap")]
use strum_macros::{Display, EnumString};

// Default priority for a scraper
pub const DEFAULT_PRIORITY: usize = 5;

// Default cache time for a job.
// If its None then do not cache the job
// If the cachetime is greq then zero we'll add the job to a local cache
// If the cachetime is zero it will be only valid for the job run
pub const DEFAULT_CACHETIME: Option<usize> = None;
pub const DEFAULT_CACHECHECK: JobCacheType = JobCacheType::TimeReptimeParam;

///
/// Job cache chcekr type. When a job gets added into a DB this field will determine what needs to be done
/// to check if we should add this into the DB.
///
/// Note a direct match is also performed and if the job is seen then we ignore it.
///
#[derive(
    Debug, Hash, Eq, PartialEq, Clone, Serialize, bincode::Encode, bincode::Decode, Deserialize,
)]
pub enum JobCacheType {
    // Checks the time, reptime and param fields. If these match other jobs then we don't add
    TimeReptimeParam,
    // Just checks the params field
    Param,
}
#[derive(
    Debug, Hash, Eq, PartialEq, Clone, Serialize, bincode::Encode, bincode::Decode, Deserialize,
)]

pub enum GreqLeqOrEq {
    GreaterThan,
    LessThan,
    Equal,
}
#[cfg_attr(feature = "clap", derive(Debug, Subcommand))]
pub enum CheckSourceUrlsEnum {
    /// Just print the suspected urls
    Print,
    /// Delete the bad suspect urls
    Delete,
}

///
/// Manages the conditions that determines which enclave should trigger
///
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum EnclaveCondition {
    Any,
    None,
    FileSizeGreater(usize),
    FileSizeLessthan(usize),
    TagNameAndNamespace((String, String)),
}

///
/// Manages the conditions that determines which enclave stop processing at
///
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum EnclaveStopCondition {
    /// We've found a location to put our file
    FileDownloadLocation,
}

#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum EnclaveAction {
    DownloadToLocation(usize),
    AddTagAndNamespace((String, GenericNamespaceObj, TagType, Option<SubTag>)),
    DownloadToDefault,
    //Functionally similar to DownloadToLocation however this is used for just putting an item in a
    //location and does not download it. Really just for nice logging :D
    PutAtDefault,
}

/// Database Tags Object
#[derive(
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Clone,
)]
pub struct DbTagNNS {
    pub name: String,
    pub namespace: usize,
}

/// Database Tags Object
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub struct DbTagObjCompatability {
    pub id: usize,
    pub name: String,
    pub namespace: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(EnumIter, Display))]
// Manages what the search can do.
pub enum Search {
    Fid(Vec<String>),
    Tid(Vec<String>),
    Tag(Vec<String>),
    Hash(Vec<String>),
}

#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(EnumIter, Display))]
pub enum Tasks {
    // CSV importation. cp mv hardlink
    Csv(String, CsvCopyMvHard),
    Remove(TasksRemove),
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "clap", derive(EnumIter, Display))]
pub enum TasksRemove {
    RemoveNamespaceId(usize),
    RemoveNamespaceString(String),
    #[default]
    None,
}

#[derive(
    Debug,
    Clone,
    Eq,
    Hash,
    PartialEq,
    Ord,
    PartialOrd,
    Deserialize,
    Serialize,
    bincode::Encode,
    bincode::Decode,
)]
///
/// Holds the login type that we need
///
pub enum LoginType {
    Cookie(String, Option<String>),
    Api(String, Option<String>),
    ApiNamespaced(String, Option<String>, Option<String>),
    Login(String, Option<(String, String)>),
    Other(String, Option<String>),
}
#[derive(
    Debug,
    Clone,
    Eq,
    Hash,
    PartialEq,
    Ord,
    PartialOrd,
    Deserialize,
    Serialize,
    bincode::Encode,
    bincode::Decode,
)]

///
/// Data storage for the login type needed to determine if we have to use this to access a site or
/// if its just a nice to have
///
pub enum LoginNeed {
    Required,
    Optional,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum StoredInfo {
    Storage(Vec<(String, String)>),
}

///
/// Info for scrapers as apart of the Global merge
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ScraperInfo {
    /// Ratelimit for this site
    pub ratelimit: (u64, std::time::Duration),
    pub sites: Vec<String>,
    pub priority: usize,
    // How many threads should we use to scrape a page. If none then use as many threads as on cpu
    pub num_threads: Option<usize>,
    pub modifiers: Vec<ScraperModifiers>,
}

///
/// Modifiers to add to a scraper job
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ScraperModifiers {
    // A useragent to use when scraping text or pulling siteinfo
    TextUseragent(String),
    // A useragent to use when downloading media
    MediaUseragent(String),
    //,Adds a header to a media download
    MediaHeader((String, String)),
}

///
/// Info for plugins as apart of the Global merge
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct PluginInfo2 {
    pub com_channel: bool,
    // If this redirect tag exists as a site then we direct any processed data to the specified
    // site scraper instead of the plugin. Useful if you have a scraper that handles download
    // parsing and a plugin that can recursivly add jobs
    pub redirect: Option<String>,
}

///
/// Used to hold plugin or scraper data.
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ScraperOrPlugin {
    Scraper(ScraperInfo),
    Plugin(PluginInfo2),
}

/// A conjoined twin of scrapers and plugins
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct GlobalPluginScraper {
    /// Name of the site (human readable plz)
    pub name: String,
    /// Verison of the item
    pub version: usize,
    /// Weather this item should handle the file download
    pub should_handle_file_download: bool,
    /// Weather this item needs to handle text scraping
    pub should_handle_text_scraping: bool,
    /// If we should send files back when we're scraping
    pub should_send_files_on_scrape: bool,
    /// Any data thats needed to access restricted content
    pub login_type: Vec<(String, LoginType, LoginNeed, Option<String>, bool)>,
    /// Storage for the item, Will be loaded into the user_data slot for scrapers
    pub stored_info: Option<StoredInfo>,
    /// Any callbacks that should run on any events
    pub callbacks: Vec<GlobalCallbacks>,
    /// Storage for plugin or scraper info. Determines type.
    pub storage_type: Option<ScraperOrPlugin>,
}

///
/// Returns a default item for the GlobalPluginScraper
///
pub fn return_default_globalpluginparser() -> GlobalPluginScraper {
    GlobalPluginScraper {
        name: "".to_string(),
        version: 0,
        should_handle_file_download: false,
        should_handle_text_scraping: false,
        should_send_files_on_scrape: false,
        login_type: vec![],
        stored_info: None,
        callbacks: vec![],
        storage_type: None,
    }
}

pub fn return_default_jobsobj() -> DbJobsObj {
    DbJobsObj {
        id: None,
        time: 0,
        reptime: Some(0),
        priority: DEFAULT_PRIORITY,
        cachetime: DEFAULT_CACHETIME,
        cachechecktype: DEFAULT_CACHECHECK,
        site: "".to_string(),
        param: Vec::new(),
        jobmanager: DbJobsManager {
            jobtype: DbJobType::NoScrape,
            recreation: None,
        },
        isrunning: false,
        system_data: BTreeMap::new(),
        user_data: BTreeMap::new(),
    }
}

/// Determines how to run a function
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "clap", derive(EnumIter, Display))]
pub enum StartupThreadType {
    // Runs plugin and waits until it finished
    Inline,
    // Spawns a new thread, Runs concurrently to the calling function.
    Spawn,
    // DEFAULT - Waits for the on_start to finish. Runs cocurently to other on_start functions
    SpawnInline,
}

/// Information about CSV what we should do with the files.
#[derive(Debug, Default)]
#[cfg_attr(feature = "clap", derive(EnumIter, Display))]
pub enum CsvCopyMvHard {
    #[default]
    Copy,
    Move,
    Hardlink,
}

/// Tells DB which table to load.
#[derive(
    PartialEq, Debug, Serialize, bincode::Encode, bincode::Decode, Deserialize, Clone, Copy,
)]
#[cfg_attr(feature = "clap", derive(EnumIter))]
pub enum LoadDBTable {
    // Files table
    Files,
    // Jobs table
    Jobs,
    // Namespace mapping
    Namespace,
    // Parents mapping table
    Parents,
    // Relationships table
    Relationship,
    // Settings table
    Settings,
    // Tags storage table
    Tags,
    // Dead source urls
    DeadSourceUrls,

    // Loads all unloaded tables.
    All,
}
/*
#[allow(dead_code)]
pub fn stringto_commit_type(into: &String) -> CommitType {
    for each in CommitType::iter() {
        if into == &each.to_string() {
            return each;
        }
    }
    let mut panic = "Could Not format CommitType as one of: ".to_string();
    for each in CommitType::iter() {
        panic += format!("{} ", each).as_str();
    }
    panic!("{}", panic);
}*/

/// Dummy Holder Dummy thick
#[allow(dead_code)]
#[derive(Debug, PartialEq, Serialize, bincode::Encode, bincode::Decode, Deserialize)]
#[cfg_attr(feature = "clap", derive(EnumIter))]
pub enum SearchHolder {
    And(Vec<usize>),
    Or(Vec<usize>),
    Not(Vec<usize>),
}
/// Allows searching inside the db. search_relate relates the item in the vec with
/// eachother the IDs in search relate correspond to the id's in searches. IE: if 0
/// & 1 are an AND search then the 4 search items are AND searched in db in
/// addition to the 4 terms in the searches
#[allow(dead_code)]
#[derive(Debug, PartialEq, Serialize, bincode::Encode, bincode::Decode, Deserialize)]
pub struct SearchObj {
    pub search_relate: Option<Vec<SearchHolder>>,
    pub searches: Vec<SearchHolder>,
}

/*

#[allow(dead_code)]
pub fn stringto_search_type(into: &String) -> Search {
    for each in Search::iter() {
        if into == &each.to_string() {
            return each;
        }
    }
    let mut panic = "Could Not format CommitType as one of: ".to_string();
    for each in Search::iter() {
        panic += format!("{} ", each).as_str();
    }
    panic!("{}", panic);
}

#[allow(dead_code)]
pub fn stringto_jobtype(into: &String) -> DbJobType {
    for each in DbJobType::iter() {
        if into == &each.to_string() {
            return each;
        }
    }
    let mut panic = "Could not format DbJobType as one of:".to_string();
    for each in DbJobType::iter() {
        panic += format!("{} ", each).as_str();
    }
    panic!("{}", panic);
}*/

#[cfg(feature = "clap")]
use clap::ValueEnum;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
#[cfg_attr(feature = "clap", derive(Display, EnumIter))]
pub enum ScraperType {
    Manual,
    Automatic,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ScraperReturn {
    // STOP IMMEDIENTLY: ISSUE WITH SITE : PANICS no save
    EMCStop(String),
    // Hit nothing to search. Move to next job.
    Nothing,
    // Stop current job, Record issue Move to next.
    Stop(String),
    // Wait X seconds before retrying.
    Timeout(u64),
}

///
/// Kinda stupid. Will see if I need this in the future
///
#[derive(Debug)]
pub enum Flags {
    Redo,
}

/// What the scraper passes between loaded 3rd party scrapers and the internal
/// scrpaer.
#[derive(Debug)]
pub struct ScraperObject {
    pub file: HashSet<FileObject>,
    pub tag: HashSet<TagObject>,
    pub flag: Vec<Flags>,
}

/// Shared data to be passed for jobs
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    Deserialize,
    Serialize,
    bincode::Encode,
    bincode::Decode,
)]
pub struct ScraperData {
    pub job: JobScraper,
    pub system_data: BTreeMap<String, String>,
    pub user_data: BTreeMap<String, String>,
}

/// Defines what we need to reimport a file to derive a source URL. Currently only
/// support hash.
pub struct ScraperFileRegen {
    pub hash: HashesSupported,
}

/// Input for the scraper to parse the info from the system.
pub struct ScraperFileInput {
    pub hash: Option<String>,
    pub ext: Option<String>,
}

/// Defines what data the scraper will return. Will likely be a source URL or if we
/// can't parse from hash.
#[derive(Debug)]
pub struct ScraperFileReturn {
    pub tag: Option<GenericNamespaceObj>,
}

/// File object should of done this sooner lol
#[derive(
    Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode, Clone, Eq, Hash, PartialEq,
)]
pub struct DbFileObj {
    pub id: usize,
    pub hash: String,
    pub ext_id: usize,
    pub storage_id: usize,
}

/// File object with no id field if unknown
#[derive(
    Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode, Clone, Eq, Hash, PartialEq,
)]
pub struct DbFileObjNoId {
    pub hash: String,
    pub ext_id: usize,
    pub storage_id: usize,
}

/// Wrapper for DbFileStorage
#[derive(
    Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode, Clone, Eq, Hash, PartialEq,
)]
pub enum DbFileStorage {
    // Complete file object
    Exist(DbFileObj),
    // File object except ID
    NoIdExist(DbFileObjNoId),
    // Only the fileid is known at the time
    NoExist(usize),
    // Nothing is known (generate fid)
    NoExistUnknown,
}

/// File object Should only be used for parsing data from plugins
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode, Clone)]
pub struct PluginFileObj {
    pub id: Option<usize>,
    pub hash: Option<String>,
    pub ext: Option<String>,
    pub location: Option<String>,
}

/// Namespace object should of done this sooner lol
#[derive(Debug, Clone, Serialize, bincode::Encode, bincode::Decode, Deserialize)]
pub struct DbNamespaceObj {
    pub id: usize,
    pub name: String,
    pub description: Option<String>,
}

/// Namespace object should of done this sooner lol
// #[derive(Debug)] #[derive(Eq, Hash, PartialEq)] pub struct PluginRelatesObj {
// pub tag: Option`<String>`, pub name: Option`<String>`, pub description:
// Option`<String>`, }
/// Database Jobs object
#[derive(
    Debug, Hash, Eq, PartialEq, Clone, Serialize, bincode::Encode, bincode::Decode, Deserialize,
)]
pub struct DbJobsObj {
    /// id of the job. If exist then we are gravy baby
    pub id: Option<usize>,
    /// time job was added into db
    pub time: usize,
    /// Time to run job
    pub reptime: Option<usize>,
    // Determines which job we should run first. Higher values go first
    pub priority: usize,
    // How long should we keep a job in cache
    pub cachetime: Option<usize>,
    // How should we deduplicate jobs
    pub cachechecktype: JobCacheType,

    /// Site we're processing
    pub site: String,
    /// any params that need to get passed into the scraper, plugin, scraper etc
    pub param: Vec<ScraperParam>,
    /// jobs manager. Configuration goes here for things
    pub jobmanager: DbJobsManager,
    /// Is this job currently running
    pub isrunning: bool,
    /// Any data that should be not tampered with by a scrapre, plugin etc. system useage only
    pub system_data: BTreeMap<String, String>,
    /// Any data that should be editied by the scraper, plugin etc. Can persist between job runs
    pub user_data: BTreeMap<String, String>,
}

/// Manager on job type and logic
#[derive(
    Debug,
    Hash,
    Eq,
    PartialEq,
    Clone,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
    Ord,
    PartialOrd,
)]
pub struct DbJobsManager {
    pub jobtype: DbJobType,
    pub recreation: Option<DbJobRecreation>,
    // #[arg(long)] pub additionaldata: Option<(Vec`<String>`, Vec`<String>`)>,
}

/// Recreate current job on x event
#[derive(
    Debug,
    Hash,
    Eq,
    PartialEq,
    Clone,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
    Ord,
    PartialOrd,
)]
pub enum DbJobRecreation {
    OnTagId(usize, Option<usize>),
    OnTag(String, usize, Option<usize>),
    // first number is the wait time betwen jobs second field is a count. If count is eq None then
    // we should never remove job. Else if count eq zero then remove job
    AlwaysTime(usize, Option<usize>),
}

/// Type of job in db. Will be used to confirm what the scraping logic should work.
#[derive(
    Debug,
    Copy,
    Hash,
    Eq,
    PartialEq,
    Clone,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
    Ord,
    PartialOrd,
)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(
    feature = "clap",
    derive(EnumIter, ValueEnum, EnumString),
    clap(rename_all = "kebab_case")
)]
pub enum DbJobType {
    /// Default recognises this as a param query.
    Params,
    /// Runs a plugin directly (don't use plz).
    Plugin,
    /// Signifies that this is a url to a file that does not need to be parsed by a scraper.
    FileUrl,
    /// Something else sends to scraper.
    Scraper,
    /// Do Not Reachout to network to scrape this item
    NoScrape,
}

/// Database Parents Object.
#[derive(
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
)]
pub struct DbParentsObj {
    pub tag_id: usize,
    pub relate_tag_id: usize,
    /// IE Only limit this to A->B as it relates to C.
    pub limit_to: Option<usize>,
}

/// Database Relationship Object
#[derive(Debug)]
pub struct DbRelationshipObj {
    pub fileid: usize,
    pub tagid: usize,
}

/// Database Settings Object
#[derive(Debug, Deserialize, Clone, Serialize, bincode::Encode, bincode::Decode)]
pub struct DbSettingObj {
    pub name: String,
    pub pretty: Option<String>,
    pub num: Option<usize>,
    pub param: Option<String>,
}

/// Database search object
#[derive(Debug)]
pub struct DbSearchObject {
    pub tag: String,
    pub namespace: Option<String>,
    pub namespace_id: Option<usize>,
}

/// Database search enum between 2 tags
#[derive(Debug)]
pub enum DbSearchTypeEnum {
    And,
    Or,
}

/// Database search query
#[derive(Debug)]
pub struct DbSearchQuery {
    pub tag_one: DbSearchObject,
    pub tag_two: DbSearchObject,
    pub search_enum: DbSearchTypeEnum,
}

/// Database Relationship For Plugin passing
#[derive(Debug, Clone)]
pub struct DbPluginRelationshipObj {
    pub file_hash: String,
    pub tag_name: String,
    pub tag_namespace: String,
}

#[derive(Debug, Clone)]
pub enum DBPluginOutputEnum {
    Add(Vec<DBPluginOutput>),
    Del(Vec<DBPluginOutput>),
    None,
}

#[derive(PartialEq, Debug)]
pub struct DBPluginTagOut {
    pub name: String,
    pub parents: Option<Vec<DbPluginParentsObj>>,
    pub namespace: String,
}

/// Database Parents Object.
#[derive(Debug, PartialEq)]
pub struct DbPluginParentsObj {
    pub tag_namespace_string: String,
    pub relate_namespace_id: String,
    pub relate_tag_id: String,
}

///
/// Defines actions we can do on import
///
#[cfg(feature = "clap")]
#[derive(Debug, clap::Parser, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab_case")]
pub enum FileAction {
    ///Copies the file
    Copy,
    /// Moves the file into the db.
    Move,
    /// Hardlinks the file into the db
    HardLink,
}

/// Plugin output for the passed object
#[derive(Debug, Clone)]
pub struct DBPluginOutput {
    // Adds a tag to DB
    pub tag: Vec<TagObject>,

    // Adds a setting
    pub setting: Vec<DbSettingObj>,
    // Adds a relationship into the DB.
    pub relationship: Vec<DbPluginRelationshipObj>,
    // Adds a job
    pub jobs: Vec<DbJobsObj>,
    // Adds a file into db
    pub file: Vec<PluginFileObj>,
}

#[derive(
    Debug, Eq, Hash, PartialEq, Deserialize, Serialize, bincode::Encode, bincode::Decode, Clone,
)]
pub enum FileSource {
    Url(String),
    Bytes(Vec<u8>),
}

/// Represents one file
#[derive(Debug, Eq, Hash, PartialEq, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub struct FileObject {
    pub source: Option<FileSource>,
    // Hash of file
    pub hash: HashesSupported,
    pub tag_list: Vec<TagObject>,
    // Skips downloading the file if a tag matches this.
    pub skip_if: Vec<SkipIf>,
}

#[derive(Debug, Eq, Hash, PartialEq, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum SourceOrUrl {
    Url(String),
    File(Vec<u8>),
}

///
/// Search types for searching
/// String just checks that a string exists in the tag and it runs the callback
/// Regex searches the tag via regex searching
///
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SearchType {
    String(String),
    Regex(String),
}

#[cfg(feature = "regex")]
#[derive(Clone, Debug)]
pub struct RegexStorage(pub Regex);

#[cfg(feature = "regex")]
impl Eq for RegexStorage {}

#[cfg(feature = "regex")]
impl PartialEq for RegexStorage {
    fn eq(&self, other: &RegexStorage) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

#[cfg(feature = "regex")]
impl std::hash::Hash for RegexStorage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

/// Holder of Tag info. Keeps relationalship info into account.
#[derive(
    Debug, Eq, PartialEq, Hash, Clone, Deserialize, Serialize, bincode::Encode, bincode::Decode,
)]
pub struct TagObject {
    pub namespace: GenericNamespaceObj,
    pub tag: String,
    pub tag_type: TagType,
    pub relates_to: Option<SubTag>,
}

#[derive(
    Debug, Eq, PartialEq, Hash, Clone, Deserialize, Serialize, bincode::Encode, bincode::Decode,
)]
pub struct SubTag {
    pub namespace: GenericNamespaceObj,
    pub tag: String,
    pub limit_to: Option<Tag>,
    pub tag_type: TagType,
}

#[derive(
    Debug, Eq, PartialEq, Hash, Clone, Deserialize, Serialize, bincode::Encode, bincode::Decode,
)]
pub struct GenericNamespaceObj {
    pub name: String,
    pub description: Option<String>,
}

/// Tag Type object. Represents metadata for parser.
#[derive(Debug, Clone)]
#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum TagType {
    // Normal tag.
    Normal,
    // Do not run any regex on this pos
    NormalNoRegex,
    // Scraper to download and parse a new url.
    ParseUrl((ScraperData, Option<SkipIf>)),
    // Probably will add support for something like file descriptors or plugin
    // specific things.
    Special,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, bincode::Encode, bincode::Decode,
)]
pub struct Tag {
    pub tag: String,
    pub namespace: GenericNamespaceObj,
    // pub needsrelationship: bool, // If theirs a relationship then we will not add
    // it to the checkers.
}

/// Used for skipping a ParseUrl in TagType if a tag exists.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, bincode::Encode, bincode::Decode,
)]
pub enum SkipIf {
    // If a relationship between any file and tag exists.
    FileTagRelationship(Tag),
    // The tag is qnique and if their are X number or more of GenericNamespaceObj
    // associated with the file Then we'll skip it
    FileNamespaceNumber((Tag, GenericNamespaceObj, usize)),
    // Skips a file if the hash X exists
    FileHash(String),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    Deserialize,
    Serialize,
    bincode::Encode,
    bincode::Decode,
)]
pub struct JobScraper {
    pub site: String,
    pub param: Vec<ScraperParam>,
    pub job_type: DbJobType,
}

/// Supported types of hashes in Rust Hydrus
#[derive(Debug, Clone, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq)]
//#[cfg_attr(feature = "clap", derive(Display))]
pub enum HashesSupported {
    Md5(String),
    Sha1(String),
    Sha256(String),
    Sha512(String),
    None,
}

#[derive(Debug)]
pub struct JobsAdd {
    pub site: String,
    pub query: String,
    pub time: String,
}

// Manages what the jobs are.
#[derive(Debug)]
pub struct JobsRemove {
    pub site: String,
    pub query: String,
    pub time: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum AllFields {
    JobsAdd(JobsAdd),
    JobsRemove(JobsRemove),
    Search(Search),
    Nothing,
    Tasks(Tasks),
}

/// Plugin Callable actions for callbacks
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum PluginCallback {
    // Ran when a file is downloaded
    Download,
    // Starts when the software start
    Start(StartupThreadType),
    // Used for when we need to get / register a login
    LoginNeeded,
    // Custom callback to be used for cross communication
    Callback(CallbackInfo),
    // Runs when a tag has exists.
    // First when the tag exists OR when the namespace exists
    // Use None when searching all or Some when searching restrictivly
    Tag(Vec<(Option<SearchType>, Option<String>, Option<String>)>),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum GlobalCallbacks {
    // Ran when a file is downloaded
    Download,
    // Runs when a file is imported manually
    Import,
    // Starts when the software start
    Start(StartupThreadType),
    // Used for when we need to get / register a login
    LoginNeeded,
    // Custom callback to be used for cross communication
    Callback(CallbackInfo),
    // Runs when a tag has exists.
    // First when the ns exists, 2nd when the namespace does not exist
    // Use None when searching all or Some when searching restrictivly
    Tag((Option<SearchType>, Vec<String>, Vec<String>)),
}

/// Callback info for live plugins Gets sent to plugins
#[derive(Debug, PartialEq, Eq, Hash, Serialize, bincode::Encode, bincode::Decode, Deserialize)]
pub struct CallbackInfoInput {
    // Version of the expected call. Its on the plugin to handle this properly
    pub vers: usize,
    // Name of variable
    pub data_name: Vec<String>,
    // Data for variable of data_name
    pub data: Vec<CallbackCustomDataReturning>,
}

/// Callback info for live plugins
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct CallbackInfo {
    // Name of plugin's function
    pub func: String,
    // Version of plugin
    pub vers: usize,
    // Name of variable
    pub data_name: Vec<String>,
    // Data for variable of data_name
    pub data: Vec<CallbackCustomData>,
}

/// Data that's to be recieved to a plugin
#[derive(
    Debug, PartialEq, Eq, Hash, Clone, Serialize, bincode::Encode, bincode::Decode, Deserialize,
)]
pub enum CallbackCustomData {
    String,
    U8,
    Usize,
    VString,
    VU8,
    VUsize,
    VCallback,
}

pub enum FileDownloadReturn {}

/// Data that gets sent to a plugin
#[derive(
    Debug, PartialEq, Eq, Hash, Serialize, bincode::Encode, bincode::Decode, Deserialize, Clone,
)]
pub enum CallbackCustomDataReturning {
    String(String),
    U8(Vec<u8>),
    Usize(usize),
    VString(Vec<String>),
    VU8(Vec<u8>),
    VUsize(Vec<usize>),
    VCallback(Vec<CallbackCustomDataReturning>),
}

impl std::fmt::Display for CallbackCustomDataReturning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallbackCustomDataReturning::String(s) => write!(f, "{}", s),
            CallbackCustomDataReturning::U8(bytes) => {
                write!(f, "U8({:?})", bytes)
            }
            CallbackCustomDataReturning::Usize(v) => write!(f, "Usize({})", v),
            CallbackCustomDataReturning::VString(v) => {
                write!(f, "VString({:?})", v)
            }
            CallbackCustomDataReturning::VU8(v) => {
                write!(f, "VU8({:?})", v)
            }
            CallbackCustomDataReturning::VUsize(v) => {
                write!(f, "VUsize({:?})", v)
            }
            CallbackCustomDataReturning::VCallback(list) => {
                write!(f, "VCallback([")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "])")
            }
        }
    }
}

/// information block for plugin info
#[derive(Debug)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: f32,
    pub api_version: f32,
    pub callbacks: Vec<PluginCallback>,
    pub communication: Option<PluginSharedData>,
}

#[derive(Debug)]
pub struct PluginSharedData {
    pub thread: StartupThreadType,
    pub com_channel: Option<PluginCommunicationChannel>,
}

#[derive(Debug)]
pub enum PluginCommunicationChannel {
    Pipe(String),
    None,
}

/// Scraper type passed to params
/// Generic holder for params that are from a job
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    Deserialize,
    Serialize,
    bincode::Encode,
    bincode::Decode,
)]
pub enum ScraperParam {
    Normal(String),
    Url(String),
    Login(LoginType),
    Database(String),
}
