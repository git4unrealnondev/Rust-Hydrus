use nohash_hasher::NoHashHasher;
use pipe;
use std::fmt;
use std::{collections::HashMap, hash::BuildHasherDefault};
use strum::IntoEnumIterator;
use strum_macros::Display;
use strum_macros::EnumIter;

#[derive(Debug, EnumIter, Clone, Eq, Hash, PartialEq)]
pub enum CommitType {
    StopOnNothing, // Processes all files and data doesn't stop processing.
    StopOnFile,    // Stops processing if it sees a file it's already seen.
                   //SkipOnFile,
                   //AddToDB,
}

impl fmt::Display for CommitType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
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
    EMCStop(String), // STOP IMMEDIENTLY: ISSUE WITH SITE : PANICS no save
    Nothing,         // Hit nothing to search. Move to next job.
    Stop(String),    // Stop current job, Record issue Move to next.
    Timeout(u64),    // Wait X seconds before retrying.
}

///
/// What the scraper passes between loaded 3rd party scrapers and the internal scrpaer.
///
#[derive(Debug)]
pub struct ScraperObject {
    pub file: HashMap<u64, FileObject, BuildHasherDefault<NoHashHasher<u64>>>,
}

///
/// File object
/// should of done this sooner lol
///
#[derive(Debug)]
pub struct DbFileObj {
    pub id: Option<usize>,
    pub hash: Option<String>,
    pub ext: Option<String>,
    pub location: Option<String>,
}

///
/// Namespace object
/// should of done this sooner lol
///
#[derive(Debug)]
pub struct DbNamespaceObj {
    pub id: Option<usize>,
    pub name: Option<String>,
    pub description: Option<String>,
}

///
/// Database Jobs object
///
#[derive(Debug)]
pub struct DbJobsObj {
    pub time: Option<usize>,
    pub reptime: Option<usize>,
    pub site: Option<String>,
    pub param: Option<String>,
    pub committype: Option<CommitType>,
}

///
/// Database Parents Object.
///
#[derive(Debug)]
pub struct DbParentsObj {
    pub tag_namespace_id: usize,
    pub tag_id: usize,
    pub relate_namespace_id: usize,
    pub relate_tag_id: usize,
}

///
/// Database Relationship Object
///
#[derive(Debug)]
pub struct DbRelationshipObj {
    pub fileid: usize,
    pub tagid: usize,
}

///
/// Database Settings Object
///
#[derive(Debug)]
pub struct DbSettingObj {
    pub name: String,
    pub pretty: Option<String>,
    pub num: Option<usize>,
    pub param: Option<String>,
}

///
/// Database Tags Object
///
#[derive(Debug)]
pub struct DbTagObj {
    pub id: Option<usize>,
    pub name: String,
    pub parents: Option<usize>,
    pub namespace: Option<usize>,
}

///
/// Database Relationship For Plugin passing
///
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

///
/// Database Parents Object.
///
#[derive(Debug, PartialEq)]
pub struct DbPluginParentsObj {
    pub tag_namespace_string: String,
    pub relate_namespace_id: String,
    pub relate_tag_id: String,
}

///
/// Plugin output for the passed object
///
#[derive(Debug)]
pub struct DBPluginOutput {
    pub tag: Option<Vec<DBPluginTagOut>>,   // Adds a tag to DB
    pub setting: Option<Vec<DbSettingObj>>, // Adds a setting
    pub relationship: Option<Vec<DbPluginRelationshipObj>>, // Adds a relationship into the DB.
    pub parents: Option<Vec<DbParentsObj>>, // Adds a parent object in db
    pub jobs: Option<Vec<DbJobsObj>>,       // Adds a job
    pub namespace: Option<Vec<DbNamespaceObj>>, // Adds a namespace
    pub file: Option<Vec<DbFileObj>>,       // Adds a file into db
}

///
/// Represents one file
///
#[derive(Debug)]
pub struct FileObject {
    pub source_url: String,
    pub hash: HashesSupported, // Hash of file
    pub tag_list: HashMap<u64, TagObject, BuildHasherDefault<NoHashHasher<u64>>>,
}

///
/// Holder of Tag info.
/// Keeps relationalship info into account.
///
#[derive(Debug)]
pub struct TagObject {
    pub namespace: String,
    pub tag: String,
    pub tag_type: TagType,
    pub relates_to: Option<(String, String)>,
}

///
/// Tag Type object. Represents metadata for parser.
///
#[derive(Debug)]
#[allow(dead_code)]
pub enum TagType {
    Normal,  // Normal tag.
    Special, // Probably will add support for something like file descriptors or plugin specific things.
}

///
/// Supported types of hashes in Rust Hydrus
///
#[derive(Debug, Clone, Display)]
#[allow(dead_code)]
pub enum HashesSupported {
    Md5(String),
    Sha1(String),
    Sha256(String),
    None,
}

#[derive(Debug)]
pub struct JobsAdd {
    pub site: String,
    pub query: String,
    pub time: String,
    pub committype: CommitType,
}

#[derive(Debug)] // Manages what the jobs are.
pub struct JobsRemove {
    pub site: String,
    pub query: String,
    pub time: String,
}

#[derive(Debug, EnumIter, Display)]
// Manages what the search can do.
pub enum Search {
    Fid(Vec<String>),
    Tid(Vec<String>),
    Tag(Vec<String>),
    Hash(Vec<String>),
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

#[derive(Debug, EnumIter, Display)]
pub enum Tasks {
    Csv(String, CsvCopyMvHard), // CSV importation. cp mv hardlink
}

///
/// Determines how to run a function
///
#[derive(Debug, EnumIter, Display)]
pub enum PluginThreadType {
    Inline,        // Run plugin inside of the calling function. DEFAULT
    Spawn,         // Spawns a new thread, Runs concurrently to the calling function.
    SpawnBlocking, // Spawns a new thread, Blocks the main thread until the plugin finishes work.
    Daemon, // Spawns a thread as a daemon all calls to the plugin will be routed to the daemon thread.
}

///
/// Information about CSV what we should do with the files.
///
#[derive(Debug, EnumIter, Display, Default)]
pub enum CsvCopyMvHard {
    #[default]
    Copy,
    Move,
    Hardlink,
}

///
/// Tells DB which table to load.
///
#[derive(EnumIter, PartialEq, Debug)]
pub enum LoadDBTable {
    Files,        // Files table
    Jobs,         // Jobs table
    Namespace,    // Namespace mapping
    Parents,      // Parents mapping table
    Relationship, // Relationships table
    Settings,     // Settings table
    Tags,         // Tags storage table
    All,          // Loads all unloaded tables.
}

///
/// Plugin Callable actions for callbacks
///
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PluginCallback {
    OnDownload, // Ran when a file is downloaded
    OnStart,    // Starts when the software start
}

///
/// information block for plugin info
///
#[derive(Debug)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: f32,
    pub api_version: f32,
    pub callbacks: Vec<PluginCallback>,
    pub communication: Option<PluginSharedData>,
}

///
///
///
#[derive(Debug)]
pub struct PluginSharedData {
    pub thread: PluginThreadType,
    pub com_channel: Option<PluginCommunicationChannel>,
}

///
///
///
#[derive(Debug)]
pub enum PluginCommunicationChannel {
    pipe(String),
    None,
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
// let temp = AllFields::JobsAdd(JobsAdd{Site: "yeet".to_owned(), Query: "yeet".to_owned(), Time: "Lo".to_owned(), Loop: "yes".to_owned(), ReCommit: "Test".
