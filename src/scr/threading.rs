use crate::database;
use crate::download;
use crate::download::hash_bytes;
use crate::download::process_bytes;
use crate::globalload;
use crate::globalload::GlobalLoad;
use crate::logging;

use crate::logging::info_log;
use crate::sharedtypes;
use crate::sharedtypes::ScraperReturn;
use async_std::task;
use file_format::FileFormat;

// use log::{error, info};
use ratelimit::Ratelimiter;
use reqwest::blocking::Client;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;
//use std::sync::RwLock;

// use std::sync::Mutex;
use rusty_pool::ThreadPool;
//use std::sync::Mutex;
use crate::Mutex;
use crate::RwLock;
use std::thread;
use std::time::Duration;
use thread_control::*;

pub struct Threads {
    _workers: usize,
    worker: HashMap<usize, Worker>,
    worker_control: HashMap<usize, Flag>,
    scraper_storage: HashMap<sharedtypes::GlobalPluginScraper, usize>,
}

/// Holder for workers. Workers manage their own threads.
impl Default for Threads {
    fn default() -> Self {
        Self::new()
    }
}

impl Threads {
    pub fn new() -> Self {
        let workers = 0;
        Threads {
            _workers: workers,
            worker: HashMap::new(),
            worker_control: HashMap::new(),
            scraper_storage: HashMap::new(),
        }
    }

    /// Adds a worker to the threadvec.
    pub fn startwork(
        &mut self,
        jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
        db: Arc<RwLock<database::Main>>,
        scrapermanager: sharedtypes::GlobalPluginScraper,
        globalload: Arc<RwLock<GlobalLoad>>,
    ) {
        // Stupid prefilter because an item can be either a scraper or a plugin. Not sure how I
        // didn't hit this issue sooner lol
        if let Some(sharedtypes::ScraperOrPlugin::Scraper(_)) = scrapermanager.storage_type {
        } else {
            return;
        }

        if !self.scraper_storage.contains_key(&scrapermanager) {
            let (flag, control) = make_pair();
            self.scraper_storage
                .insert(scrapermanager.clone(), self._workers);
            let worker = Worker::new(
                self._workers,
                jobstorage,
                db,
                scrapermanager,
                globalload,
                control,
            );
            self.worker_control.insert(self._workers, flag);
            self.worker.insert(self._workers, worker);
            self._workers += 1;
        }
        // self._workers.push(worker);
    }

    ///
    /// Checks if we're empty
    ///
    pub fn check_empty(&self) -> bool {
        self.worker_control.is_empty()
    }

    /// Checks and clears the worker pools & long stored data
    pub fn check_threads(&mut self) {
        let mut temp = Vec::new();

        // Checks if a thread is processing data currently
        for (id, threadflag) in &self.worker_control {
            if !threadflag.alive() {
                logging::info_log(format!("Removing Worker Thread {}", id));
                temp.push(*id)
            }
        }

        // Removing the data from thread handler
        for id in temp {
            self.worker_control.remove(&id);
            self.worker.remove(&id);
            for (scraper, idscraper) in self.scraper_storage.clone() {
                if idscraper == id {
                    self.scraper_storage.remove(&scraper);
                }
            }
        }

        // Reset counter
        if self.worker.is_empty() {
            self._workers = 0;
        }
    }
}

/// Worker holder for data. Will add a scraper processor soon tm.
struct Worker {
    id: usize,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// closes the thread that the worker contains. Used in easy thread handeling Only
/// reason to do this over doing this with default drop behaviour is the logging.
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            futures::executor::block_on(async { thread.join().unwrap() });
            info_log(format!("Shutting Down Worker from Worker: {}", self.id));
        }
    }
}
///
/// Creates a relelimiter object
pub fn create_ratelimiter(
    input: (u64, Duration),
    worker_id: &usize,
    job_id: &usize,
) -> Arc<Mutex<Ratelimiter>> {
    Arc::new(Mutex::new(download::ratelimiter_create(
        worker_id, job_id, input.0, input.1,
    )))
}

