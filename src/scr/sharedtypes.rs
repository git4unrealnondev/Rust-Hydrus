use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::Display;

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

#[derive(Debug)]
pub struct jobs_add {
    pub site: String,
    pub query: String,
    pub time: String,
    pub committype: CommitType,
}

#[derive(Debug)] // Manages what the job can do.
pub struct jobs_remove {
    pub site: String,
    pub query: String,
    pub time: String,
}

#[derive(Debug)]
// Manages what the search can do.
pub struct search {
    pub fid: String,
    pub tid: String,
}

#[derive(Debug)]
pub enum AllFields {
    EJobsAdd(jobs_add),
    EJobsRemove(jobs_remove),
    ESearch(search),
    ENothing,
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

// let temp = AllFields::JobsAdd(JobsAdd{Site: "yeet".to_owned(), Query: "yeet".to_owned(), Time: "Lo".to_owned(), Loop: "yes".to_owned(), ReCommit: "Test".to_owned(), CommitType: CommitType::StopOnNothing});
