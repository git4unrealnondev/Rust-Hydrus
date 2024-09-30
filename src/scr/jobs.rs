use crate::database;
use crate::logging;
use crate::plugins::PluginManager;
use crate::scraper;
use crate::scraper::InternalScraper;
use crate::sharedtypes;
use crate::sharedtypes::ScraperType;
use crate::threading;
use crate::time_func;
use std::collections::{HashMap, HashSet};

use std::sync::Arc;
//use std::sync::{Arc, Mutex};
use std::sync::Mutex;
pub struct Jobs {
    //References jobid in _inmemdb hashmap :D
    pub _jobref: HashMap<scraper::InternalScraper, HashSet<sharedtypes::DbJobsObj>>,
    pub previously_seen: HashSet<(String, String)>,
    pub db: Arc<Mutex<database::Main>>,
}

///
/// Jobs manager creates & manages jobs
///
impl Jobs {
    pub fn new(db: Arc<Mutex<database::Main>>) -> Self {
        Jobs {
            _jobref: HashMap::new(),
            previously_seen: HashSet::new(),
            db,
        }
    }

    ///
    /// Returns a list of all jobs associated with a scraper
    ///
    pub fn jobs_get(&self, scraper: &scraper::InternalScraper) -> HashSet<sharedtypes::DbJobsObj> {
        match self._jobref.get(scraper) {
            Some(jobs) => jobs.clone(),
            None => HashSet::new(),
        }
    }

    pub fn jobs_remove_dbjob(
        &mut self,
        scraper: &scraper::InternalScraper,
        data: &sharedtypes::DbJobsObj,
    ) {
        match self._jobref.get_mut(scraper) {
            None => {}
            Some(joblist) => {
                crate::logging::info_log(&format!("Removing job {:?}", &data));
                joblist.remove(data);
                let mut db = self.db.lock().unwrap();
                db.del_from_jobs_byid(&data.id);
            }
        }
    }

    ///
    /// jobs_remove removes job from DB & from
    ///
    pub fn jobs_remove(
        &mut self,
        scraper: &scraper::InternalScraper,
        data: sharedtypes::ScraperData,
    ) {
        for job in self.jobs_get(scraper) {
            if data.job.site == job.site && Some(data.job.original_param.clone()) == job.param {
                crate::logging::info_log(&format!("Removing job {:?}", &job));
                let mut db = self.db.lock().unwrap();
                db.del_from_jobs_byid(&job.id);
                self._jobref.get_mut(scraper).unwrap().remove(&job);
            }
        }
    }

    ///
    /// Adds jobs to db and to previosuly seen hashset
    ///
    pub fn jobs_add(
        &mut self,
        scraper: &scraper::InternalScraper,
        data: sharedtypes::ScraperData,
        addtodb: bool,
    ) {
        if let None = self._jobref.get(scraper) {
            return;
        }

        if self
            .previously_seen
            .contains(&(data.job.site.clone(), data.job.original_param.clone()))
        {
            return;
        } else {
            self.previously_seen
                .insert((data.job.site.clone(), data.job.original_param.clone()));
        }

        let mut db = self.db.lock().unwrap();

        let jobid = db.jobs_add(
            None,
            0,
            0,
            data.job.site.clone(),
            data.job.original_param.clone(),
            addtodb,
            sharedtypes::CommitType::StopOnNothing,
            &data.job.job_type,
            data.system_data.clone(),
            data.user_data.clone(),
        );
        let jobsmanager = sharedtypes::DbJobsManager {
            jobtype: data.job.job_type,
            recreation: None,
            additionaldata: None,
        };

        let jobs_obj: sharedtypes::DbJobsObj = sharedtypes::DbJobsObj {
            id: jobid,
            time: Some(0),
            reptime: Some(0),
            site: data.job.site,
            param: Some(data.job.original_param),
            jobmanager: jobsmanager,
            committype: Some(sharedtypes::CommitType::StopOnNothing),
            isrunning: false,
            system_data: data.system_data,
            user_data: data.user_data,
        };
        crate::logging::info_log(&format!("Adding job: {:?}", &jobs_obj));
        self._jobref.get_mut(scraper).unwrap().insert(jobs_obj);
    }