impl Worker {
    fn new(
        id: usize,
        jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
        dba: Arc<RwLock<database::Main>>,
        scraper: sharedtypes::GlobalPluginScraper,
        globalload: Arc<RwLock<GlobalLoad>>,
        threadflagcontrol: Control,
    ) -> Worker {
        // info_log(&format!( "Creating Worker for id: {} Scraper Name: {} With a jobs
        // length of: {}", &id, &scraper._name, &jobstorage..len() ));
        let db = dba.clone();
        let jobstorage = jobstorage.clone();
        let globalload = globalload.clone();
        let ratelimiter_main;
        if let Some(sharedtypes::ScraperOrPlugin::Scraper(ref scraper_info)) = scraper.storage_type
        {
            ratelimiter_main = create_ratelimiter(scraper_info.ratelimit, &id, &0);
        } else {
            return Worker { id, thread: None };
        }
        let thread = thread::spawn(move || {
            let ratelimiter_obj = ratelimiter_main.clone();

            let modifiers = download::get_modifiers(&scraper);

            let client_text = Arc::new(RwLock::new(download::client_create(
                modifiers.clone(),
                true,
            )));
            let client_file = Arc::new(RwLock::new(download::client_create(modifiers, false)));
            let mut should_remove_original_job;
            'bigloop: loop {
                let jobsstorage;
                {
                    jobsstorage = jobstorage.read().unwrap().jobs_get(&scraper).clone();
                }

                if jobsstorage.is_empty() {
                    logging::info_log(format!(
                        "Worker {} -- Stopping loop because we have no jobs.",
                        &id
                    ));
                    break 'bigloop;
                }
                for mut job in jobsstorage {
                    let jobid = job.id.unwrap();
                    should_remove_original_job = true;
                    let currentjob = job.clone();
                    {
                        for (key, login, login_needed, _help_text, overwrite_db_entry) in
                            scraper.login_type.iter()
                        {
                            match login {
                                sharedtypes::LoginType::Api(_name, _api_key) => {
                                    todo!();
                                }
                                sharedtypes::LoginType::ApiNamespaced(
                                    name,
                                    api_namespace,
                                    api_body,
                                ) => {
                                    let unwrappydb = &mut db.write().unwrap();
                                    if *overwrite_db_entry && api_namespace.is_some() {
                                        unwrappydb.setting_add(
                                            format!("API_NAMESPACED_NAMESPACE_{}_{}", key, name),
                                            None,
                                            None,
                                            api_namespace.clone(),
                                            true,
                                        );
                                    }
                                    if *overwrite_db_entry && api_body.is_some() {
                                        unwrappydb.setting_add(
                                            format!("API_NAMESPACED_BODY_{}_{}", key, name),
                                            None,
                                            None,
                                            api_body.clone(),
                                            true,
                                        );
                                    }
                                    let ns_stored =
                                        if let Some(setting_obj) = unwrappydb.settings_get_name(
                                            &format!("API_NAMESPACED_NAMESPACE_{}_{}", key, name),
                                        ) {
                                            setting_obj.param.clone()
                                        } else {
                                            None
                                        };
                                    let body_stored =
                                        if let Some(setting_obj) = unwrappydb.settings_get_name(
                                            &format!("API_NAMESPACED_BODY_{}_{}", key, name),
                                        ) {
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

                                    if ns_stored.is_some() | body_stored.is_some() {
                                        let apins = sharedtypes::LoginType::ApiNamespaced(
                                            name.to_string(),
                                            ns_stored,
                                            body_stored,
                                        );
                                        job.param.push(sharedtypes::ScraperParam::Login(apins));
                                    }
                                }
                                sharedtypes::LoginType::Cookie(_name, _cookie) => {
                                    todo!();
                                }
                                sharedtypes::LoginType::Other(_name, _other) => {
                                    todo!();
                                }
                                sharedtypes::LoginType::Login(_name, _login_info) => {
                                    todo!();
                                }
                            }
                        }
                    }

                    // Makes recursion possible
                    if let Some(recursion) = &job.jobmanager.recreation {
                        should_remove_original_job = false;
                        if let sharedtypes::DbJobRecreation::AlwaysTime(timestamp, _count) =
                            recursion
                        {
                            let mut data = job.clone();
                            data.time = crate::time_func::time_secs();
                            data.reptime = Some(*timestamp);
                            jobstorage
                                .write()
                                .unwrap()
                                .jobs_decrement_count(&data, &scraper, &id);

                            // Updates the database with the "new" object. Will have the same ID
                            // but time and reptime will be consistient to when we should run this
                            // job next
                            let unwrappydb = &mut db.write().unwrap();
                            unwrappydb.jobs_update_db(data);
                        }
                    }

                    // Legacy data holder for plugin system
                    let job_holder_legacy = sharedtypes::JobScraper {
                        site: job.site.clone(),
                        param: job.param.clone(),
                        job_type: job.jobmanager.jobtype,
                    };

                    // Legacy data holder obj for plguins
                    let mut scraper_data_holder = sharedtypes::ScraperData {
                        job: job_holder_legacy.clone(),
                        system_data: job.system_data.clone(),
                        user_data: job.user_data.clone(),
                    };

                    // Loads anything passed from the scraper at compile time into the user_data
                    // field
                    if let Some(ref stored_info) = scraper.stored_info {
                        match stored_info {
                            sharedtypes::StoredInfo::Storage(storage) => {
                                for (key, val) in storage.iter() {
                                    scraper_data_holder
                                        .user_data
                                        .insert(key.to_string(), val.to_string());
                                }
                            }
                        }
                    }
                    let urlload = match job.jobmanager.jobtype {
                        sharedtypes::DbJobType::Params => {
                            let mut out = Vec::new();

                            match globalload::url_dump(
                                &job.param,
                                &scraper_data_holder,
                                globalload.clone(),
                                &scraper,
                            ) {
                                Ok(temp) => {
                                    for (url, scraperdata) in temp {
                                        out.push((
                                            sharedtypes::ScraperParam::Url(url),
                                            scraperdata,
                                        ));
                                    }
                                }
                                Err(err) => {
                                    logging::error_log(&format!(
                                        "
Worker: {id} JobId: {} -- While trying to parse parameters we got this error: {:?}
                                        ",
                                        jobid, err
                                    ));
                                    logging::error_log(&format!(
                                        "Worker: {} JobId: {} -- Telling system to keep job due to previous error.",
                                        id, jobid
                                    ));
                                    jobstorage.write().unwrap().jobs_remove_job(&scraper, &job);
                                    should_remove_original_job = false;
                                }
                            }
                            out
                        }
                        sharedtypes::DbJobType::Plugin => {
                            continue;
                        }
                        sharedtypes::DbJobType::NoScrape => {
                            let mut out = Vec::new();
                            for item in job.param.iter() {
                                out.push((item.clone(), scraper_data_holder.clone()));
                            }
                            out
                        }
                        sharedtypes::DbJobType::FileUrl => Vec::new(),
                        // sharedtypes::DbJobType::FileUrl => { let parpms: Vec<(String, ScraperData)> = (
                        // job.param .clone() .unwrap() .split_whitespace() .map(str::to_string)
                        // .collect(), scraper_data_holder, ); parpms }
                        sharedtypes::DbJobType::Scraper => {
                            let mut out = Vec::new();
                            for item in job.param.iter() {
                                out.push((item.clone(), scraper_data_holder.clone()));
                            }
                            out
                        }
                    };

                    'urlloop: for (scraperparam, scraperdata) in urlload {
                        'errloop: loop {
                            let resp;
                            let out_st;
                            if let sharedtypes::ScraperParam::Url(ref url_string) = scraperparam {
                                if !scraper.should_handle_text_scraping {
                                    resp = task::block_on(download::dltext_new(
                                        url_string,
                                        client_text.clone(),
                                        &ratelimiter_obj,
                                        &id,
                                    ));
                                    let st = match resp {
                                        Ok((respstring, resp_url)) => globalload::parser_call(
                                            &respstring,
                                            &resp_url,
                                            &scraperdata,
                                            globalload.clone(),
                                            &scraper,
                                        ),
                                        Err(_) => {
                                            logging::error_log(&format!(
                                                "Worker: {} -- While processing job {:?} was unable to download text.",
                                                &id, &job
                                            ));
                                            break 'errloop;
                                        }
                                    };
                                    out_st = match st {
                                        Ok(objectscraper) => objectscraper,
                                        Err(ScraperReturn::Nothing) => {
                                            // job_params.lock().unwrap().remove(&scraper_data);
                                            logging::info_log(format!(
                                                "Worker: {id} JobId: {} -- Exiting loop due to nothing.",
                                                jobid
                                            ));
                                            break 'urlloop;
                                        }
                                        Err(ScraperReturn::EMCStop(emc)) => {
                                            panic!("EMC STOP DUE TO: {}", emc);
                                        }
                                        Err(ScraperReturn::Stop(stop)) => {
                                            // let temp = scraper_data.clone().job;
                                            // job_params.lock().unwrap().remove(&scraper_data);
                                            logging::error_log(&format!(
                                                "Stopping job: {:?}",
                                                stop
                                            ));
                                            continue;
                                        }
                                        Err(ScraperReturn::Timeout(time)) => {
                                            let time_dur = Duration::from_secs(time);
                                            thread::sleep(time_dur);
                                            continue;
                                        }
                                    };
                                } else {
                                    loop {
                                        match globalload::text_scraping(
                                            url_string,
                                            &scraperdata.job.param,
                                            &scraperdata,
                                            globalload.clone(),
                                            &scraper,
                                        ) {
                                            Ok(scraperobj) => {
                                                out_st = scraperobj;
                                                break;
                                            }
                                            Err(scraperreturn) => match scraperreturn {
                                                sharedtypes::ScraperReturn::Timeout(time) => {
                                                    thread::sleep(Duration::from_secs(time))
                                                }
                                                _ => {
                                                    logging::error_log(&format!(
                                                        "Worker: {} -- While processing job {:?} was unable to download text.",
                                                        &id, &job
                                                    ));
                                                    break 'errloop;
                                                }
                                            },
                                        }

                                        /*if let Ok(scraperobj) = globalload::text_scraping(
                                            url_string,
                                            &scraperdata.job.param,
                                            &scraperdata,
                                            globalload.clone(),
                                            &scraper,
                                        ) {
                                            out_st = scraperobj;
                                        } else {
                                            logging::error_log(&format!("Worker: {} -- While processing job {:?} was unable to download text.",&id, &job));
                                            break 'errloop;
                                        }*/
                                    }
                                }
                            } else {
                                // Finished checking everything for URLs and other stuff.
                                break 'errloop;
                            }
                            for flag in out_st.flag {
                                match flag {
                                    sharedtypes::Flags::Redo => {
                                        should_remove_original_job = false;
                                    }
                                }
                            }
                            for tag in out_st.tag.iter() {
                                parse_jobs(
                                    tag,
                                    None,
                                    jobstorage.clone(),
                                    db.clone(),
                                    &scraper,
                                    &id,
                                    &jobid,
                                    globalload.clone(),
                                );
                            }

                            // Spawns the multithreaded pool
                            let pool = ThreadPool::default();

                            // Parses files from urls
                            for mut file in out_st.file {
                                let ratelimiter_obj = ratelimiter_main.clone();
                                let globalload = globalload.clone();
                                let db = db.clone();
                                let client = client_file.clone();
                                let jobstorage = jobstorage.clone();
                                let scraper = scraper.clone();
                                pool.execute(move || {
                                    main_file_loop(
                                        &mut file,
                                        db,
                                        ratelimiter_obj,
                                        globalload,
                                        client,
                                        jobstorage,
                                        &scraper,
                                        &id,
                                        &jobid,
                                    );
                                });
                                // End of err catching loop. break 'errloop;
                            }
                            pool.join();
                            break 'errloop;
                        }
                    }
                    {
                        if should_remove_original_job {
                            jobstorage.write().unwrap().jobs_remove_dbjob(
                                &scraper,
                                &currentjob,
                                &id,
                            );
                        } else {
                            let mut unwrappy = db.write().unwrap();
                            unwrappy.transaction_flush();
                        }
                        {
                            jobstorage
                                .write()
                                .unwrap()
                                .jobs_remove_job(&scraper, &currentjob);
                        }
                    }
                }
            }
            threadflagcontrol.stop();
            jobstorage
                .write()
                .unwrap()
                .clear_previously_seen_cache(&scraper);
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

/// Parses tags and adds the tags into the db.
pub fn parse_tags(
    db: Arc<RwLock<database::Main>>,
    tag: &sharedtypes::TagObject,
    file_id: Option<usize>,
    worker_id: &usize,
    job_id: &usize,
    manager: Arc<RwLock<GlobalLoad>>,
) -> BTreeSet<sharedtypes::ScraperData> {
    let mut url_return: BTreeSet<sharedtypes::ScraperData> = BTreeSet::new();
    match &tag.tag_type {
        sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex => {
            // println!("Adding tag: {} {:?}", tag.tag, &file_id); We've recieved a normal
            // tag. Will parse.

            if tag.tag_type != sharedtypes::TagType::NormalNoRegex {
                // Runs regex mostly
                manager.read().unwrap().plugin_on_tag(tag);
            }
            let tag_id = db.write().unwrap().tag_add_tagobject(tag, true);
            match file_id {
                None => {}
                Some(id) => {
                    let mut unwrappy = db.write().unwrap();
                    unwrappy.relationship_add(id, tag_id, true);
                }
            }
            url_return
        }
        sharedtypes::TagType::ParseUrl((jobscraped, skippy)) => {
            match skippy {
                None => {
                    url_return.insert(jobscraped.clone());
                }
                Some(skip_if) => match skip_if {
                    sharedtypes::SkipIf::FileHash(sha512hash) => {
                        let unwrappy = db.read().unwrap();
                        if unwrappy.file_get_hash(sha512hash).is_none() {
                            url_return.insert(jobscraped.clone());
                        }
                    }
                    sharedtypes::SkipIf::FileNamespaceNumber((
                        unique_tag,
                        namespace_filter,
                        filter_number,
                    )) => {
                        let mut cnt = 0;
                        let unwrappy = db.read().unwrap();
                        if let Some(nidf) = &unwrappy.namespace_get(&namespace_filter.name)
                            && let Some(nid) = &unwrappy.namespace_get(&unique_tag.namespace.name)
                            && let Some(tid) = &unwrappy.tag_get_name(unique_tag.tag.clone(), *nid)
                        {
                            let fids = unwrappy.relationship_get_fileid(tid);
                            if fids.len() == 1 {
                                let fid = fids.iter().next().unwrap();
                                for tidtofilter in unwrappy.relationship_get_tagid(fid).iter() {
                                    //if unwrappy.namespace_contains_id(nidf) {
                                    if unwrappy.namespace_contains_id(nidf, tidtofilter) {
                                        cnt += 1;
                                    }
                                }
                            }
                        }
                        if cnt >= *filter_number {
                            info_log(format!(
                                "Not downloading because unique namespace is greater then limit number. {}",
                                unique_tag.tag
                            ));
                        } else {
                            info_log(
                                    "Downloading due to unique namespace not existing or number less then limit number.".to_string(),
                                );
                            url_return.insert(jobscraped.clone());
                        }
                    }
                    sharedtypes::SkipIf::FileTagRelationship(taginfo) => 'tag: {
                        let unwrappy = db.read().unwrap();
                        let nid = unwrappy.namespace_get(&taginfo.namespace.name);
                        let id = match nid {
                            None => {
                                println!("Namespace does not exist: {:?}", taginfo.namespace);
                                url_return.insert(jobscraped.clone());
                                break 'tag;
                            }
                            Some(id) => id,
                        };
                        match &unwrappy.tag_get_name(taginfo.tag.clone(), id) {
                            None => {
                                println!("WillDownload: {}", taginfo.tag);
                                url_return.insert(jobscraped.clone());
                            }
                            Some(tag_id) => {
                                let rel_hashset = unwrappy.relationship_get_fileid(tag_id);
                                if rel_hashset.is_empty() {
                                    info_log(format!(
                                        "Worker: {worker_id} JobId: {job_id} -- Will download from {} because tag name {} has no relationship.",
                                        jobscraped.job.site, taginfo.tag
                                    ));
                                    url_return.insert(jobscraped.clone());
                                } else {
                                    info_log(format!(
                                        "Worker: {worker_id} JobId: {job_id} -- Skipping because this already has a relationship. {}",
                                        taginfo.tag
                                    ));
                                }
                                break 'tag;
                            }
                        }
                    }
                },
            }

            // Returns the url that we need to parse.
            url_return
        }
        sharedtypes::TagType::Special => {
            // Do nothing will handle this later lol.
            url_return
        }
    }
}

///
/// Downloads a file into the db if needed
///
fn download_add_to_db(
    ratelimiter_obj: Arc<Mutex<Ratelimiter>>,
    source: &String,
    location: String,
    globalload: Arc<RwLock<GlobalLoad>>,
    client: Arc<RwLock<Client>>,
    db: Arc<RwLock<database::Main>>,
    file: &mut sharedtypes::FileObject,
    worker_id: &usize,
    job_id: &usize,
    scraper: &sharedtypes::GlobalPluginScraper,
) -> Option<usize> {
    // Early exit for if the file is a dead url
    {
        let unwrappydb = &mut db.read().unwrap();
        if unwrappydb.check_dead_url(source) {
            logging::info_log(format!(
                "Worker: {worker_id} JobID: {job_id} -- Skipping {} because it's a dead link.",
                source
            ));
            return None;
        }
    }

    let mut_client = &mut client.write().unwrap();

    // Download file doesn't exist. URL doesn't exist in DB Will download
    let blopt;
    {
        blopt = download::dlfile_new(
            mut_client,
            db.clone(),
            file,
            Some(globalload),
            &ratelimiter_obj,
            source,
            worker_id,
            job_id,
            Some(scraper),
        );
    }

    match blopt {
        download::FileReturnStatus::File((hash, file_ext)) => {
            let unwrappydb = &mut db.write().unwrap();

            let ext_id = unwrappydb.extension_put_string(&file_ext);

            unwrappydb.storage_put(&location);
            let storage_id = unwrappydb.storage_get_id(&location).unwrap();

            let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                hash,
                ext_id,
                storage_id,
            });
            let fileid = unwrappydb.file_add(file);
            let source_url_ns_id = unwrappydb.create_default_source_url_ns_id();
            let tagid = unwrappydb.tag_add(source, source_url_ns_id, true, None);
            unwrappydb.relationship_add(fileid, tagid, true);
            return Some(fileid);
        }
        download::FileReturnStatus::DeadUrl(dead_url) => {
            let unwrappydb = &mut db.write().unwrap();
            unwrappydb.add_dead_url(&dead_url);
        }
        _ => {}
    }

    None
}

