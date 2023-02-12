use nohash_hasher::NoHashHasher;
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
pub enum ScraperReturn {
    EMCStop(String), // STOP IMMEDIENTLY: ISSUE WITH SITE : PANICS no save
    Nothing,         // Hit nothing to search. Move to next job.
    Stop(String),    // Stop current job, Record issue Move to next.
    Timeout(u64),    // Wait X seconds before retrying.
}

///
/// What the scraper passes between loaded 3rd party scrapers and the internal scrpaer.
///
pub struct ScraperObject {
    pub file: HashMap<u64, FileObject, BuildHasherDefault<NoHashHasher<u64>>>,
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
pub enum TagType {
    Normal,  // Normal tag.
    Special, // Probably will add support for something like file descriptors or plugin specific things.
}

///
/// Supported types of hashes in Rust Hydrus
///
#[derive(Debug, Clone, Display)]
pub enum HashesSupported {
    Md5(String),
    Sha1(String),
    Sha256(String),
    None,
}

#[derive(Debug)]
pub struct jobs_add {
    pub site: String,
    pub query: String,
    pub time: String,
    pub committype: CommitType,
}

#[derive(Debug)] // Manages what the jobs are.
pub struct jobs_remove {
    pub site: String,
    pub query: String,
    pub time: String,
}

#[derive(Debug, EnumIter, Display)]
// Manages what the search can do.
pub enum Search {
    fid(Vec<String>),
    tid(Vec<String>),
    tag(Vec<String>),
    hash(Vec<String>),
}

#[derive(Debug)]
pub enum AllFields {
    EJobsAdd(jobs_add),
    EJobsRemove(jobs_remove),
    ESearch(Search),
    ENothing,
    ETasks(Tasks),
}

#[derive(Debug, EnumIter, Display)]
pub enum Tasks {
    csv(String, CsvCopyMvHard), // CSV importation. cp mv hardlink
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

pub fn stringto_Search_type(into: &String) -> Search {
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
// let temp = AllFields::JobsAdd(JobsAdd{Site: "yeet".to_owned(), Query: "yeet".to_owned(), Time: "Lo".to_owned(), Loop: "yes".to_owned(), ReCommit: "Test".to_owned(), CommitType: CommitType::StopOnNothing});
