use crate::database;
use crate::logging;
use crate::plugins::PluginManager;
use crate::scraper;
use crate::scraper::ScraperManager;
use crate::sharedtypes;
use crate::sharedtypes::SiteStruct;
use crate::threading;
use crate::time_func;
use crate::time_func::time_secs;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// use std::sync::{Arc, Mutex};
use std::sync::Mutex;
use std::time::Duration;

///
/// Holds the previously seen jobs
///
#[derive(Debug, Hash, PartialEq, Eq)]
struct PreviouslySeenObj {
    id: usize,
    site: String,
    original_param: String,
    time: Option<usize>,
    reptime: Option<usize>,
}

pub struct Jobs {
    db: Arc<Mutex<database::Main>>,
    scraper_manager: Arc<Mutex<ScraperManager>>,
    site_job: HashMap<sharedtypes::SiteStruct, HashSet<sharedtypes::DbJobsObj>>,
    previously_seen: HashMap<sharedtypes::SiteStruct, HashSet<PreviouslySeenObj>>,
}

impl Jobs {
    pub fn new(
        db: Arc<Mutex<database::Main>>,
        scraper_manager: Arc<Mutex<ScraperManager>>,
    ) -> Self {
        Jobs {
            db,
            scraper_manager,
            site_job: HashMap::new(),
            previously_seen: HashMap::new(),
        }
    }

    pub fn debug(&self) {
        dbg!(&self.site_job, &self.previously_seen);
    }
    ///
    /// Adds job into the storage
    ///
    pub fn jobs_add(
        &mut self,
        scraper: sharedtypes::SiteStruct,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) {
        let obj = PreviouslySeenObj {
            id: dbjobsobj.id,
            site: dbjobsobj.site.clone(),
            original_param: dbjobsobj.param.clone().unwrap_or("".to_string()),
            time: dbjobsobj.time,
            reptime: dbjobsobj.reptime,
        };

        if let Some(list) = self.previously_seen.get(&scraper) {
            if list.contains(&obj) {
                return;
            }
        }

        if time_func::time_secs() >= dbjobsobj.time.unwrap() + dbjobsobj.reptime.unwrap() {
            crate::logging::info_log(&format!("Adding job: {:?}", &dbjobsobj));

            match self.previously_seen.get_mut(&scraper) {
                Some(list) => {
                    list.insert(obj);
                }
                None => {
                    let mut temp = HashSet::new();
                    temp.insert(obj);
                    self.previously_seen.insert(scraper.clone(), temp);
                }
            }
            match self.site_job.get_mut(&scraper) {
                Some(list) => {
                    list.insert(dbjobsobj);
                }
                None => {
                    let mut temp = HashSet::new();
                    temp.insert(dbjobsobj);
                    self.site_job.insert(scraper, temp);
                }
            }
        }
    }

    ///
    /// Gets if a job exists inside for a scraper
    ///
    pub fn jobs_get(&self, scraper: &sharedtypes::SiteStruct) -> HashSet<sharedtypes::DbJobsObj> {
        self.site_job
            .get(scraper)
            .cloned()
            .unwrap_or(HashSet::new())
    }
    ///
    /// Gets all the sitestruct objs that are loaded for jobs
    ///
    pub fn job_scrapers_get(&self) -> HashSet<&sharedtypes::SiteStruct> {
        self.site_job.keys().into_iter().collect()
    }

    ///
    /// Removes job from internal list and removes it from the db aswell
    ///
    pub fn jobs_remove_dbjob(
        &mut self,
        scraper: &sharedtypes::SiteStruct,
        data: &sharedtypes::DbJobsObj,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper) {
            let job_list_static = job_list.clone();
            for job in job_list_static {
                if job.id == data.id && job_list.remove(&job) {
                    let mut db = self.db.lock().unwrap();
                    db.del_from_jobs_byid(&job.id);
                }
            }
        }
    }

    ///
    /// Decrements count if this is applicable to job
    ///
    pub fn jobs_decrement_count(
        &mut self,
        data: &sharedtypes::DbJobsObj,
        scraper: &sharedtypes::SiteStruct,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper) {
            if let Some(job) = job_list.get(data) {
                dbg!(job);

                if let Some(recursion) = &job.jobmanager.recreation {
                    match recursion {
                        sharedtypes::DbJobRecreation::OnTag(_, _, _) => {}
                        sharedtypes::DbJobRecreation::OnTagId(_, _) => {}
                        sharedtypes::DbJobRecreation::AlwaysTime(_, count) => {
                            if let Some(mut count) = count {
                                if count <= 0 {
                                    self.jobs_remove_dbjob(scraper, data);
                                } else {
                                    count -= 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ///
    /// Clears the previously seen cache if the site_job contains the scraper
    ///
    pub fn clear_previously_seen_cache(&mut self, scraper: &sharedtypes::SiteStruct) {
        if self.site_job.remove(scraper).is_some() {
            self.previously_seen.clear();
        }
    }

    // pub fn jobs_remove(&mut self, scraper: &sharedtypes::SiteStruct, data: &sharedtypes::ScraperData) {
    //
    // }

    ///
    /// Determines if we need to load tables to run job
    /// Might add the login info here if I can
    ///
    pub fn jobs_run_new(&mut self) {
        if self.site_job.is_empty() {
            logging::log(&format!("No jobs to run"));
        } else {
            self.db
                .lock()
                .unwrap()
                .load_table(&sharedtypes::LoadDBTable::All);
        }
    }

    ///
    ///Loads jobs from DB into the internal Jobs structure
    ///
    pub fn jobs_load(&mut self) {
        // Loads table if this hasn't been loaded yet
        self.db
            .lock()
            .unwrap()
            .load_table(&sharedtypes::LoadDBTable::Jobs);

        let mut hashjobs;
        {
            let db = self.db.lock().unwrap();
            hashjobs = db.jobs_get_all().clone();
        }

        let mut jobs_vec = Vec::new();
        {
            let scrapermanager = self.scraper_manager.lock().unwrap();
            for scraper in scrapermanager.scraper_get() {
                for sites in scrapermanager.sites_get(scraper) {
                    for (id, job) in hashjobs.iter() {
                        if sites == job.site {
                            jobs_vec.push((scraper.clone(), *id, job.clone()));
                        }
                    }
                }
            }
        }
        for (scraper, id, job) in jobs_vec {
            hashjobs.remove(&id);

            self.jobs_add(scraper.clone(), job.clone());
        }
    }
}

#[cfg(test)]

mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{database::test_database, scraper::test_scrapermanager};

    use super::Jobs;

    fn create_default() -> Jobs {
        let sc = test_scrapermanager::emulate_loaded();
        let scraper_manager = Arc::new(Mutex::new(sc));

        let dsb = test_database::setup_default_db();
        let db = Arc::new(Mutex::new(dsb));

        Jobs::new(db, scraper_manager)
    }

    #[test]
    fn insert_duplicate_job() {
        let mut job = create_default();

        job.jobs_load();
    }
}
