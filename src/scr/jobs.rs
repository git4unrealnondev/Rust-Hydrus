use crate::scr::database;
use crate::scr::scraper;
use ahash::AHashMap;
use crate::scr::time;
use log::info;

pub struct Jobs {
    _jobid: Vec<u128>,
    _secs: u128,
    _sites: Vec<String>,
    _params: Vec<String>,
    //References jobid in _inmemdb hashmap :D
    _jobstorun: Vec<u16>,
    scrapermanager: scraper::ScraperManager,
}

///
/// Jobs manager creates & manages jobs
///
impl Jobs {
    pub fn new(newmanager: scraper::ScraperManager) -> Self {
        Jobs {
            _jobid: Vec::new(),
            _sites: Vec::new(),
            _params: Vec::new(),
            _secs: 0,
            _jobstorun: Vec::new(),
            scrapermanager: newmanager,
        }
    }

    ///
    /// Loads jobs to run into _jobstorun
    ///
    pub fn jobs_get(&mut self, db: &database::Main) {
        self._secs = time::time_secs();
        let ttl = db.jobs_get_max();
        if ttl > 0 {
            for each in 0..ttl {
                let (a, b, c, d) = db.jobs_get(each);
                let auint = a.parse::<u128>().unwrap();
                let cuint = c.parse::<u128>().unwrap();
                let mut add = false;

                //Working with uint. CANT BE NEGATIVE.
                //oopsie, time is in future skip this.
                if cuint > auint {
                    continue;
                }
                let beans = self.scrapermanager.sites_get();
                let mut cnt = 0;
                for eacha in beans {
                    if eacha.contains(&b.to_string()) {
                        add = true;
                        continue;
                    }
                    cnt += 1;
                }
                let test = auint + cuint;
                if self._secs >= test && add {
                    self._jobstorun.push(cnt);
                    self._params.push(d);
                    self._sites.push(b);
                } else {
                    let msg = format!("Ignoring job: {}. Due to no scraper. ", &b);
                    println!("{}", msg);
                    info!("{}", msg);
                    continue;
                }
            }
        }

        let msg = format!(
            "Loaded {} jobs out of {} jobs due to time or no scraper available.",
            self._jobstorun.len(),
            db.jobs_get_max()
        );
        info!("{}", msg);
        println!("{}", msg);
        db.dbg_show_internals();
    }

    ///
    /// Runs jobs as they are needed to.
    ///
    pub fn jobs_run(&mut self, db: &mut database::Main) {
        // Sets up and checks scrapers

        let mut loaded_params: AHashMap<u128, Vec<String>> = AHashMap::new();

        if self.scrapermanager.scraper_get().len() == 0 {println!("No jobs to run..."); return}

        for each in 0..self.scrapermanager.scraper_get().len() {
            let name = self.scrapermanager.scraper_get()[each].name_get();

            dbg!(&format!("manual_{}", name));

            let name_result = db.settings_get_name(&format!("manual_{}", name));
            let each_u128: u128 = each.try_into().unwrap();
            let mut to_load = Vec::new();
            match name_result {
                Ok(_) => {println!("Dont have to add manual to db.");

                    to_load.push(self._params[each].to_string());
                    to_load.push(name_result.unwrap().1.to_string());

                    loaded_params.insert(each_u128, to_load);
                },
                Err("None") => {
                    let (cookie, cookie_name) = self.library_cookie_needed(
                        self._jobstorun[each].into(),
                        self._params[each].to_string(),
                    );
                    db.setting_add(
                        format!(
                            "manual_{}",
                            self.scrapermanager.scraper_get()[each].name_get()
                        ),
                        "Manually controlled scraper.".to_string(),
                        0,
                        cookie_name.to_string(),
                        true,
                    );
                    to_load.push(self._params[each].to_string());
                    loaded_params.insert(each_u128, to_load);

                }
                Err(&_) => continue,
            };
        }

        // setup for scraping jobs will probably outsource this to another file :D.
        for each in 0..self._jobstorun.len() {

            let each_u128: u128 = each.try_into().unwrap();
            println!(
                "Running Job: {} {} {}",
                self._jobstorun[each], self._sites[each], self._params[each]
            );

            let parzd: Vec<&str> = self._params[each].split(" ").collect::<Vec<&str>>();
            let mut parsed: Vec<String> = Vec::new();
            for a in parzd {
                parsed.push(a.to_string());
            }

            let index: usize = self._jobstorun[each].into();

            // url is the output from the designated scraper that has the correct

            let mut url: String = "".to_string();

            url =
                self.library_url_get(self._jobstorun[each].into(), &loaded_params[&each_u128]);
            dbg!(url);
            //println!("{:?}", self.library_url_dump(self._jobstorun[each].into(), self._params[each].to_string()) );
        }
        // Initilazing the scrapers.
    }

    /// ALL of the lower functions are just wrappers for the scraper library.
    /// This is nice because their's no unsafe code anywhere else inside code base.

    ///
    /// Returns a url to grab for.
    ///
    pub fn library_url_get(&mut self, memid: usize, params: &Vec<String>) -> String {
        return self.scrapermanager.url_load(memid, params.to_vec());
    }

    ///
    /// Returns a url to grab for.
    ///
    pub fn library_url_dump(&mut self, memid: usize, params: Vec<String>) -> Vec<String> {
        return self.scrapermanager.url_dump(memid, params);
    }
    ///
    /// pub fn cookie_needed(&mut self, id: usize, params: String) -> (bool, String)
    ///
    pub fn library_cookie_needed(&self, memid: usize, params: String) -> (String, String) {
        return self.scrapermanager.cookie_needed(memid, params);
    }
}
