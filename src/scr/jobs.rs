use crate::scr::database;
use crate::scr::time;
use log::info;

pub struct Jobs {
    _jobid: Vec<u128>,
    _secs: u64,
    _sites: Vec<String>,
    _params: Vec<String>,
}

///
/// Jobs manager creates & manages jobs
///
impl Jobs {
    pub fn new() -> Self {
        Jobs {
            _jobid: Vec::new(),
            _sites: Vec::new(),
            _params: Vec::new(),
            _secs: 0,
        }
    }

    ///
    /// Checks if theirs any jobs available.
    ///
    pub fn jobs_check(&mut self) {}

    pub fn jobs_get(&mut self, mut db: database::Main) -> database::Main {
        self._secs = time::time_secs();

        let (a, b, c, d) = db.jobs_get(0);
        if a == "".to_string() && b == "".to_string() && c == "".to_string() && d =="".to_string() {
            println!("No jobs loaded in memdb.");
            info!("No jobs loaded in memdb.");
            return db;
        }

        println!("{} {} {} {}", self._secs, a, b, c);

        return db;
    }

    pub fn jobs_add(&mut self, site: String, params: Vec<String>) {}
}
