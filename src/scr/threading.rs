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

use std::ops::ControlFlow;
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
        db: database::Main,
        scrapermanager: sharedtypes::GlobalPluginScraper,
        globalload: GlobalLoad,
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
        database: database::Main,
        scraper: sharedtypes::GlobalPluginScraper,
        globalload: GlobalLoad,
        threadflagcontrol: Control,
    ) -> Worker {
        // info_log(&format!( "Creating Worker for id: {} Scraper Name: {} With a jobs
        // length of: {}", &id, &scraper._name, &jobstorage..len() ));
        let jobstorage = jobstorage.clone();
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
                    jobsstorage = jobstorage.read().jobs_get_priority_order(&scraper).clone();
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
                                    {
                                        if *overwrite_db_entry && api_namespace.is_some() {
                                            database.setting_add(
                                                format!(
                                                    "API_NAMESPACED_NAMESPACE_{}_{}",
                                                    key, name
                                                ),
                                                None,
                                                None,
                                                api_namespace.clone(),
                                                true,
                                            );
                                            database.transaction_flush();
                                        }
                                        if *overwrite_db_entry && api_body.is_some() {
                                            database.setting_add(
                                                format!("API_NAMESPACED_BODY_{}_{}", key, name),
                                                None,
                                                None,
                                                api_body.clone(),
                                                true,
                                            );
                                            database.transaction_flush();
                                        }
                                    }
                                    let ns_stored =
                                        if let Some(setting_obj) = database.settings_get_name(
                                            &format!("API_NAMESPACED_NAMESPACE_{}_{}", key, name),
                                        ) {
                                            setting_obj.param.clone()
                                        } else {
                                            None
                                        };
                                    let body_stored =
                                        if let Some(setting_obj) = database.settings_get_name(
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
                            {
                                let mut data = job.clone();
                                data.time = crate::time_func::time_secs();
                                data.reptime = Some(*timestamp);
                                jobstorage
                                    .write()
                                    .jobs_decrement_count(&data, &scraper, &id);

                                // Updates the database with the "new" object. Will have the same ID
                                // but time and reptime will be consistient to when we should run this
                                // job next
                                database.jobs_update_db(data);
                            }
                            database.transaction_flush();
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

                            match globalload.url_dump(&job.param, &scraper_data_holder, &scraper) {
                                Ok(temp) => {
                                    for (url, scraperdata) in temp {
                                        out.push((
                                            sharedtypes::ScraperParam::Url(url),
                                            scraperdata,
                                        ));
                                    }
                                }
                                Err(err) => {
                                    logging::error_log(format!(
                                        "
Worker: {id} JobId: {} -- While trying to parse parameters we got this error: {:?}
                                        ",
                                        jobid, err
                                    ));
                                    logging::error_log(format!(
                                        "Worker: {} JobId: {} -- Telling system to keep job due to previous error.",
                                        id, jobid
                                    ));
                                    jobstorage.write().jobs_remove_job(&scraper, &job);
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
                            let out_sts;
                            if let sharedtypes::ScraperParam::Url(ref url_string) = scraperparam {
                                if !scraper.should_handle_text_scraping {
                                    resp = task::block_on(download::dltext_new(
                                        url_string,
                                        client_text.clone(),
                                        &ratelimiter_obj,
                                        &id,
                                    ));
                                    let st = match resp {
                                        Ok((respstring, resp_url)) => globalload.parser_call(
                                            &respstring,
                                            &resp_url,
                                            &scraperdata,
                                            &scraper,
                                        ),
                                        Err(err) => {
                                            logging::error_log(format!(
                                                "Worker: {} -- While processing job {:?} was unable to download text. Had err {:?}",
                                                &id, &job, err
                                            ));
                                            break 'urlloop;
                                        }
                                    };
                                    out_sts = st;
                                } else {
                                    out_sts = globalload.text_scraping(
                                        url_string,
                                        &scraperdata.job.param,
                                        &scraperdata,
                                        &scraper,
                                    )
                                }
                            } else {
                                // Finished checking everything for URLs and other stuff.
                                break 'errloop;
                            }
                            // If we get nothing in then treat it as if we have nothing and stop job
                            if out_sts.is_empty() {
                                break 'urlloop;
                            }
                            for out_st in out_sts.iter() {
                                match out_st {
                                    // Valid data from the scraper
                                    sharedtypes::ScraperReturn::Data(out_st) => {
                                        for flag in out_st.flag.iter() {
                                            match flag {
                                                sharedtypes::Flags::Redo => {
                                                    should_remove_original_job = false;
                                                }
                                            }
                                        }

                                        // Extracts any jobs from the tags field
                                        for tag in out_st.tag.iter() {
                                            parse_jobs(
                                                tag,
                                                None,
                                                jobstorage.clone(),
                                                database.clone(),
                                                &scraper,
                                                &id,
                                                &jobid,
                                                globalload.clone(),
                                            );
                                        }
                                        // Spawns the multithreaded pool
                                        let pool = ThreadPool::default();

                                        // Parses files from urls
                                        for mut file in out_st.file.clone() {
                                            let ratelimiter_obj = ratelimiter_main.clone();
                                            let globalload = globalload.clone();
                                            let db = database.clone();
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
                                        }
                                        pool.join();
                                    }
                                    // Nothing was returned so we stop job searching
                                    sharedtypes::ScraperReturn::Nothing => {
                                        logging::info_log(format!(
                                            "Worker: {id} JobId: {} -- Exiting loop due to nothing.",
                                            jobid
                                        ));

                                        // If the last item is nothing then stop
                                        if *out_st == out_sts[out_sts.len() - 1] {
                                            break 'urlloop;
                                        }

                                        break 'errloop;
                                    }
                                    // Emergency stop should never use as it halts the program
                                    sharedtypes::ScraperReturn::EMCStop(emc) => {
                                        panic!("EMC STOP DUE TO: {}", emc);
                                    }
                                    // Stops the job same thing as nothing but gives a reason
                                    sharedtypes::ScraperReturn::Stop(stop) => {
                                        logging::error_log(format!("Stopping job: {:?}", stop));
                                        break 'urlloop;
                                    }
                                    // Waits a specified time before retrying
                                    sharedtypes::ScraperReturn::Timeout(time) => {
                                        let time_dur = Duration::from_secs(*time);
                                        thread::sleep(time_dur);
                                        continue;
                                    }
                                }
                                // If the last item is data then process the next item
                                if *out_st == out_sts[out_sts.len() - 1] {
                                    break 'errloop;
                                }
                            }
                        }
                    }
                    {
                        if should_remove_original_job {
                            jobstorage
                                .write()
                                .jobs_remove_dbjob(&scraper, &currentjob, &id);
                        }
                        {
                            jobstorage.write().jobs_remove_job(&scraper, &currentjob);
                        }
                    }
                }
            }
            threadflagcontrol.stop();
            jobstorage.write().clear_previously_seen_cache(&scraper);
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

