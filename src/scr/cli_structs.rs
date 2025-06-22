use crate::sharedtypes;
use clap::{arg, Parser, Subcommand, ValueEnum};

/// From: git4unrealnondev Das code sucks.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct MainWrapper {
    #[command(subcommand)]
    pub a: Option<Test>,
}

#[derive(Debug, Parser)]
pub enum Test {
    /// Manages jobs in db.
    #[clap(subcommand)]
    Job(JobStruct),
    /// Searches the DB.
    #[clap(subcommand)]
    Search(SearchStruct),
    /// Tasks to perform with the program.
    #[clap(subcommand)]
    Tasks(TasksStruct),
}

#[derive(Subcommand, Debug)]
pub enum TasksStruct {
    /// Manages a CSV file.
    #[clap(subcommand)]
    Csv(CsvStruct),
    /// Database related tasks
    #[clap(subcommand)]
    Database(Database),
    /// Reimports a directory based on scraper.
    #[clap(subcommand)]
    Reimport(Reimport),
    /// Scraper related actions
    #[clap(subcommand)]
    Scraper(ScraperAction),

    /// Imports a file into the db.
    Import(Directory),
}

/// Type of job in db. Will be used to confirm what the scraping logic should work.
#[derive(Debug, Copy, Hash, Eq, PartialEq, Clone, ValueEnum)]
#[clap(rename_all = "kebab_case")]
pub enum DbJobType {
    /// Default recognises this as a param query.
    Params,
    /// Runs a plugin directly (don't use plz).
    Plugin,
    /// Signifies that this is a FileUrl.
    FileUrl,
    /// Something else sends to scraper.
    Scraper,
}

#[derive(Debug, Parser)]
pub enum ScraperAction {
    /// Tests a scraper
    Test(ScraperTest),
}

#[derive(Debug, Parser)]
pub struct ScraperTest {
    /// Scraper to call
    pub scraper: String,
    /// Action to call into a scraper
    pub action_type: DbJobType,
    /// Input to send to the scraper
    pub input: Option<String>,
}

#[derive(Debug, Parser)]
pub struct DirectoryLocation {
    pub location: String,
    pub site: String,
}
#[derive(Debug, Parser)]
#[clap(rename_all = "kebab_case")]
pub struct Directory {
    /// Location to search
    pub location: String,
    #[clap(default_value = "copy")]
    pub file_action: sharedtypes::FileAction,
}

#[derive(Debug, Subcommand)]
pub enum Reimport {
    DirectoryLocation(DirectoryLocation),
}

#[derive(Debug, Subcommand)]
pub enum Database {
    #[clap(subcommand)]
    Remove(NamespaceInfo),
    #[clap(subcommand)]
    RemoveWhereNot(NamespaceInfo),
    /// Compresses the databases tag & relationships. Will add parent support soon.
    CompressDatabase,
    /// Checks the in memory DB
    CheckInMemdb,
    /// Checks the files on the filesystem. Warning is hella slow uses multithreading
    /// to make this go faster but still...
    CheckFiles,
    /// Backs up the database to a folder defined in settings.
    BackupDB,
}

/// Removes a namespace, tags & relationships from db.
#[derive(Debug, Subcommand)]
pub enum NamespaceInfo {
    /// A Namespace String to search for.
    NamespaceString(NamespaceString),
    /// A Namespace Id to search for.
    NamespaceId(NamespaceId),
}

#[derive(Debug, Parser)]
pub struct NamespaceString {
    /// Namespace String to search to remove.
    #[arg(exclusive = true, required = true)]
    pub namespace_string: String,
}

#[derive(Debug, Parser)]
pub struct NamespaceId {
    /// Namespace Id to remove.
    #[arg(exclusive = true, required = true)]
    pub namespace_id: usize,
}

#[derive(Debug, Subcommand)]
pub enum CsvStruct {
    /// Manages a CSV file.
    Csv,
}

#[derive(Subcommand, Debug)]
pub enum JobStruct {
    /// Adds a job to the system
    Add(JobAddStruct),
    /// Bulk adds jobs to the system
    AddBulk(JobBulkAddStruct),
    /// Removes a job from the system
    Remove(JobRemovalStruct),
}