    fn jobs_add_jobsobj(
        &mut self,
        scraper: &scraper::InternalScraper,
        data: sharedtypes::DbJobsObj,
        addtodb: bool,
    ) -> bool {
        if self
            .previously_seen
            .contains(&(data.site.clone(), data.param.clone().unwrap()))
        {
            return false;
        } else {
            self.previously_seen
                .insert((data.site.clone(), data.param.clone().unwrap()));
        }

        let mut db = self.db.lock().unwrap();

        let _ = db.jobs_add(
            Some(data.id),
            data.time.unwrap(),
            data.reptime.unwrap(),
            data.site.clone(),
            data.param.clone().unwrap(),
            addtodb,
            sharedtypes::CommitType::StopOnNothing,
            &data.jobmanager.jobtype,
            data.system_data.clone(),
            data.user_data.clone(),
        );
        crate::logging::info_log(&format!("Adding job: {:?}", &data));
        // self._jobref.get_mut(scraper).unwrap().insert(data);
        // Inserts job into scraper's job list
        match self._jobref.get_mut(scraper) {
            Some(jobs) => {
                jobs.insert(data);
            }
            None => {
                let mut temp = HashSet::new();
                temp.insert(data);
                self._jobref.insert(scraper.clone(), temp);
            }
        }
        if let None = self._jobref.get(scraper) {
            return false;
        }

        true
    }

    ///
    /// Loads jobs to run into _jobref
    ///
    pub fn jobs_load(&mut self, scrapermanager: &scraper::ScraperManager) {
        let mut scraper_site_map = HashMap::new();

        //self._secs = time_func::time_secs();
        //let _ttl = db.jobs_get_max();
        let hashjobs;
        {
            let mut db = self.db.lock().unwrap();
            hashjobs = db.jobs_get_all().clone();
        }
        let beans = scrapermanager.scraper_get();

        for scraper in beans.into_iter() {
            for site in scraper._sites.clone() {
                scraper_site_map.insert(site, scraper.clone());
            }
        }
        let mut flushdb_flag = false;

        let mut cnt = 0;

        for (id, jobsobj) in hashjobs.clone() {
            // If our time is greater then time created + offset then run job.
            // Hella basic but it works need to make this better.
            if time_func::time_secs() >= jobsobj.time.unwrap() + jobsobj.reptime.unwrap() {
                if let Some(scraper) = scraper_site_map.get(&jobsobj.site) {
                    if self.jobs_add_jobsobj(&scraper, jobsobj.clone(), false) {
                        cnt += 1;
                    } else {
                        dbg!("Dupe for job: {}", jobsobj, id);
                        let mut db = self.db.lock().unwrap();
                        db.del_from_jobs_byid(&id);
                        flushdb_flag = true;
                    }
                }
            }
        }

        let mut db = self.db.lock().unwrap();
        // Flushes DB if we've deleted dupe jobs
        if flushdb_flag {
            {
                db.transaction_flush();
            }
        }

        //dbg!(db.jobs_get_isrunning());
        //dbg!(&invalidjobvec);
        //dbg!(&duplicatejobvec);
        let msg = format!(
            "Loaded {} jobs out of {} jobs. Didn't load {} Jobs due to lack of scrapers or timing.",
            &cnt,
            db.jobs_get_max(),
            db.jobs_get_max() - cnt,
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
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
        scrapermanager: &scraper::ScraperManager,
    ) {
        let dba = adb.clone();
        let mut db = dba.lock().unwrap();

        //let mut name_ratelimited: HashMap<String, (u64, Duration)> = AHashMap::new();
        let mut scraper_and_job: HashMap<InternalScraper, Vec<sharedtypes::DbJobsObj>> =
            HashMap::new();
        //let mut job_plus_storeddata: HashMap<String, String> = AHashMap::new();

        // Checks if their are no jobs to run.
        if scrapermanager.scraper_get().is_empty() || self._jobref.is_empty() {
            println!("No jobs to run...");
            return;
        } else {
            // Loads DB into memory. Everything that hasn't been loaded already
            db.load_table(&sharedtypes::LoadDBTable::All);
        }

        /*// Appends ratelimited into hashmap for multithread scraper.
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
        }*/

        /* // Loops through each InternalScraper and creates a thread for it.
        for each in scraper_and_job {
            let scraper = each.0;

            // Captures the libloading library from the _library.
            // Removes item from hashmap so the thread can have ownership of libloaded scraper.
            let scrap = self.scrapermanager._library.remove(&scraper).unwrap();
            let jobs = each.1;

            thread.startwork(scraper, jobs, adb, scrap, pluginmanager);
        }*/
    }
    ///
    /// pub fn cookie_needed(&mut self, id: usize, params: String) -> (bool, String)
    ///
    pub fn library_cookie_needed(
        &self,
        memid: &InternalScraper,
        scrapermanager: scraper::ScraperManager,
    ) -> (ScraperType, String) {
        let libloading = scrapermanager.returnlibloading(memid);
        scraper::cookie_need(libloading)
        //self.scrapermanager.cookie_needed(memid)
    }
}
