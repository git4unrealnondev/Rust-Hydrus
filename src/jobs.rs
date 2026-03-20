use crate::Main;
use crate::database;
use crate::logging;
use crate::sharedtypes;
use crate::time_func;
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
//use std::sync::Mutex;

///
/// Holds the previously seen jobs
///
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct PreviouslySeenObj {
    site: String,
    params: Vec<sharedtypes::ScraperParam>,
    time: u64,
    reptime: u64,
    user_data: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct Jobs {
    db: Main,
    site_job: HashMap<sharedtypes::GlobalPluginScraper, HashSet<sharedtypes::DbJobsObj>>,
    previously_seen: HashMap<sharedtypes::GlobalPluginScraper, HashSet<PreviouslySeenObj>>,
}

impl Jobs {
    pub fn new(db: Main) -> Self {
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
    /// Returns the job id number if it exists
    ///
    fn jobs_add_internal(
        &mut self,

        scraper: sharedtypes::GlobalPluginScraper,
        dbjobsobj: sharedtypes::DbJobsObj,
    ) -> Option<u64> {
        let mut dbjobsobj = dbjobsobj;

        let obj = PreviouslySeenObj {
            site: dbjobsobj.site.clone(),
            params: dbjobsobj.param.clone(),
            time: dbjobsobj.time,
            reptime: dbjobsobj.reptime,
            user_data: dbjobsobj.user_data.clone(),
        };

        // Stupid prefilter because an item can be either a scraper or a plugin. Not sure how I
        // didn't hit this issue sooner lol
        // Have to filter here because if a regex or something gets parsed as the "owner" of this
        // call then the program can poop itself
        if let Some(sharedtypes::ScraperOrPlugin::Scraper(_)) = scraper.storage_type {
        } else {
            return None;
        }

        if let Some(list) = self.previously_seen.get(&scraper) {
            // If we match directly then we should be good
            if list.contains(&obj) {
                /*logging::info_log(&format!(
                    "Skipping obj because I've seen it already: {:?}",
                    &obj
                ));*/
                return None;
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
                            return None;
                        }
                    }
                }
            }
        }
        let mut out = None;
        if time_func::time_secs() >= dbjobsobj.time + dbjobsobj.reptime {
            if dbjobsobj.id.is_none() {
                let mut temp = dbjobsobj.clone();
                temp.id = None;
                let id = self.db.jobs_add_new(temp);
                out = Some(id);
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
    ) -> Option<u64> {
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

        out.truncate(10);
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
        worker_id: &u64,
    ) {
        if let Some(job_list) = self.site_job.get_mut(scraper) {
            let job_list_static = job_list.clone();
            for job in job_list_static {
                if job.id == data.id && job_list.remove(&job) {
                    logging::info_log(format!("Worker: {worker_id} --Removing Job: {:?}", &job));
                    self.db.del_from_jobs_byid(job.id);
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
        worker_id: &u64,
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
        use std::collections::HashMap;

        let hashjobs = self.db.jobs_get_all().clone();

        // job_id -> (best_scraper, job)
        let mut best_jobs: HashMap<
            u64,
            (sharedtypes::GlobalPluginScraper, sharedtypes::DbJobsObj),
        > = HashMap::new();

        for (_, job) in hashjobs.iter() {
            if job.priority == 0 {
                continue;
            }

            let job_id = match job.id {
                Some(id) => id,
                None => continue,
            };

            for (scraper, site) in &globalplugin_sites {
                // Only allow actual scrapers
                let scraper_info = match &scraper.storage_type {
                    Some(sharedtypes::ScraperOrPlugin::Scraper(info)) => info,
                    _ => continue,
                };

                if site != &job.site {
                    continue;
                }

                best_jobs
                    .entry(job_id)
                    .and_modify(|(existing_scraper, _)| {
                        let existing_priority = match &existing_scraper.storage_type {
                            Some(sharedtypes::ScraperOrPlugin::Scraper(info)) => info.priority,
                            _ => 0,
                        };

                        if scraper_info.priority > existing_priority {
                            *existing_scraper = scraper.clone();
                        }
                    })
                    .or_insert((scraper.clone(), job.clone()));
            }
        }

        let mut jobs_vec: Vec<_> = best_jobs.into_values().collect();

        // Optional: sort by scraper priority (higher first)
        jobs_vec.sort_by_key(|(scraper, _)| match &scraper.storage_type {
            Some(sharedtypes::ScraperOrPlugin::Scraper(info)) => std::cmp::Reverse(info.priority),
            _ => std::cmp::Reverse(0),
        });

        let mut commit = false;

        for (scraper, job) in jobs_vec.iter_mut() {
            self.process_logintype(job, scraper);
            if self.jobs_add(scraper.clone(), job.clone()).is_some() {
                commit = true;
            }
        }

        if commit {
            self.db.transaction_flush();
        }
    }

    /// Gets all logins for the the job that are needed
    fn process_logintype(
        &self,
        job: &mut sharedtypes::DbJobsObj,
        scraper: &sharedtypes::GlobalPluginScraper,
    ) {
        for (key, login, login_needed, help_text, overwrite_db_entry) in scraper.login_type.iter() {
            match login {
                sharedtypes::LoginType::Api(_name, _api_key) => {
                    todo!();
                }
                sharedtypes::LoginType::ApiNamespaced(name, api_namespace, api_body) => {
                    /* {
                        if *overwrite_db_entry && api_namespace.is_some() {
                            self.db.setting_add(
                                format!("API_NAMESPACED_NAMESPACE_{}_{}", key, name),
                                None,
                                None,
                                api_namespace.clone(),
                            );
                        }
                        if *overwrite_db_entry && api_body.is_some() {
                            self.db.setting_add(
                                format!("API_NAMESPACED_BODY_{}_{}", key, name),
                                None,
                                None,
                                api_body.clone(),
                            );
                        }
                    }*/
                    let ns_stored = if let Some(setting_obj) = self
                        .db
                        .settings_get_name(&format!("API_NAMESPACED_NAMESPACE_{}_{}", key, name))
                    {
                        setting_obj.param.clone()
                    } else {
                        None
                    };
                    let body_stored = if let Some(setting_obj) = self
                        .db
                        .settings_get_name(&format!("API_NAMESPACED_BODY_{}_{}", key, name))
                    {
                        setting_obj.param.clone()
                    } else {
                        None
                    };

                    // If we have nothing in either of these then we likely need to
                    // pull info from the plugins this is used to pull login info
                    // from an external plugin like a web interface or UI
                    if (ns_stored.is_none() | body_stored.is_none())
                        & (*login_needed == sharedtypes::LoginNeed::Required)
                    {
                        dbg!(&login_needed);
                        todo!();
                    }

                    // Special actions need to be taken if we are logging into
                    // something that requires a login
                    if *login_needed == sharedtypes::LoginNeed::Required {
                        break;
                    }

                    /* if ns_stored.is_some() | body_stored.is_some() {
                        let apins = sharedtypes::LoginType::ApiNamespaced(
                            name.to_string(),
                            ns_stored.into(),
                            body_stored.into(),
                        );
                        job.param.push(sharedtypes::ScraperParam::Login(apins));
                    }*/
                }
                sharedtypes::LoginType::Cookie(_name, _cookie) => {
                    todo!();
                }
                sharedtypes::LoginType::Other(_name, _other) => {
                    todo!();
                }
                sharedtypes::LoginType::Login(name, login_info) => {
                    let username_string = format!("{}_username", name);
                    let password_string = format!("{}_password", name);
                    let username = self.db.settings_get_name(&username_string);
                    let password = self.db.settings_get_name(&password_string);

                    match (username, password) {
                        (Some(username), Some(password)) => {
                            job.param.push(sharedtypes::ScraperParam::Login(
                                sharedtypes::LoginType::Login(
                                    name.clone(),
                                    Some(sharedtypes::LoginUsernameOrPassword {
                                        username: username.param.unwrap().into(),
                                        password: password.param.unwrap().into(),
                                    }),
                                ),
                            ));
                        }
                        (Some(username), None) => {}
                        (None, Some(password)) => {}
                        (None, None) => {
                            if sharedtypes::LoginNeed::Required == *login_needed {
                                if let Some(help_text) = help_text {
                                    logging::info_log(help_text);
                                }

                                logging::info_log("Username for site: ");

                                let mut input = String::new();
                                std::io::stdin()
                                    .read_line(&mut input)
                                    .expect("Failed to read line");
                                self.db.setting_add(
                                    username_string.clone(),
                                    Some("Username for site.".into()),
                                    None,
                                    Some(input.trim().into()),
                                );
                                logging::info_log("Password for site: ");
                                let mut input = String::new();
                                std::io::stdin()
                                    .read_line(&mut input)
                                    .expect("Failed to read line");

                                self.db.setting_add(
                                    password_string.clone(),
                                    Some("Password for site.".into()),
                                    None,
                                    Some(input.trim().into()),
                                );

                                job.param.push(sharedtypes::ScraperParam::Login(
                                    sharedtypes::LoginType::Login(
                                        name.to_string(),
                                        Some(sharedtypes::LoginUsernameOrPassword {
                                            username: username_string.into(),
                                            password: password_string.into(),
                                        }),
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_database {
    use crate::RwLock;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::Main;
    use crate::database::database::test_database;
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
            reptime: 1,
            site: "test".to_string(),
            jobmanager: crate::sharedtypes::DbJobsManager {
                jobtype: crate::sharedtypes::DbJobType::Scraper,
                ..Default::default()
            },
            ..Default::default()
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

            let mut dbjobsobj = return_dbjobsobj();
            dbjobsobj.id = None;
            let jobid = job.jobs_add(scraper.clone(), dbjobsobj.clone()).unwrap();
            assert_eq!(job.site_job.get(&scraper).unwrap().len(), 1);
            assert_eq!(job.previously_seen.get(&scraper).unwrap().len(), 1);
            dbjobsobj.id = Some(jobid);
            job.jobs_remove_dbjob(&scraper, &dbjobsobj, &0);
            let unwrappy = job.db;
            dbg!(unwrappy.jobs_get_all());
            assert_eq!(unwrappy.jobs_get_all().keys().len(), 0);
            assert_eq!(job.site_job.get(&scraper).unwrap().len(), 0);
        }
    }
}
