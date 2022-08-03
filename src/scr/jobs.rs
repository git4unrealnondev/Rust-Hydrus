use crate::scr::database;
use crate::scr::time;
use log::info;

pub struct Jobs {
    _jobid: Vec<u128>,
    _secs: u128,
    _sites: Vec<String>,
    _params: Vec<String>,
    //References jobid in _inmemdb hashmap :D
    _jobstorun: Vec<u16>,
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
            _jobstorun: Vec::new(),
        }
    }

    ///
    /// Loads jobs to run into _jobstorun
    ///
    pub fn jobs_get(&mut self, db: &database::Main) {
        self._secs = time::time_secs();

        let jobs_to_run: Vec<u16> = Vec::new();

        let ttl = db.jobs_get_max();
        if ttl > 0 {
            for each in 0..ttl {
                let (a, b, c, d) = db.jobs_get(each);
                let auint = a.parse::<u128>().unwrap();
                let mut cuint = c.parse::<u128>().unwrap();

                //Working with uint. CANT BE NEGATIVE.
                //oopsie, time is in future skip this.
                if cuint > auint {continue}
                let test = auint - cuint;
                if self._secs >= test {self._jobstorun.push(each.try_into().unwrap()); self._params.push(d); self._sites.push(b);}
            }
        }

        let msg = format!("Loaded {} jobs.", self._jobstorun.len());
        info!("{}",msg);
        println!("{}", msg)
    }

    ///
    /// Runs jobs as they are needed to.
    ///
    pub fn jobs_run(self) {
        for each in 0..self._jobstorun.len() {println!("{} {} {}", self._jobstorun[each], self._sites[each], self._params[each]);}
    }

}