/// Adds support for bulk adding jobs
#[derive(Debug, Parser)]
pub struct JobBulkAddStruct {
    /// Webite, Setup by nickname or by url base
    #[arg(exclusive = false, required = true)]
    pub site: String,
    /// Tag query for multiple items use " " and a space to seperate tags
    #[arg(exclusive = false, required = true)]
    pub query: String,
    /// Time, special time of now for running a job now.
    #[arg(exclusive = false, required = true)]
    pub time: String,
    /// Job type to run
    #[arg(exclusive = false, required = true)]
    pub jobtype: Option<sharedtypes::DbJobType>,
    /// Loops through all items have a , to seperate the items currently injects into
    /// the query parameter of the job using {inject} as the injection point
    #[arg(value_delimiter = ',', exclusive = false, required = true)]
    pub bulkadd: Vec<String>,
    #[clap(subcommand)]
    pub recursion: Option<DbJobRecreationClap>,
    #[arg(
        exclusive = false,
        required = true,
        num_args = 2,
        value_delimiter = ',',
        value_terminator = ";"
    )]
    pub system_data: Option<Vec<String>>,
}

/// Manager on job type and logic
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct DbJobsManagerClap {
    pub jobtype: DbJobType,
    pub recreation: Option<DbJobRecreationClap>,
    // #[arg(long)] pub additionaldata: Option<(Vec`<String>`, Vec`<String>`)>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Subcommand)]
pub enum DbJobRecreationClap {
    OnTagId(IdTimestamp),
    OnTagExist(TagClap),
    /// Spawns the job after processing
    AlwaysTime(Timestamp),
}

#[derive(Debug, Parser, Clone, Eq, PartialEq, Hash)]
pub struct IdTimestamp {
    #[arg(required = true, exclusive = true)]
    pub id: usize,
    #[arg(exclusive = false, required = true)]
    pub timestamp: Option<usize>,
}

#[derive(Debug, Parser, Clone, Eq, PartialEq, Hash)]
pub struct Timestamp {
    // Timestamp in seconds to start another job
    #[arg(exclusive = false, required = true)]
    pub timestamp: usize,
    /// Number of times to run a job
    #[arg(exclusive = false, required = false)]
    pub count: Option<usize>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Parser)]
pub struct TagClap {
    #[arg(exclusive = false, required = true)]
    pub name: String,
    #[arg(exclusive = false, required = true)]
    pub namespace: usize,
    #[arg(exclusive = false, required = true)]
    pub timestamp: Option<usize>,
}

/// Holder of job adding.
#[derive(Debug, Parser, Clone)]
pub struct JobAddStruct {
    /// Webite, Setup by nickname or by url base
    #[arg(exclusive = false, required = true)]
    pub site: String,
    /// Tag query for multiple items use " " and a space to seperate tags
    #[arg(exclusive = false, required = true)]
    pub query: String,
    /// Time, special time of now for running a job now.
    #[arg(exclusive = false, required = true)]
    pub time: String,
    /// Job type to run
    #[arg(exclusive = false, required = false)]
    pub jobtype: Option<sharedtypes::DbJobType>,
    #[arg(
        exclusive = false,
        required = false,
        value_delimiter = ',',
        value_terminator = ";",
        long = "system"
    )]
    pub system_data: Vec<String>,
    #[clap(subcommand)]
    pub recursion: Option<DbJobRecreationClap>,
}

/// Holder of job removal.
#[derive(Debug, Parser)]
pub struct JobRemovalStruct {
    /// Webite, Setup by nickname or by url base
    #[arg(exclusive = false, required = true)]
    pub site: String,
    /// Tag query for multiple items use " " and a space to seperate tags
    #[arg(exclusive = false, required = true)]
    pub query: String,
    /// Time, special time of now for running a job now.
    #[arg(exclusive = false, required = true)]
    pub time: String,
    /// TODO need to fix this later.
    #[arg(exclusive = false, required = true)]
    pub loops: String,
}

/// Search struct for parsing.
#[derive(Debug, Subcommand)]
pub enum SearchStruct {
    /// Searches By File ID.
    // #[arg(exclusive = true, required = false, long)]
    Fid(Id),
    /// Searches By Tag Id.
    // #[arg(exclusive = true, required = false, long)]
    Tid(Id),
    /// Searches By Tag name needs namespace.
    // #[arg(exclusive = true, required = false, long)]
    Tag(Tag),
    /// Searches By Hash.
    Hash(Hashy),
    /// Searches for parent relations with a tag
    Parent(Parent),
}

#[derive(Debug, Parser)]
pub struct Parent {
    #[arg(required = true, exclusive = false)]
    pub tag: String,
    #[arg(required = true, exclusive = false)]
    pub namespace: usize,
}

#[derive(Debug, Parser)]
pub struct Tag {
    #[arg(required = true, exclusive = false)]
    pub tag: String,
    #[arg(required = true, exclusive = false)]
    pub namespace: String,
}

#[derive(Debug, Parser, Clone, Eq, PartialEq, Hash)]
pub struct Id {
    #[arg(required = true, exclusive = true)]
    pub id: usize,
}

#[derive(Parser, Debug)]
pub struct Hashy {
    #[arg(required = true, exclusive = true)]
    pub hash: String,
}
