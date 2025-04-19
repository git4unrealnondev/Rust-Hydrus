#![allow(dead_code)]
#![allow(unused_variables)]

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::{Display, EnumString};

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

#[derive(Debug, EnumIter, Display)]
// Manages what the search can do.
pub enum Search {
    Fid(Vec<String>),
    Tid(Vec<String>),
    Tag(Vec<String>),
    Hash(Vec<String>),
}

#[derive(Debug, EnumIter, Display)]
pub enum Tasks {
    // CSV importation. cp mv hardlink
    Csv(String, CsvCopyMvHard),
    Remove(TasksRemove),
}

#[derive(Debug, EnumIter, Display, Default)]
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

/*///
/// An individual site according to the scraper
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SiteStruct {
    /// Name of the site (human readable plz)
    pub name: String,
    /// List of Strings to match to determine if we should use this scraper.
    pub sites: Vec<String>,
    /// Verison of the scraper
    pub version: usize,
    /// Ratelimit for this site
    pub ratelimit: (u64, std::time::Duration),
    /// Weather this scraper should handle the file download
    pub should_handle_file_download: bool,
    /// Weather this scraper needs to handle text scraping
    pub should_handle_text_scraping: bool,
    /// Any data thats needed to access restricted content
    pub login_type: Vec<(String, LoginType, LoginNeed, Option<String>, bool)>,
    /// Storage for the site scraper, Will be loaded into the user_data slot
    pub stored_info: Option<StoredInfo>,
}*/

///
/// Info for scrapers as apart of the Global merge
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ScraperInfo {
    /// Ratelimit for this site
    pub ratelimit: (u64, std::time::Duration),
    pub sites: Vec<String>,
}

///
/// Info for plugins as apart of the Global merge
///
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct PluginInfo2 {
    /// Manages com settings between Plugin and host
    pub com_type: PluginThreadType,
    pub com_channel: bool,
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
    /// Any data thats needed to access restricted content
    pub login_type: Vec<(String, LoginType, LoginNeed, Option<String>, bool)>,
    /// Storage for the item, Will be loaded into the user_data slot for scrapers
    pub stored_info: Option<StoredInfo>,
    /// Any callbacks that should run on any events
    pub callbacks: Vec<GlobalCallbacks>,
    /// Storage for plugin or scraper info. Determines type.
    pub storage_type: Option<ScraperOrPlugin>,
}

pub fn return_default_globalpluginparser() -> GlobalPluginScraper {
    GlobalPluginScraper {
        name: "".to_string(),
        version: 0,
        should_handle_file_download: false,
        should_handle_text_scraping: false,
        login_type: vec![],
        stored_info: None,
        callbacks: vec![],
        storage_type: None,
    }
}

/// Determines how to run a function
#[derive(Debug, EnumIter, Display, Clone, Eq, Hash, PartialEq)]
pub enum PluginThreadType {
    // Run plugin inside of the calling function. DEFAULT
    Inline,
    // Spawns a new thread, Runs concurrently to the calling function.
    Spawn,
    // Spawns a new thread, Blocks the main thread until the plugin finishes work.
    SpawnBlocking,
    // Spawns a thread as a daemon all calls to the plugin will be routed to the
    // daemon thread.
    Daemon,
}

/// Information about CSV what we should do with the files.
#[derive(Debug, EnumIter, Display, Default)]
pub enum CsvCopyMvHard {
    #[default]
    Copy,
    Move,
    Hardlink,
}

/// Tells DB which table to load.
#[derive(
    EnumIter,
    PartialEq,
    Debug,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
    Clone,
    Copy,
    ValueEnum,
)]
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
    // Loads all unloaded tables.
    All,
}

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
}

/// Dummy Holder Dummy thick
#[allow(dead_code)]
#[derive(Debug, EnumIter, PartialEq, Serialize, bincode::Encode, bincode::Decode, Deserialize)]
pub enum SearchHolder {
    And((usize, usize)),
    Or((usize, usize)),
    Not((usize, usize)),
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
}

use clap::ValueEnum;

#[derive(
    Debug,
    EnumIter,
    Clone,
    Eq,
    Hash,
    PartialEq,
    Copy,
    EnumString,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
    ValueEnum,
)]
#[serde(rename_all = "PascalCase")]
#[clap(rename_all = "kebab_case")]
pub enum CommitType {
    /// Processes all files and data doesn't stop processing.
    StopOnNothing,
    /// Stops processing if it sees a file it's already seen.
    // SkipOnFile,
    StopOnFile,
    // AddToDB,
}