/// Simple code to add jobs from a tag object
fn parse_jobs(
    tag: &sharedtypes::TagObject,
    fileid: Option<usize>,
    jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
    db: Arc<RwLock<database::Main>>,
    scraper: &sharedtypes::GlobalPluginScraper,

    worker_id: &usize,
    job_id: &usize,
    manager: Arc<RwLock<GlobalLoad>>,
) {
    let urls_to_scrape = parse_tags(db, tag, fileid, worker_id, job_id, manager);
    {
        let mut joblock = jobstorage.write().unwrap();
        for data in urls_to_scrape {
            // Defualt job object
            let dbjob = sharedtypes::DbJobsObj {
                id: None,
                time: 0,
                reptime: Some(0),
                priority: sharedtypes::DEFAULT_PRIORITY,
                cachetime: sharedtypes::DEFAULT_CACHETIME,
                cachechecktype: sharedtypes::DEFAULT_CACHECHECK,
                site: data.job.site,
                param: data.job.param,
                jobmanager: sharedtypes::DbJobsManager {
                    jobtype: data.job.job_type,
                    recreation: None,
                },
                isrunning: false,
                system_data: data.system_data,
                user_data: data.user_data,
            };

            joblock.jobs_add(scraper.clone(), dbjob);
        }
    }
}

