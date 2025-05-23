use crate::database;
use crate::database::Main;
use crate::globalload::GlobalLoad;
use crate::logging;
use crate::sharedtypes;
use crate::time_func;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

///
/// Holds the previously seen jobs
///
#[derive(Debug, Hash, PartialEq, Eq)]
struct PreviouslySeenObj {
    site: String,
    params: Vec<sharedtypes::ScraperParam>,
    time: usize,
    reptime: Option<usize>,
}

pub struct Jobs {
    db: Arc<Mutex<database::Main>>,
    site_job: HashMap<sharedtypes::GlobalPluginScraper, HashSet<sharedtypes::DbJobsObj>>,
    previously_seen: HashMap<sharedtypes::GlobalPluginScraper, HashSet<PreviouslySeenObj>>,
}

impl Jobs {
    pub fn new(db: Arc<Mutex<database::Main>>) -> Self {
        Jobs {
            db,
            site_job: HashMap::new(),
            previously_seen: HashMap::new(),
        }
    }

    pub fn debug(&self) {
        dbg!(&self.site_job, &self.previously_seen);
    }

    ///
    /// Actual job adding logic. Only way to get around Mutex lock of the DB when regex gets
    /// called.
    ///
    fn jobs_add_internal(
        &mut self,
        scraper: sharedtypes::GlobalPluginScraper,
        dbjobsobj: sharedtypes::DbJobsObj,
        db: &mut Main,
    ) {
        let mut dbjobsobj = dbjobsobj;

        let obj = PreviouslySeenObj {
            site: dbjobsobj.site.clone(),
            params: dbjobsobj.param.clone(),
            time: dbjobsobj.time,
            reptime: dbjobsobj.reptime,
        };
        // Stupid prefilter because an item can be either a scraper or a plugin. Not sure how I
        // didn't hit this issue sooner lol
        // Have to filter here because if a regex or something gets parsed as the "owner" of this
        // call then the program can poop itself
        if let Some(sharedtypes::ScraperOrPlugin::Scraper(_)) = scraper.storage_type {
        } else {
            return;
        }

        if let Some(list) = self.previously_seen.get(&scraper) {
            // If we match directly then we should be good
            if list.contains(&obj) {
                return;
            }

            for job_to_check in list {
                match dbjobsobj.cachechecktype {
                    sharedtypes::JobCacheType::TimeReptimeParam => {}
                    sharedtypes::JobCacheType::Param => {
                        dbg!(
                            &job_to_check.params,
                            &dbjobsobj.param,
                            job_to_check.params == dbjobsobj.param
                        );
                        if job_to_check.params == dbjobsobj.param {
                            dbg!("SKIPPING");
                            return;
                        }
                    }
                }
            }
        }

        if time_func::time_secs() >= dbjobsobj.time + dbjobsobj.reptime.unwrap() {
            if dbjobsobj.id.is_none() {
                let mut temp = dbjobsobj.clone();
                temp.id = None;
                let id = db.jobs_add_new(temp);

                // Updates the ID field with something from the db
                dbjobsobj.id = Some(id);
                crate::logging::info_log(&format!("Adding job: {:?}", &dbjobsobj));
            }

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

    pub fn jobs_add_nolock(
        &mut self,
        scraper: sharedtypes::GlobalPluginScraper,
        dbjobsobj: sharedtypes::DbJobsObj,
        db: &mut Main,
    ) {
        self.jobs_add_internal(scraper, dbjobsobj, db);
    }

    ///
    /// Adds job into the storage
    ///
    pub fn jobs_add(
        &mut self,
        scraper: sharedtypes::GlobalPluginScraper,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) {
        self.jobs_add_internal(scraper, dbjobsobj, &mut self.db.clone().lock().unwrap());
    }

    ///
    /// Gets if a job exists inside for a scraper
    ///
    pub fn jobs_get(
        &self,
        scraper: &sharedtypes::GlobalPluginScraper,
    ) -> HashSet<sharedtypes::DbJobsObj> {
        self.site_job
            .get(scraper)
            .cloned()
            .unwrap_or(HashSet::new())
    }

    ///
    /// Gets all the GlobalPluginScraper objs that are loaded for jobs
    ///
    pub fn job_scrapers_get(&self) -> HashSet<&sharedtypes::GlobalPluginScraper> {
        self.site_job.keys().collect()
    }

    ///
    /// Removes job from internal list and removes it from the db aswell
    ///
    pub fn jobs_remove_dbjob(
        &mut self,
        scraper: &sharedtypes::GlobalPluginScraper,
        data: &sharedtypes::DbJobsObj,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper) {
            let job_list_static = job_list.clone();
            for job in job_list_static {
                if job.id == data.id && job_list.remove(&job) {
                    let mut db = self.db.lock().unwrap();
                    db.del_from_jobs_byid(job.id.as_ref());
                }
            }
        }
    }

    ///
    /// Removes from the internal list only.
    /// Does not touch DB
    ///
    pub fn jobs_remove_job(
        &mut self,
        scraper: &sharedtypes::GlobalPluginScraper,
        data: &sharedtypes::DbJobsObj,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper) {
            job_list.remove(data);
        }
    }