///
/// Better handling of skipping
///
enum SkipResult {
    // Skip because of a dead url or someting
    SkipNoFile,
    // We've already got the file, with id x
    SkipExistingFile(usize),
    // Download that sucket
    Download,
}

/// Parses tags and adds the tags into the database.
pub fn parse_tags(
    database: database::Main,
    tag: &sharedtypes::TagObject,
    file_id: Option<usize>,
    worker_id: &usize,
    job_id: &usize,
    manager: GlobalLoad,
) -> BTreeSet<sharedtypes::ScraperData> {
    let mut url_return: BTreeSet<sharedtypes::ScraperData> = BTreeSet::new();
    match &tag.tag_type {
        sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex => {
            if tag.tag_type != sharedtypes::TagType::NormalNoRegex {
                // Runs regex mostly
                manager.plugin_on_tag(tag);
            }
            let tag_id = database.tag_add_tagobject(tag, true);
            match file_id {
                None => {}
                Some(id) => {
                    database.relationship_add(id, tag_id, true);
                }
            }
            /*if let Some(fid) = file_id {
                database
                    .unwrap()
                    .relationship_tag_add(fid, vec![tag.clone()]);
            }*/

            url_return
        }
        sharedtypes::TagType::ParseUrl((jobscraped, skippy)) => {
            match skippy {
                None => {
                    url_return.insert(jobscraped.clone());
                }
                Some(skip_if) => match skip_if {
                    sharedtypes::SkipIf::FileHash(sha512hash) => {
                        if database.file_get_hash(sha512hash).is_none() {
                            url_return.insert(jobscraped.clone());
                        }
                    }
                    sharedtypes::SkipIf::FileNamespaceNumber((
                        unique_tag,
                        namespace_filter,
                        filter_number,
                    )) => {
                        let mut cnt = 0;
                        if let Some(nidf) = &database.namespace_get(&namespace_filter.name)
                            && let Some(nid) = &database.namespace_get(&unique_tag.namespace.name)
                            && let Some(tid) = &database.tag_get_name(unique_tag.tag.clone(), *nid)
                        {
                            let fids = database.relationship_get_fileid(tid);
                            if fids.len() == 1 {
                                let fid = fids.iter().next().unwrap();
                                for tidtofilter in database.relationship_get_tagid(fid).iter() {
                                    //if database.namespace_contains_id(nidf) {
                                    if database.namespace_contains_id(nidf, tidtofilter) {
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
                        let nid = database.namespace_get(&taginfo.namespace.name);
                        let id = match nid {
                            None => {
                                println!("Namespace does not exist: {:?}", taginfo.namespace);
                                url_return.insert(jobscraped.clone());
                                break 'tag;
                            }
                            Some(id) => id,
                        };
                        match &database.tag_get_name(taginfo.tag.clone(), id) {
                            None => {
                                println!("WillDownload: {}", taginfo.tag);
                                url_return.insert(jobscraped.clone());
                            }
                            Some(tag_id) => {
                                let rel_hashset = database.relationship_get_fileid(tag_id);
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
    globalload: GlobalLoad,
    client: Arc<RwLock<Client>>,
    database: database::Main,
    file: &mut sharedtypes::FileObject,
    worker_id: &usize,
    job_id: &usize,
    scraper: &sharedtypes::GlobalPluginScraper,
) -> Option<usize> {
    // Early exit for if the file is a dead url
    {
        if database.check_dead_url(source) {
            logging::info_log(format!(
                "Worker: {worker_id} JobID: {job_id} -- Skipping {} because it's a dead link.",
                source
            ));
            return None;
        }
    }

    let blopt;
    {
        //let mut_client = &mut client.write();

        // Download file doesn't exist. URL doesn't exist in DB Will download
        blopt = download::dlfile_new(
            client,
            database.clone(),
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
            let fileid;
            {
                let ext_id = database.extension_put_string(&file_ext);

                let storage_id = database.storage_put(&location);

                let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                    hash,
                    ext_id,
                    storage_id,
                });
                fileid = database.file_add(file);
                let source_url_ns_id = database.create_default_source_url_ns_id();
                let tagid = database.tag_add(source, source_url_ns_id, true, None);
                database.relationship_add(fileid, tagid, true);
            }
            return Some(fileid);
        }
        download::FileReturnStatus::DeadUrl(dead_url) => {
            database.add_dead_url(&dead_url);
        }
        _ => {}
    }
    database.transaction_flush();
    None
}

/// Simple code to add jobs from a tag object
fn parse_jobs(
    tag: &sharedtypes::TagObject,
    fileid: Option<usize>,
    jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
    database: database::Main,
    scraper: &sharedtypes::GlobalPluginScraper,

    worker_id: &usize,
    job_id: &usize,
    manager: GlobalLoad,
) {
    let urls_to_scrape = parse_tags(database.clone(), tag, fileid, worker_id, job_id, manager);

    let should_flush = !urls_to_scrape.is_empty();

    {
        let mut joblock = jobstorage.write();
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

    // Only flush if we got jobs in from this. Should only do this once
    if should_flush {
        database.transaction_flush();
    }
}

/// Parses weather we should skip downloading the file
/// Returns a Some(usize) if the fileid exists
fn parse_skipif(
    file_tag: &sharedtypes::SkipIf,
    file_url_source: &String,
    database: database::Main,
    worker_id: &usize,
    job_id: &usize,
) -> Option<usize> {
    match file_tag {
        sharedtypes::SkipIf::FileHash(sha512hash) => {
            return database.file_get_hash(sha512hash);
        }
        sharedtypes::SkipIf::FileNamespaceNumber((unique_tag, namespace_filter, filter_number)) => {
            let mut cnt = 0;
            let fids;
            if let Some(nidf) = &database.namespace_get(&namespace_filter.name)
                && let Some(nid) = database.namespace_get(&unique_tag.namespace.name)
                && let Some(tid) = &database.tag_get_name(unique_tag.tag.clone(), nid)
            {
                fids = database.relationship_get_fileid(tid);
                if fids.len() == 1 {
                    let fid = fids.iter().next().unwrap();
                    for tidtofilter in database.relationship_get_tagid(fid).iter() {
                        if database.namespace_contains_id(nidf, tidtofilter) {
                            //if database.namespace_contains_id(nidf, tidtofilter) {
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
            if let Some(nsid) = database.namespace_get(&tag.namespace.name)
                && database.tag_get_name(tag.tag.to_string(), nsid).is_some()
            {
                info_log(format!(
                    "Worker: {worker_id} JobId: {job_id} -- Skipping file: {} Due to skip tag {} already existing in Tags Table.",
                    file_url_source, tag.tag
                ));
                if let Some(tid) = database.tag_get_name(tag.tag.to_string(), nsid) {
                    return database.relationship_get_one_fileid(&tid);
                }
            }
        }
    }
    None
}

/// Main file checking loop manages the downloads
pub fn main_file_loop(
    file: &mut sharedtypes::FileObject,
    database: database::Main,
    ratelimiter_obj: Arc<Mutex<Ratelimiter>>,
    globalload: GlobalLoad,
    client: Arc<RwLock<Client>>,
    jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
    scraper: &sharedtypes::GlobalPluginScraper,
    worker_id: &usize,
    job_id: &usize,
) {
    let fileid;

    let source_url_id = 0;

    match file.source.clone() {
        Some(source) => match source {
            sharedtypes::FileSource::Url(source_url) => {
                // If url exists in db then don't download thread::sleep(Duration::from_secs(10));
                for file_tag in file.skip_if.iter() {
                    if let Some(file_id) =
                        parse_skipif(file_tag, &source_url, database.clone(), worker_id, job_id)
                    {
                        for tag in file.tag_list.iter() {
                            parse_tags(
                                database.clone(),
                                tag,
                                Some(file_id),
                                worker_id,
                                job_id,
                                globalload.clone(),
                            );
                        }
                        database.transaction_flush();
                        return;
                    }
                }

                let location = { database.location_get() };

                let url_tag;
                {
                    url_tag = database.tag_get_name(source_url.clone(), source_url_id);
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
                            database.clone(),
                            file,
                            worker_id,
                            job_id,
                            scraper,
                        ) {
                            None => return,
                            Some(out) => out,
                        }
                    }
                    Some(url_id) => {
                        let file_id;
                        {
                            // We've already got a valid relationship
                            file_id = database.relationship_get_one_fileid(&url_id);
                            /*if let Some(fid) = file_id {
                                database.file_get_id(&fid).unwrap();
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
                                    database.clone(),
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

                /* dbg!(&fileid);
                let mut conn = {
                    let db = database;
                    database.get_database_connection()
                };
                let tn = conn.transaction().unwrap();

                for tag in file.tag_list.iter() {
                    parse_tags(

                        database.clone(),
                        tag,
                        Some(fileid),
                        worker_id,
                        job_id,
                        globalload.clone(),
                        false,
                    );
                }
                database.transaction_flush();*/
            }
            sharedtypes::FileSource::Bytes(bytes) => {
                let bytes = &bytes::Bytes::from(bytes);
                let file_ext = FileFormat::from_bytes(bytes).extension().to_string();
                let sha512 = hash_bytes(bytes, &sharedtypes::HashesSupported::Sha512("".into()));
                process_bytes(
                    bytes,
                    Some(globalload.clone()),
                    &sha512.0,
                    &file_ext,
                    database.clone(),
                    file,
                    None,
                );
                fileid = database.file_get_hash(&sha512.0).unwrap();
            }
        },
        None => return,
    }

    for tag in file.tag_list.iter() {
        parse_tags(
            database.clone(),
            tag,
            Some(fileid),
            worker_id,
            job_id,
            globalload.clone(),
        );

        parse_jobs(
            tag,
            Some(fileid),
            jobstorage.clone(),
            database.clone(),
            scraper,
            worker_id,
            job_id,
            globalload.clone(),
        );
    }
    database.transaction_flush();
}
