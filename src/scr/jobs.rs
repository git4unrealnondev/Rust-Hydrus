use crate::database;
use crate::logging;
use crate::sharedtypes;
use crate::time_func;
use std::collections::{HashMap, HashSet};
//use std::sync::Mutex;

///
/// Holds the previously seen jobs
///
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct PreviouslySeenObj {
    site: String,
    params: Vec<sharedtypes::ScraperParam>,
    time: usize,
    reptime: Option<usize>,
}

#[derive(Clone)]
pub struct Jobs {
    db: database::Main,
    site_job: HashMap<sharedtypes::GlobalPluginScraper, HashSet<sharedtypes::DbJobsObj>>,
    previously_seen: HashMap<sharedtypes::GlobalPluginScraper, HashSet<PreviouslySeenObj>>,
}

impl Jobs {
    pub fn new(db: database::Main) -> Self {
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
    ) -> bool {
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
            return false;
        }

        if let Some(list) = self.previously_seen.get(&scraper) {
            // If we match directly then we should be good
            if list.contains(&obj) {
                /*logging::info_log(&format!(
                    "Skipping obj because I've seen it already: {:?}",
                    &obj
                ));*/
                return false;
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
                            return false;
                        }
                    }
                }
            }
        }
        let mut out = false;
        if time_func::time_secs() >= dbjobsobj.time + dbjobsobj.reptime.unwrap() {
            if dbjobsobj.id.is_none() {
                let mut temp = dbjobsobj.clone();
                temp.id = None;
                let id = self.db.jobs_add_new(temp);
                out = true;
                // Updates the ID field with something from the db
                dbjobsobj.id = Some(id);
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
            crate::logging::info_log(format!("Adding job: {:?}", &dbjobsobj));
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
        out
    }

    pub fn jobs_add_nolock(
        &mut self,

        scraper: sharedtypes::GlobalPluginScraper,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) {
        self.jobs_add_internal(scraper, dbjobsobj);
    }

    ///
    /// Adds job into the storage
    ///
    pub fn jobs_add(
        &mut self,

        scraper: sharedtypes::GlobalPluginScraper,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) -> bool {
        self.jobs_add_internal(scraper, dbjobsobj)
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
    /// Gets a list of jobs with the highest priority first.
    ///
    pub fn jobs_get_priority_order(
        &self,
        scraper: &sharedtypes::GlobalPluginScraper,
    ) -> Vec<sharedtypes::DbJobsObj> {
        let mut out = Vec::new();
        for job in self.jobs_get(scraper) {
            out.push(job);
        }
        out.sort_by_key(|key| key.priority);
        out.reverse();

        out
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
        worker_id: &usize,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper) {
            let job_list_static = job_list.clone();
            for job in job_list_static {
                if job.id == data.id && job_list.remove(&job) {
                    logging::info_log(format!("Worker: {worker_id} --Removing Job: {:?}", &job));
                    self.db.del_from_jobs_byid(job.id.as_ref());
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
        worker_id: &usize,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper)
            && let Some(job) = job_list.get(data)
            && let Some(recursion) = &job.jobmanager.recreation
        {
            match recursion {
                sharedtypes::DbJobRecreation::OnTag(_, _, _) => {}
                sharedtypes::DbJobRecreation::OnTagId(_, _) => {}
                sharedtypes::DbJobRecreation::AlwaysTime(_, count) => {
                    if let &Some(count) = count {
                        if count == 0 {
                            self.jobs_remove_dbjob(scraper, data, worker_id);
                        } else {
                            //count -= 1;
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
    pub fn jobs_run_new(&self) {
        logging::info_log("Checking if we have any Jobs to run.".to_string());
        if self.site_job.is_empty() {
            logging::info_log("No jobs to run".to_string());
        } else {
            for (scraper, jobsobj) in self.site_job.iter() {
                logging::info_log(format!(
                    "Scraper: {} has {} jobs to run.",
                    scraper.name,
                    jobsobj.len()
                ));
            }
        }
    }

    ///
    ///Loads jobs from DB into the internal Jobs structure
    ///
    pub fn jobs_load(
        &mut self,
        globalplugin_sites: Vec<(sharedtypes::GlobalPluginScraper, String)>,
    ) {
        let hashjobs;
        let mut jobs_vec = Vec::new();

        {
            hashjobs = self.db.jobs_get_all().clone();
        }

        {
            'mainloop: for (scraper, sites) in globalplugin_sites {
                // Stupid prefilter because an item can be either a scraper or a plugin. Not sure how I
                // didn't hit this issue sooner lol
                if let Some(sharedtypes::ScraperOrPlugin::Scraper(_)) = scraper.storage_type {
                } else {
                    continue 'mainloop;
                }

                //for sites in global_load.read().sites_get(&scraper) {
                for (_, job) in hashjobs.iter() {
                    if sites == job.site && job.priority != 0 {
                        jobs_vec.push((scraper.clone(), job.clone()));
                    }
                }
                //}
            }
        }

        // Sorts by priority
        jobs_vec.sort_by_key(|job| job.1.priority);

        let jobs_vec: Vec<(sharedtypes::GlobalPluginScraper, sharedtypes::DbJobsObj)> = jobs_vec
            .into_iter()
            .filter(|job| job.1.priority != 0)
            .collect();
        let mut commit = false;
        for (scraper, job) in jobs_vec {
            if self.jobs_add(scraper, job) {
                commit = true;
            }
        }
        if commit {
            self.db.transaction_flush();
        }
    }
}

#[cfg(test)]
pub(crate) mod test_database {
    use crate::RwLock;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::database::Main;
    use crate::database::test_database;
    use crate::globalload::{GlobalLoad, test_globalload};
    use crate::sharedtypes::DbJobsObj;
    use crate::sharedtypes::{self, DEFAULT_PRIORITY};

    use super::Jobs;

    ///
    /// Creates a default job obj to use
    ///
    pub fn create_default() -> Vec<Jobs> {
        let mut out = Vec::new();
        let dsb = test_database::setup_default_db();
        for db in dsb {
            out.push(Jobs::new(db));
        }
        out
    }

    ///
    /// Sets up for globalloading
    ///
    fn get_globalload(db: Main, jobs: Arc<RwLock<Jobs>>) -> GlobalLoad {
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
        for mut job in create_default() {
            let globalload = get_globalload(job.db.clone(), Arc::new(RwLock::new(job.clone())));
            let globalload_sites = globalload.return_all_sites();
            job.jobs_load(globalload_sites);
            let mut scraper = sharedtypes::return_default_globalpluginparser();
            scraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
                sharedtypes::ScraperInfo {
                    ratelimit: (1, Duration::from_secs(1)),
                    sites: vec!["Nah".to_string()],
                    priority: DEFAULT_PRIORITY,
                    num_threads: None,
                    modifiers: vec![],
                },
            ));

            let dbjobsobj = return_dbjobsobj();
            job.jobs_add(scraper.clone(), dbjobsobj.clone());
            job.jobs_add(scraper.clone(), dbjobsobj);
            assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
            assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);

            assert_eq!(job.jobs_get(&scraper).len(), 1);
        }
    }
    #[test]
    fn jobs_remove_job() {
        for mut job in create_default() {
            let globalload = get_globalload(job.db.clone(), Arc::new(RwLock::new(job.clone())));
            let globalload_sites = globalload.return_all_sites();
            job.jobs_load(globalload_sites);
            let mut scraper = sharedtypes::return_default_globalpluginparser();
            scraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
                sharedtypes::ScraperInfo {
                    ratelimit: (1, Duration::from_secs(1)),
                    sites: vec!["Nah".to_string()],
                    priority: DEFAULT_PRIORITY,
                    num_threads: None,
                    modifiers: vec![],
                },
            ));

            let dbjobsobj = return_dbjobsobj();
            job.jobs_add(scraper.clone(), dbjobsobj.clone());
            job.debug();
            assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
            assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);
            job.jobs_remove_dbjob(&scraper, &dbjobsobj, &0);

            let unwrappy = job.db;
            assert_eq!(unwrappy.jobs_get_all().keys().len(), 0);
            assert_eq!(job.site_job.get(&scraper).unwrap().len(), 0);
        }
    }
}