/// Parses weather we should skip downloading the file
/// Returns a Some(usize) if the fileid exists
fn parse_skipif(
    file_tag: &sharedtypes::SkipIf,
    file_url_source: &String,
    db: Arc<RwLock<database::Main>>,
    worker_id: &usize,
    job_id: &usize,
) -> Option<usize> {
    match file_tag {
        sharedtypes::SkipIf::FileHash(sha512hash) => {
            let unwrappy = db.read().unwrap();
            return unwrappy.file_get_hash(sha512hash);
        }
        sharedtypes::SkipIf::FileNamespaceNumber((unique_tag, namespace_filter, filter_number)) => {
            let unwrappydb = db.read().unwrap();
            let mut cnt = 0;
            let fids;
            if let Some(nidf) = &unwrappydb.namespace_get(&namespace_filter.name)
                && let Some(nid) = unwrappydb.namespace_get(&unique_tag.namespace.name)
                && let Some(tid) = &unwrappydb.tag_get_name(unique_tag.tag.clone(), nid)
            {
                fids = unwrappydb.relationship_get_fileid(tid);
                if fids.len() == 1 {
                    let fid = fids.iter().next().unwrap();
                    for tidtofilter in unwrappydb.relationship_get_tagid(fid).iter() {
                        if unwrappydb.namespace_contains_id(nidf, tidtofilter) {
                            //if unwrappydb.namespace_contains_id(nidf, tidtofilter) {
                            cnt += 1;
                        }
                    }
                }
            } else {
                return None;
            }
            if cnt > *filter_number {
                info_log(format!(
                    "Not downloading because unique namespace is greater then limit number. {}",
                    unique_tag.tag
                ));
            } else {
                info_log(
                    "Downloading due to unique namespace not existing or number less then limit number.".to_string(),
                );
                let vec: Vec<usize> = fids.iter().cloned().collect();
                return Some(vec[0]);
            }
        }
        sharedtypes::SkipIf::FileTagRelationship(tag) => {
            let unwrappydb = db.read().unwrap();
            if let Some(nsid) = unwrappydb.namespace_get(&tag.namespace.name)
                && unwrappydb.tag_get_name(tag.tag.to_string(), nsid).is_some()
            {
                info_log(format!(
                    "Worker: {worker_id} JobId: {job_id} -- Skipping file: {} Due to skip tag {} already existing in Tags Table.",
                    file_url_source, tag.tag
                ));
                if let Some(tid) = unwrappydb.tag_get_name(tag.tag.to_string(), nsid) {
                    return unwrappydb.relationship_get_one_fileid(&tid);
                }
            }
        }
    }
    None
}

