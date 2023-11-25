use clap::{arg, Args, Parser, Subcommand, ValueEnum};

///
/// From: git4unrealnondev
/// Das code sucks.
///
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct MainWrapper {
    #[command(subcommand)]
    pub a: Option<test>,
}

#[derive(Debug, Parser)]
pub enum test {
    /// Manages their jobs in db.
    #[clap(subcommand)]
    Job(JobStruct),
    /// Searches the DB.
    Search(SearchStruct),
    /// Db Tasks Structure
    #[clap(subcommand)]
    Tasks(TasksStruct),
}

#[derive(Subcommand, Debug)]
pub enum TasksStruct {
    /// Manages a CSV file.
    #[clap(subcommand)]
    Csv(CsvStruct),
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
    /// Removes a job from the system
    Remove(JobRemovalStruct),
}

/// Holder of job adding.
#[derive(Debug, Parser)]
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
    /// TODO need to fix this later.
    #[arg(exclusive = false, required = true)]
    pub committype: String,
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
#[derive(Debug, Parser)]
pub struct SearchStruct {
    /// Searches By File ID.
    #[arg(exclusive = true, required = false, long)]
    fid: usize,
    /// Searches By Tag Id.
    #[arg(exclusive = true, required = false, long)]
    tid: usize,
    /// Searches By Tag name needs namespace.
    #[arg(exclusive = true, required = false, long)]
    tag: String,
    /// Searches By Hash.
    #[arg(exclusive = true, required = false, long)]
    hash: String,
}