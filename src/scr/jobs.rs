use crate::database;
use crate::logging;
use crate::scraper::ScraperManager;
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
    id: usize,
    site: String,
    params: Vec<sharedtypes::ScraperParam>,
    time: usize,
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
    /// Does not append jobs into db. This is intentional as the threading module adds the jobs to
    /// db
    ///
    pub fn jobs_add(
        &mut self,
        scraper: sharedtypes::SiteStruct,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) {
        let obj = PreviouslySeenObj {
            id: dbjobsobj.id,
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
            crate::logging::info_log(&format!("Adding job: {:?}", &dbjobsobj));

            /*let mut unwrappy = self.db.lock().unwrap();
            unwrappy.jobs_add(
                Some(dbjobsobj.id),
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
            );*/

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
    use std::collections::{BTreeMap, HashMap};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use crate::sharedtypes::{DbJobsObj, SiteStruct};
    use crate::{database::test_database, scraper::test_scrapermanager};

    use super::Jobs;

    fn create_default() -> Jobs {
        let sc = test_scrapermanager::emulate_loaded();
        let scraper_manager = Arc::new(Mutex::new(sc));

        let dsb = test_database::setup_default_db();
        let db = Arc::new(Mutex::new(dsb));

        Jobs::new(db, scraper_manager)
    }

    ///
    /// Returns a default sitestruct
    ///
    fn return_sitestruct() -> SiteStruct {
        crate::sharedtypes::SiteStruct {
            name: "test".to_string(),
            sites: vec!["test".to_string()],
            version: 1,
            ratelimit: (1, Duration::from_secs(1)),
            should_handle_file_download: false,
            should_handle_text_scraping: false,
            login_type: vec![],
            stored_info: None,
        }
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

        job.jobs_load();
        let scraper = return_sitestruct();

        let dbjobsobj = return_dbjobsobj();
        job.jobs_add(scraper.clone(), dbjobsobj.clone());
        job.jobs_add(scraper.clone(), dbjobsobj);
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
        assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);

        let unwrappy = job.db.lock().unwrap();

        assert_eq!(unwrappy.jobs_get_all().keys().len(), 1);
    }
    #[test]
    fn jobs_remove_job() {
        let mut job = create_default();

        job.jobs_load();
        let scraper = return_sitestruct();

        let dbjobsobj = return_dbjobsobj();
        job.jobs_add(scraper.clone(), dbjobsobj.clone());
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
        assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);
        job.jobs_remove_dbjob(&scraper, &dbjobsobj);

        let unwrappy = job.db.lock().unwrap();

        assert_eq!(unwrappy.jobs_get_all().keys().len(), 0);
        assert_eq!(job.site_job.get(&scraper).unwrap().len(), 0);
    }
}