/// Main file checking loop manages the downloads
pub fn main_file_loop(
    file: &mut sharedtypes::FileObject,
    db: Arc<RwLock<database::Main>>,
    ratelimiter_obj: Arc<Mutex<Ratelimiter>>,
    globalload: Arc<RwLock<GlobalLoad>>,
    client: Arc<RwLock<Client>>,
    jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
    scraper: &sharedtypes::GlobalPluginScraper,
    worker_id: &usize,
    job_id: &usize,
) {
    let fileid;
    match file.source.clone() {
        Some(source) => match source {
            sharedtypes::FileSource::Url(source_url) => {
                // If url exists in db then don't download thread::sleep(Duration::from_secs(10));
                for file_tag in file.skip_if.iter() {
                    if let Some(file_id) =
                        parse_skipif(file_tag, &source_url, db.clone(), worker_id, job_id)
                    {
                        for tag in file.tag_list.iter() {
                            parse_tags(
                                db.clone(),
                                tag,
                                Some(file_id),
                                worker_id,
                                job_id,
                                globalload.clone(),
                            );
                        }
                        return;
                    }
                }

                // Gets the source url namespace id
                let source_url_id = {
                    let mut unwrappydb = db.write().unwrap();
                    unwrappydb.create_default_source_url_ns_id()
                };

                let location = {
                    let unwrappydb = db.read().unwrap();
                    unwrappydb.location_get()
                };

                let url_tag;
                {
                    let unwrappydb = db.read().unwrap();
                    url_tag = unwrappydb.tag_get_name(source_url.clone(), source_url_id);
                };

                // Get's the hash & file ext for the file.
                fileid = match url_tag {
                    None => {
                        match download_add_to_db(
                            ratelimiter_obj,
                            &source_url,
                            location,
                            globalload.clone(),
                            client,
                            db.clone(),
                            file,
                            worker_id,
                            job_id,
                            scraper,
                        ) {
                            None => return,
                            Some(id) => id,
                        }
                    }
                    Some(url_id) => {
                        let file_id;
                        {
                            // We've already got a valid relationship
                            let unwrappydb = &mut db.read().unwrap();
                            file_id = unwrappydb.relationship_get_one_fileid(&url_id);
                            /*if let Some(fid) = file_id {
                                unwrappydb.file_get_id(&fid).unwrap();
                            }*/
                        }

                        // fixes busted links.
                        match file_id {
                            Some(file_id) => {
                                info_log(format!(
                                    "Worker: {worker_id} JobId: {job_id} -- Skipping file: {} Due to already existing in Tags Table.",
                                    &source_url
                                ));
                                file_id
                            }
                            None => {
                                match download_add_to_db(
                                    ratelimiter_obj,
                                    &source_url,
                                    location,
                                    globalload.clone(),
                                    client,
                                    db.clone(),
                                    file,
                                    worker_id,
                                    job_id,
                                    scraper,
                                ) {
                                    None => return,
                                    Some(id) => id,
                                }
                            }
                        }
                    }
                };
            }
            sharedtypes::FileSource::Bytes(bytes) => {
                let bytes = &bytes::Bytes::from(bytes);
                let file_ext = FileFormat::from_bytes(bytes).extension().to_string();
                let sha512 = hash_bytes(bytes, &sharedtypes::HashesSupported::Sha512("".into()));

                dbg!(&sha512.0);

                process_bytes(
                    bytes,
                    Some(globalload.clone()),
                    &sha512.0,
                    &file_ext,
                    db.clone(),
                    file,
                    None,
                );

                let unwrappy = db.read().unwrap();
                fileid = unwrappy.file_get_hash(&sha512.0).unwrap();
            }
        },
        None => return,
    }

    for tag in file.tag_list.iter() {
        parse_jobs(
            tag,
            Some(fileid),
            jobstorage.clone(),
            db.clone(),
            scraper,
            worker_id,
            job_id,
            globalload.clone(),
        );
    }
}