    ///
    /// Decrements count if this is applicable to job
    ///
    pub fn jobs_decrement_count(
        &mut self,
        data: &sharedtypes::DbJobsObj,
        scraper: &sharedtypes::GlobalPluginScraper,
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
                                if count == 0 {
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
    pub fn clear_previously_seen_cache(&mut self, scraper: &sharedtypes::GlobalPluginScraper) {
        if self.site_job.remove(scraper).is_some() {
            self.previously_seen.clear();
        }
    }

    // pub fn jobs_remove(&mut self, scraper: &sharedtypes::GlobalPluginScraper, data: &sharedtypes::ScraperData) {
    //
    // }

    ///
    /// Determines if we need to load tables to run job
    /// Might add the login info here if I can
    ///
    pub fn jobs_run_new(&mut self) {
        logging::info_log(&"Checking if we have any Jobs to run.".to_string());
        if self.site_job.is_empty() {
            logging::info_log(&"No jobs to run".to_string());
        } else {
            for (scraper, jobsobj) in self.site_job.iter() {
                logging::info_log(&format!(
                    "Scraper: {} has {} jobs to run.",
                    scraper.name,
                    jobsobj.len()
                ));
            }

            self.db
                .lock()
                .unwrap()
                .load_table(&sharedtypes::LoadDBTable::All);
        }
    }

    ///
    ///Loads jobs from DB into the internal Jobs structure
    ///
    pub fn jobs_load(&mut self, global_load: Arc<RwLock<GlobalLoad>>) {
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
            'mainloop: for scraper in global_load.read().unwrap().scraper_get() {
                // Stupid prefilter because an item can be either a scraper or a plugin. Not sure how I
                // didn't hit this issue sooner lol
                if let Some(sharedtypes::ScraperOrPlugin::Scraper(_)) = scraper.storage_type {
                } else {
                    continue 'mainloop;
                }

                for sites in global_load.read().unwrap().sites_get(&scraper) {
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
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use crate::database::test_database;
    use crate::database::Main;
    use crate::globalload::{test_globalload, GlobalLoad};
    use crate::sharedtypes;
    use crate::sharedtypes::DbJobsObj;

    use super::Jobs;

    ///
    /// Creates a default job obj to use
    ///
    pub fn create_default() -> Jobs {
        let dsb = test_database::setup_default_db();
        let db = Arc::new(Mutex::new(dsb));

        Jobs::new(db)
    }

    ///
    /// Sets up for globalloading
    ///
    fn get_globalload(db: Arc<Mutex<Main>>, jobs: Arc<Mutex<Jobs>>) -> Arc<Mutex<GlobalLoad>> {
        test_globalload::emulate_loaded(db, jobs)
    }

    ///
    /// Returns a default jobsobj
    ///
    fn return_dbjobsobj() -> DbJobsObj {
        crate::sharedtypes::DbJobsObj {
            id: Some(0),
            time: 0,
            reptime: Some(1),
            priority: sharedtypes::DEFAULT_PRIORITY,
            cachetime: sharedtypes::DEFAULT_CACHETIME,
            cachechecktype: sharedtypes::DEFAULT_CACHECHECK,
            site: "test".to_string(),
            param: vec![],
            jobmanager: crate::sharedtypes::DbJobsManager {
                jobtype: crate::sharedtypes::DbJobType::Scraper,
                recreation: None,
            },
            isrunning: false,
            system_data: BTreeMap::new(),
            user_data: BTreeMap::new(),
        }
    }

    #[test]
    fn insert_duplicate_job() {
        let mut job = create_default();
        let globalload = get_globalload(job.db.clone(), Arc::new(Mutex::new(create_default())));

        job.jobs_load(globalload);
        let scraper = sharedtypes::return_default_globalpluginparser();

        let dbjobsobj = return_dbjobsobj();
        job.jobs_add(scraper.clone(), dbjobsobj.clone());
        job.jobs_add(scraper.clone(), dbjobsobj);
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
        assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);

        let unwrappy = job.db.lock().unwrap();
        dbg!(job.jobs_get(&scraper));
        assert_eq!(job.jobs_get(&scraper).len(), 1);
    }
    #[test]
    fn jobs_remove_job() {
        let mut job = create_default();

        let globalload = get_globalload(job.db.clone(), Arc::new(Mutex::new(create_default())));
        job.jobs_load(globalload);
        let scraper = sharedtypes::return_default_globalpluginparser();

        let dbjobsobj = return_dbjobsobj();
        job.jobs_add(scraper.clone(), dbjobsobj.clone());
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
        assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);
        job.jobs_remove_dbjob(&scraper, &dbjobsobj);

        let unwrappy = job.db.lock().unwrap();
        dbg!(unwrappy.jobs_get_all(), &dbjobsobj);
        assert_eq!(unwrappy.jobs_get_all().keys().len(), 0);
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 0);
    }
}