impl fmt::Display for CommitType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively: fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, EnumIter, Clone, PartialEq, Hash, Eq, Display)]
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

/// What the scraper passes between loaded 3rd party scrapers and the internal
/// scrpaer.
#[derive(Debug)]
pub struct ScraperObject {
    pub file: HashSet<FileObject>,
    pub tag: HashSet<TagObject>,
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
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
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
    pub id: usize,
    /// time job was added into db
    pub time: usize,
    /// Time to run job
    pub reptime: Option<usize>,
    /// Site we're processing
    pub site: String,
    /// any params that need to get passed into the scraper, plugin, scraper etc
    pub param: Vec<ScraperParam>,
    /// jobs manager. Configuration goes here for things
    pub jobmanager: DbJobsManager,
    /// I don't think I use this
    pub committype: Option<CommitType>,
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
    EnumIter,
    Display,
    Serialize,
    bincode::Encode,
    bincode::Decode,
    Deserialize,
    ValueEnum,
    Ord,
    PartialOrd,
)]
#[clap(rename_all = "kebab_case")]
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
#[derive(Debug)]
pub struct DbPluginRelationshipObj {
    pub file_hash: String,
    pub tag_name: String,
    pub tag_namespace: String,
}

#[derive(Debug)]
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

/// Namespace for plugin objects
#[derive(Debug, PartialEq)]
pub struct DbPluginNamespace {
    pub name: String,
    pub description: Option<String>,
}

/// Plugin output for the passed object
#[derive(Debug)]
pub struct DBPluginOutput {
    // Adds a tag to DB
    pub tag: Option<Vec<DBPluginTagOut>>,
    // Add;s a setting
    pub setting: Option<Vec<DbSettingObj>>,
    // Adds a relationship into the DB.
    pub relationship: Option<Vec<DbPluginRelationshipObj>>,
    // Adds a parent object in db
    pub parents: Option<Vec<DbParentsObj>>,
    // Adds a job
    pub jobs: Option<Vec<DbJobsObj>>,
    // Adds a namespace
    pub namespace: Option<Vec<DbPluginNamespace>>,
    // Adds a file into db
    pub file: Option<Vec<PluginFileObj>>,
}

/// Represents one file
#[derive(Debug, Eq, Hash, PartialEq, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub struct FileObject {
    pub source_url: Option<String>,
    // Hash of file
    pub hash: HashesSupported,
    pub tag_list: Vec<TagObject>,
    // Skips downloading the file if a tag matches this.
    pub skip_if: Vec<SkipIf>,
}

///
/// Search types for searching
/// String just checks that a string exists in the tag and it runs the callback
/// Regex searches the tag via regex searching
///
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SearchType {
    String(String),
    #[cfg(feature = "regex")]
    Regex(RegexStorage),

    #[cfg(not(feature = "regex"))]
    Regex(),
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
#[derive(Debug, Clone, Display, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq)]
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
    pub committype: CommitType,
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
    Start,
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
    // Starts when the software start
    Start,
    // Used for when we need to get / register a login
    LoginNeeded,
    // Custom callback to be used for cross communication
    Callback(CallbackInfo),
    // Runs when a tag has exists.
    // First when the ns exists, 2nd when the namespace does not exist
    // Use None when searching all or Some when searching restrictivly
    Tag((Option<SearchType>, Option<String>, Option<String>)),
}

/// Callback info for live plugins Gets sent to plugins
#[derive(Debug, PartialEq, Eq, Hash, Serialize, bincode::Encode, bincode::Decode, Deserialize)]
pub struct CallbackInfoInput {
    // Name of variable
    pub data_name: Vec<String>,
    // Data for variable of data_name
    pub data: Option<Vec<CallbackCustomDataReturning>>,
}

/// Callback info for live plugins
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct CallbackInfo {
    // Name of registered plugin to call
    pub name: String,
    // Name of plugin's function
    pub func: String,
    // Version of plugin
    pub vers: usize,
    // Name of variable
    pub data_name: Vec<String>,
    // Data for variable of data_name
    pub data: Option<Vec<CallbackCustomData>>,
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
}

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
    pub thread: PluginThreadType,
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
