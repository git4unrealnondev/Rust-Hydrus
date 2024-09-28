use crate::database;
use crate::logging;
use crate::plugins::PluginManager;
use crate::scraper;
use crate::scraper::InternalScraper;
use crate::sharedtypes;
use crate::sharedtypes::ScraperType;
use crate::threading;
use crate::time_func;
use ahash::AHashMap;

use log::info;

use rusqlite::Connection;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
//use std::sync::{Arc, Mutex};
use std::sync::Mutex;
pub struct Jobs {
    _jobid: Vec<u128>,
    _secs: usize,
    _sites: Vec<String>,
    _params: Vec<Vec<String>>,
    //References jobid in _inmemdb hashmap :D
    _jobstorun: Vec<usize>,
    _jobref: AHashMap<usize, (sharedtypes::DbJobsObj, scraper::InternalScraper)>,
    scrapermanager: scraper::ScraperManager,
}
/*#[derive(Debug, Clone)]
pub struct JobsRef {
    //pub _idindb: usize,       // What is my ID in the inmemdb
    pub _sites: String,       // Site that the user is querying
    pub _params: Vec<String>, // Anything that the user passes into the db.
    pub _jobsref: usize,      // reference time to when to run the job
    pub _jobstime: usize,     // reference time to when job is added
    pub _committype: CommitType,
    //pub _scraper: scraper::ScraperManager // Reference to the scraper that will be used
}*/

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
            _jobref: AHashMap::new(),
            scrapermanager: newmanager,
        }
    }

    ///
    /// Loads jobs to run into _jobref
    ///
    pub fn jobs_load(&mut self, db: &mut database::Main) {
        let mut scraper_site_map = HashMap::new();
        let mut query_scraper_map = HashMap::new();

        self._secs = time_func::time_secs();
        //let _ttl = db.jobs_get_max();
        let hashjobs = db.jobs_get_all();
        let beans = self.scrapermanager.scraper_get();

        for scraper in beans.into_iter() {
            for site in scraper._sites.clone() {
                scraper_site_map.insert(site, scraper.clone());
            }
        }
        let mut flushdb_flag = false;
        for (id, jobsobj) in hashjobs.clone() {
            // If our time is greater then time created + offset then run job.
            // Hella basic but it works need to make this better.
            if time_func::time_secs() >= jobsobj.time.unwrap() + jobsobj.reptime.unwrap() {
                if let Some(scraper) = scraper_site_map.get(&jobsobj.site) {
                    if !query_scraper_map
                        .contains_key(&(jobsobj.site.clone(), jobsobj.param.clone()))
                    {
                        query_scraper_map.insert(
                            (jobsobj.site.clone(), jobsobj.param.clone()),
                            jobsobj.clone(),
                        );

                        self._jobref.insert(id, (jobsobj.clone(), scraper.clone()));
                    } else {
                        dbg!("Dupe for job: {}", jobsobj, id);
                        db.del_from_jobs_byid(&id);
                        flushdb_flag = true;
                    }
                }
            }
        }
        // Flushes DB if we've deleted dupe jobs
        if flushdb_flag {
            db.transaction_flush();
        }

        //dbg!(db.jobs_get_isrunning());
        //dbg!(&invalidjobvec);
        //dbg!(&duplicatejobvec);
        let msg = format!(
            "Loaded {} jobs out of {} jobs. Didn't load {} Jobs due to lack of scrapers or timing.",
            &self._jobref.len(),
            db.jobs_get_max(),
            db.jobs_get_max() - self._jobref.len(),
        );
        logging::info_log(&msg);
    }

    ///
    /// Runs jobs in a much more sane matter
    ///
    pub fn jobs_run_new(
        &mut self,
        adb: &mut Arc<Mutex<database::Main>>,
        thread: &mut threading::Threads,
        _alt_connection: &mut Connection,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
    ) {
        let dba = adb.clone();
        let mut db = dba.lock().unwrap();

        //let mut name_ratelimited: AHashMap<String, (u64, Duration)> = AHashMap::new();
        let mut scraper_and_job: AHashMap<InternalScraper, Vec<sharedtypes::DbJobsObj>> =
            AHashMap::new();
        //let mut job_plus_storeddata: AHashMap<String, String> = AHashMap::new();

        // Checks if their are no jobs to run.
        if self.scrapermanager.scraper_get().is_empty() || self._jobref.is_empty() {
            println!("No jobs to run...");
            return;
        } else {
            // Loads DB into memory. Everything that hasn't been loaded already
            db.load_table(&sharedtypes::LoadDBTable::All);
        }

        // Appends ratelimited into hashmap for multithread scraper.
        for scrape in self.scrapermanager.scraper_get() {
            let name_result = db.settings_get_name(&format!("{:?}_{}", scrape._type, scrape._name));

            // Handles loading of settings into DB.Either Manual or Automatic to describe the functionallity
            if name_result.is_none() {
                let isolatedtitle = format!("{:?}_{}", scrape._type, scrape._name);

                let (_cookie, cookie_name) = self.library_cookie_needed(scrape);

                db.setting_add(
                    isolatedtitle,
                    Some("Automatic Scraper".to_owned()),
                    None,
                    Some(cookie_name),
                    true,
                );
            }
            // Loops through all jobs in the ref. Adds ref into
            for each in &self._jobref {
                let job = each.1;

                // Checks job type. If manual then scraper handles ALL calls from here on.
                // If Automatic then jobs will handle it.
                match job.1._type {
                    ScraperType::Manual => {}
                    ScraperType::Automatic => {
                        // Checks if InternalScraper types are the same data.
                        if &job.1 == scrape {
                            match scraper_and_job.entry(job.1.clone()) {
                                Entry::Vacant(e) => {
                                    e.insert(vec![job.0.clone()]);
                                }
                                Entry::Occupied(mut e) => {
                                    e.get_mut().push(job.0.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Loops through each InternalScraper and creates a thread for it.
        for each in scraper_and_job {
            let scraper = each.0;

            // Captures the libloading library from the _library.
            // Removes item from hashmap so the thread can have ownership of libloaded scraper.
            let scrap = self.scrapermanager._library.remove(&scraper).unwrap();
            let jobs = each.1;

            thread.startwork(scraper, jobs, adb, scrap, pluginmanager);
        }
    }
    ///
    /// pub fn cookie_needed(&mut self, id: usize, params: String) -> (bool, String)
    ///
    pub fn library_cookie_needed(&self, memid: &InternalScraper) -> (ScraperType, String) {
        let libloading = self.scrapermanager.returnlibloading(memid);
        scraper::cookie_need(libloading)
        //self.scrapermanager.cookie_needed(memid)
    }
}
