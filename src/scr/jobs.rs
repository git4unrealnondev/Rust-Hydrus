use crate::database;
use crate::globalload::GlobalLoad;
use crate::logging;
use crate::sharedtypes;
use crate::time_func;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;

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
    /// Adds job into the storage
    ///
    pub fn jobs_add(
        &mut self,
        scraper: sharedtypes::GlobalPluginScraper,
        id: Option<usize>,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) {
        let mut dbjobsobj = dbjobsobj;
        if let Some(id) = id {
            dbjobsobj.id = id;
        }

        let obj = PreviouslySeenObj {
            site: dbjobsobj.site.clone(),
            params: dbjobsobj.param.clone(),
            time: dbjobsobj.time,
            reptime: dbjobsobj.reptime,
        };

        if let Some(list) = self.previously_seen.get(&scraper) {
            if list.contains(&obj) {
                return;
            }
        }

        if time_func::time_secs() >= dbjobsobj.time + dbjobsobj.reptime.unwrap() {
            if id.is_none() {
                crate::logging::info_log(&format!("Adding job: {:?}", &dbjobsobj));
                let mut unwrappy = self.db.lock().unwrap();
                let id = unwrappy.jobs_add(
                    None,
                    dbjobsobj.time.clone(),
                    dbjobsobj.reptime.clone().unwrap(),
                    dbjobsobj.site.clone(),
                    dbjobsobj.param.clone(),
                    dbjobsobj
                        .committype
                        .clone()
                        .unwrap_or(sharedtypes::CommitType::StopOnNothing),
                    dbjobsobj.system_data.clone(),
                    dbjobsobj.user_data.clone(),
                    dbjobsobj.jobmanager.clone(),
                );

                // Updates the ID field with something from the db
                dbjobsobj.id = id;
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
        self.site_job.keys().into_iter().collect()
    }

    ///
    /// Removes job from internal list and removes it from the db aswell
    ///
    pub fn jobs_remove_dbjob(
        &mut self,
        scraper: &sharedtypes::GlobalPluginScraper,
        data: &sharedtypes::DbJobsObj,
    ) {
        self.debug();
        dbg!(scraper, data);
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
        if self.site_job.is_empty() {
            logging::log(&"No jobs to run".to_string());
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
    pub fn jobs_load(&mut self, global_load: Arc<Mutex<GlobalLoad>>) {
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
            let globalload = global_load.lock().unwrap();
            for scraper in globalload.scraper_get() {
                for sites in globalload.sites_get(&scraper) {
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

            self.jobs_add(scraper.clone(), Some(job.id), job.clone());
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
            id: 0,
            time: 0,
            reptime: Some(1),
            site: "test".to_string(),
            param: vec![],
            jobmanager: crate::sharedtypes::DbJobsManager {
                jobtype: crate::sharedtypes::DbJobType::Scraper,
                recreation: None,
            },
            committype: None,
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
        job.jobs_add(scraper.clone(), Some(0), dbjobsobj.clone());
        job.jobs_add(scraper.clone(), Some(0), dbjobsobj);
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
        job.jobs_add(scraper.clone(), Some(0), dbjobsobj.clone());
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
        assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);
        job.jobs_remove_dbjob(&scraper, &dbjobsobj);

        let unwrappy = job.db.lock().unwrap();

        assert_eq!(unwrappy.jobs_get_all().keys().len(), 0);
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 0);
    }
}
