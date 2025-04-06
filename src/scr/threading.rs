use crate::database;
use crate::download;
use crate::logging;
use crate::scraper::ScraperManager;

// use crate::jobs::JobsRef;
use crate::logging::info_log;
use crate::plugins::PluginManager;
use crate::scraper;
use crate::sharedtypes;
use crate::sharedtypes::ScraperReturn;
use async_std::task;

// use log::{error, info};
use ratelimit::Ratelimiter;
use reqwest::blocking::Client;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;

// use std::sync::Mutex;
use rusty_pool::ThreadPool;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use thread_control::*;

pub struct Threads {
    _workers: usize,
    worker: HashMap<usize, Worker>,
    worker_control: HashMap<usize, Flag>,
    scraper_storage: HashMap<sharedtypes::SiteStruct, usize>,
}

/// Holder for workers. Workers manage their own threads.
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
        jobstorage: &mut Arc<Mutex<crate::jobs::Jobs>>,
        db: &mut Arc<Mutex<database::Main>>,
        scrapermanager: sharedtypes::SiteStruct,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
        arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    ) {
        if !self.scraper_storage.contains_key(&scrapermanager) {
            let (flag, control) = make_pair();
            self.scraper_storage
                .insert(scrapermanager.clone(), self._workers);
            let worker = Worker::new(
                self._workers,
                jobstorage,
                db,
                scrapermanager,
                pluginmanager,
                control,
                arc_scrapermanager,
            );
            self.worker_control.insert(self._workers, flag);
            self.worker.insert(self._workers, worker);
            self._workers += 1;
        }
        // self._workers.push(worker);
    }

    /// Checks and clears the worker pools & long stored data
    pub fn check_threads(&mut self) {
        let mut temp = Vec::new();

        // Checks if a thread is processing data currently
        for (id, threadflag) in &self.worker_control {
            if !threadflag.alive() {
                logging::info_log(&format!("Removing Worker Thread {}", id));
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
            info_log(&format!("Shutting Down Worker from Worker: {}", self.id));
            println!("Shutting Down Worker from Worker: {}", self.id);
        }
    }
}
///
/// Creates a relelimiter object
pub fn create_ratelimiter(input: (u64, Duration)) -> Arc<Mutex<Ratelimiter>> {
    Arc::new(Mutex::new(download::ratelimiter_create(input.0, input.1)))
}

impl Worker {
    fn new(
        id: usize,
        jobstorage: &mut Arc<Mutex<crate::jobs::Jobs>>,
        dba: &mut Arc<Mutex<database::Main>>,
        scraper: sharedtypes::SiteStruct,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
        threadflagcontrol: Control,
        arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    ) -> Worker {
        // info_log(&format!( "Creating Worker for id: {} Scraper Name: {} With a jobs
        // length of: {}", &id, &scraper._name, &jobstorage..len() ));
        let mut db = dba.clone();
        let mut jobstorage = jobstorage.clone();
        let manageeplugin = pluginmanager.clone();
        let ratelimiter_main = create_ratelimiter(scraper.ratelimit);
        let thread = thread::spawn(move || {
            let ratelimiter_obj = ratelimiter_main.clone();

            let mut client = download::client_create();
            let mut should_remove_original_job = true;
            'bigloop: loop {
                let jobsstorage;
                {
                    let temp = jobstorage.lock().unwrap();
                    jobsstorage = temp.jobs_get(&scraper).clone();
                }
                for job in jobsstorage {
                    should_remove_original_job = true;
                    let currentjob = job.clone();
                    let mut par_vec: Vec<sharedtypes::ScraperParam> = Vec::new();
                    {
                        let parpms: Vec<String> = job
                            .param
                            .as_ref()
                            .unwrap()
                            .split_whitespace()
                            .map(str::to_string)
                            .collect();
                        for par in parpms {
                            let temp = sharedtypes::ScraperParam::Normal(par);
                            par_vec.push(temp)
                        }

                        let unwrappydb = &mut db.lock().unwrap();
                        for (key, login, login_needed, help_text, overwrite_db_entry) in
                            scraper.login_type.iter()
                        {
                            match login {
                                sharedtypes::LoginType::Api(name, api_key) => {
                                    todo!();
                                }
                                sharedtypes::LoginType::ApiNamespaced(
                                    name,
                                    api_namespace,
                                    api_body,
                                ) => {
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
                                    if ns_stored.is_none() | body_stored.is_none() {
                                        todo!();
                                    }

                                    // Special actions need to be taken if we are logging into
                                    // something that requires a login
                                    if *login_needed == sharedtypes::LoginNeed::Required {
                                        break;
                                    }

                                    dbg!(&ns_stored, &body_stored);
                                    if ns_stored.is_some() | body_stored.is_some() {
                                        let apins = sharedtypes::LoginType::ApiNamespaced(
                                            name.to_string(),
                                            ns_stored,
                                            body_stored,
                                        );
                                        par_vec.push(sharedtypes::ScraperParam::Login(apins));
                                    }
                                }
                                sharedtypes::LoginType::Cookie(name, cookie) => {
                                    todo!();
                                }
                                sharedtypes::LoginType::Other(name, other) => {
                                    todo!();
                                }
                                sharedtypes::LoginType::Login(name, login_info) => {
                                    todo!();
                                }
                            }
                        }
                        dbg!(&scraper);
                    }

                    // Makes recursion possible
                    if let Some(recursion) = &job.jobmanager.recreation {
                        if let sharedtypes::DbJobRecreation::AlwaysTime(timestamp, count) =
                            recursion
                        {
                            let mut temp = jobstorage.lock().unwrap();
                            let mut data = job.clone();
                            data.time = Some(crate::time_func::time_secs());
                            data.reptime = Some(*timestamp);
                            temp.jobs_decrement_count(&data, &scraper);
                            should_remove_original_job = false;
                        }
                    }

                    // Legacy data holder for plugin system
                    let job_holder_legacy = sharedtypes::JobScraper {
                        site: job.site.clone(),
                        param: par_vec.clone(),
                        original_param: job.param.clone().unwrap(),
                        job_type: job.jobmanager.jobtype,
                    };

                    // Legacy data holder obj for plguins
                    let mut scraper_data_holder = sharedtypes::ScraperData {
                        job: job_holder_legacy.clone(),
                        system_data: job.system_data,
                        user_data: job.user_data,
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
                            // job = temp.1;
                            scraper::url_dump(
                                &par_vec,
                                &scraper_data_holder,
                                arc_scrapermanager.clone(),
                                &scraper,
                            )
                        }
                        sharedtypes::DbJobType::Plugin => {
                            continue;
                        }
                        sharedtypes::DbJobType::NoScrape => {
                            vec![(job.param.clone().unwrap(), scraper_data_holder)]
                        }
                        sharedtypes::DbJobType::FileUrl => Vec::new(),
                        // sharedtypes::DbJobType::FileUrl => { let parpms: Vec<(String, ScraperData)> = (
                        // job.param .clone() .unwrap() .split_whitespace() .map(str::to_string)
                        // .collect(), scraper_data_holder, ); parpms }
                        sharedtypes::DbJobType::Scraper => {
                            vec![(job.param.clone().unwrap(), scraper_data_holder)]
                        }
                    };
                    'urlloop: for (urll, scraperdata) in urlload {
                        'errloop: loop {
                            let resp = task::block_on(download::dltext_new(
                                urll.to_string(),
                                &mut client,
                                &ratelimiter_obj,
                            ));
                            let st = match resp {
                                Ok(respstring) => scraper::parser_call(
                                    &respstring,
                                    &scraperdata,
                                    arc_scrapermanager.clone(),
                                    &scraper,
                                ),
                                Err(_) => {
                                    break 'errloop;
                                }
                            };
                            let (out_st, scraper_data_parser) = match st {
                                Ok(objectscraper) => objectscraper,
                                Err(ScraperReturn::Nothing) => {
                                    // job_params.lock().unwrap().remove(&scraper_data);
                                    dbg!("Exiting loop due to nothing.");
                                    break 'urlloop;
                                }
                                Err(ScraperReturn::EMCStop(emc)) => {
                                    panic!("EMC STOP DUE TO: {}", emc);
                                }
                                Err(ScraperReturn::Stop(stop)) => {
                                    // let temp = scraper_data.clone().job;
                                    // job_params.lock().unwrap().remove(&scraper_data);
                                    logging::error_log(&format!("Stopping job: {:?}", stop));
                                    continue;
                                }
                                Err(ScraperReturn::Timeout(time)) => {
                                    let time_dur = Duration::from_secs(time);
                                    thread::sleep(time_dur);
                                    continue;
                                }
                            };

                            // Parses tags from the tags field
                            for tag in out_st.tag.iter() {
                                parse_jobs(tag, None, &mut jobstorage, &mut db, &scraper);
                            }

                            // Spawns the multithreaded pool
                            let pool = ThreadPool::default();

                            // Parses files from urls
                            for mut file in out_st.file {
                                let ratelimiter_obj = ratelimiter_main.clone();
                                let manageeplugin = manageeplugin.clone();
                                let mut db = db.clone();
                                let client = client.clone();
                                let mut jobstorage = jobstorage.clone();
                                let scraper = scraper.clone();
                                pool.execute(move || {
                                    main_file_loop(
                                        &mut file,
                                        &mut db,
                                        ratelimiter_obj,
                                        manageeplugin,
                                        &client,
                                        &mut jobstorage,
                                        &scraper,
                                    );
                                });
                                // End of err catching loop. break 'errloop;
                            }
                            pool.join();
                            break 'errloop;
                        }
                        // { let mut joblock = jobstorage.lock().unwrap();
                        // joblock.jobs_remove_dbjob(&scraper, &currentjob);
                        //
                        // let mut db = db.lock().unwrap(); db.transaction_flush(); } let unwrappydb =
                        // &mut db.lock().unwrap(); unwrappydb.del_from_jobs_byid(&job.id);
                    }
                    {
                        if should_remove_original_job {
                            let mut joblock = jobstorage.lock().unwrap();
                            joblock.jobs_remove_dbjob(&scraper, &currentjob);
                            let mut db = db.lock().unwrap();
                            db.transaction_flush();
                        }
                    }
                    {
                        if should_remove_original_job {
                            let joblock = jobstorage.lock().unwrap();
                            if joblock.jobs_get(&scraper).is_empty() {
                                let mut db = db.lock().unwrap();
                                db.transaction_flush();
                                break 'bigloop;
                            }
                        }
                    }
                }
                if should_remove_original_job {
                    let joblock = jobstorage.lock().unwrap();
                    if joblock.jobs_get(&scraper).is_empty() {
                        threadflagcontrol.stop();
                        break 'bigloop;
                    }
                }
            }
            threadflagcontrol.stop();
            let mut joblock = jobstorage.lock().unwrap();
            joblock.clear_previously_seen_cache(&scraper);
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

/// Parses tags and adds the tags into the db.
fn parse_tags(
    db: &Arc<Mutex<database::Main>>,
    tag: &sharedtypes::TagObject,
    file_id: Option<usize>,
) -> BTreeSet<sharedtypes::ScraperData> {
    let mut url_return: BTreeSet<sharedtypes::ScraperData> = BTreeSet::new();
    let unwrappy = &mut db.lock().unwrap();

    // dbg!(&tag);
    match &tag.tag_type {
        sharedtypes::TagType::Normal => {
            // println!("Adding tag: {} {:?}", tag.tag, &file_id); We've recieved a normal
            // tag. Will parse.
            let namespace_id = unwrappy.namespace_add(
                tag.namespace.name.clone(),
                tag.namespace.description.clone(),
                true,
            );
            let tag_id = unwrappy.tag_add(&tag.tag, namespace_id, true, None);
            match &tag.relates_to {
                None => {
                    // let relate_ns_id = unwrappy.namespace_add( relate.namespace.name.clone(),
                    // relate.namespace.description, true, );
                }
                Some(relate) => {
                    let relate_ns_id = unwrappy.namespace_add(
                        relate.namespace.name.clone(),
                        relate.namespace.description.clone(),
                        true,
                    );
                    let limit_to = match &relate.limit_to {
                        None => None,
                        Some(tag) => {
                            let namespace_id = unwrappy.namespace_add(
                                tag.namespace.name.clone(),
                                tag.namespace.description.clone(),
                                true,
                            );
                            let tid = unwrappy.tag_add(&tag.tag, namespace_id, true, None);
                            Some(tid)
                        }
                    };
                    let relate_tag_id = unwrappy.tag_add(&relate.tag, relate_ns_id, true, None);
                    unwrappy.parents_add(tag_id, relate_tag_id, limit_to, true);
                }
            }
            match file_id {
                None => {}
                Some(id) => {
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
                Some(skip_if) => {
                    match skip_if {
                        sharedtypes::SkipIf::FileNamespaceNumber((
                            unique_tag,
                            namespace_filter,
                            filter_number,
                        )) => {
                            let mut cnt = 0;
                            if let Some(nidf) = unwrappy.namespace_get(&namespace_filter.name) {
                                if let Some(nid) =
                                    unwrappy.namespace_get(&unique_tag.namespace.name)
                                {
                                    if let Some(tid) =
                                        unwrappy.tag_get_name(unique_tag.tag.clone(), *nid)
                                    {
                                        if let Some(fids) = unwrappy.relationship_get_fileid(tid) {
                                            if fids.len() == 1 {
                                                let fid = fids.iter().next().unwrap();
                                                for tidtofilter in unwrappy
                                                    .relationship_get_tagid(fid)
                                                    .unwrap()
                                                    .iter()
                                                {
                                                    if unwrappy
                                                        .namespace_contains_id(nidf, tidtofilter)
                                                    {
                                                        cnt += 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if cnt >= *filter_number {
                                dbg!(&cnt, &filter_number);
                                info_log(
                                    &format!(
                                        "Not downloading because unique namespace is greater then limit number. {}",
                                        unique_tag.tag
                                    ),
                                );
                            } else {
                                info_log(
                                    &"Downloading due to unique namespace not existing or number less then limit number.".to_string(),
                                );
                                url_return.insert(jobscraped.clone());
                            }
                        }
                        sharedtypes::SkipIf::FileTagRelationship(taginfo) => 'tag: {
                            let nid = unwrappy.namespace_get(&taginfo.namespace.name);
                            let id = match nid {
                                None => {
                                    println!("Namespace does not exist: {:?}", taginfo.namespace);
                                    url_return.insert(jobscraped.clone());
                                    break 'tag;
                                }
                                Some(id) => id,
                            };
                            match unwrappy.tag_get_name(taginfo.tag.clone(), *id) {
                                None => {
                                    println!("WillDownload: {}", taginfo.tag);
                                    url_return.insert(jobscraped.clone());
                                }
                                Some(tag_id) => {
                                    let rel_hashset = unwrappy.relationship_get_fileid(tag_id);
                                    match rel_hashset {
                                        None => {
                                            println!(
                                                "Downloading: {} because no relationship",
                                                taginfo.tag
                                            );
                                            println!("Will download from: {}", taginfo.tag);
                                            url_return.insert(jobscraped.clone());
                                        }
                                        Some(_) => {
                                            info_log(
                                                &format!(
                                                    "Skipping because this already has a relationship. {}",
                                                    taginfo.tag
                                                ),
                                            );
                                            // println!("Will download from: {}", taginfo.tag); url_return.insert(jobscraped);
                                        }
                                    }
                                    println!("Ignoring: {}", taginfo.tag);
                                    break 'tag;
                                }
                            }
                        }
                    }
                }
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

fn download_add_to_db(
    ratelimiter_obj: Arc<Mutex<Ratelimiter>>,
    source: &String,
    location: String,
    manageeplugin: Arc<Mutex<PluginManager>>,
    client: &Client,
    db: Arc<Mutex<database::Main>>,
    file: &mut sharedtypes::FileObject,
) -> Option<usize> {
    // Download file doesn't exist. URL doesn't exist in DB Will download
    let blopt;
    {
        blopt = download::dlfile_new(
            client,
            db.clone(),
            file,
            &location,
            Some(manageeplugin),
            &ratelimiter_obj,
            source,
        );
    }
    match blopt {
        None => None,
        Some((hash, file_ext)) => {
            let unwrappydb = &mut db.lock().unwrap();

            let ext_id = unwrappydb.extension_put_string(&file_ext);

            unwrappydb.storage_put(&location);
            let storage_id = unwrappydb.storage_get_id(&location).unwrap();

            let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                hash,
                ext_id,
                storage_id,
            });
            let fileid = unwrappydb.file_add(file, true);
            let source_url_ns_id = unwrappydb.create_default_source_url_ns_id();
            let tagid = unwrappydb.tag_add(source, source_url_ns_id, true, None);
            unwrappydb.relationship_add(fileid, tagid, true);
            Some(fileid)
        }
    }
}

/// Simple code to add jobs from a tag object
fn parse_jobs(
    tag: &sharedtypes::TagObject,
    fileid: Option<usize>,
    jobstorage: &mut Arc<Mutex<crate::jobs::Jobs>>,
    db: &mut Arc<Mutex<database::Main>>,
    scraper: &sharedtypes::SiteStruct,
) {
    let urls_to_scrape = parse_tags(db, tag, fileid);
    {
        let mut joblock = jobstorage.lock().unwrap();
        for data in urls_to_scrape {
            // Defualt job object

            let jobid;
            {
                let mut db = db.lock().unwrap();
                jobid = db.jobs_add(
                    None,
                    0,
                    0,
                    data.job.site.clone(),
                    data.job.original_param.clone(),
                    true,
                    sharedtypes::CommitType::StopOnNothing,
                    &data.job.job_type,
                    data.system_data.clone(),
                    data.user_data.clone(),
                    sharedtypes::DbJobsManager {
                        jobtype: data.job.job_type,
                        recreation: None,
                    },
                );
            }
            let dbjob = sharedtypes::DbJobsObj {
                id: jobid,
                time: Some(0),
                reptime: Some(0),
                site: data.job.site,
                param: Some(data.job.original_param),
                jobmanager: sharedtypes::DbJobsManager {
                    jobtype: data.job.job_type,
                    recreation: None,
                },
                committype: Some(sharedtypes::CommitType::StopOnNothing),
                isrunning: false,
                system_data: data.system_data,
                user_data: data.user_data,
            };

            joblock.jobs_add(scraper.clone(), dbjob);
        }
    }
}

/// Parses weather we should skip downloading the file
fn parse_skipif(
    file_tag: &sharedtypes::SkipIf,
    file_url_source: &String,
    db: &mut Arc<Mutex<database::Main>>,
) -> bool {
    match file_tag {
        sharedtypes::SkipIf::FileNamespaceNumber((unique_tag, namespace_filter, filter_number)) => {
            let unwrappydb = db.lock().unwrap();
            let mut cnt = 0;
            if let Some(nidf) = unwrappydb.namespace_get(&namespace_filter.name) {
                if let Some(nid) = unwrappydb.namespace_get(&unique_tag.namespace.name) {
                    if let Some(tid) = unwrappydb.tag_get_name(unique_tag.tag.clone(), *nid) {
                        if let Some(fids) = unwrappydb.relationship_get_fileid(tid) {
                            if fids.len() == 1 {
                                let fid = fids.iter().next().unwrap();
                                for tidtofilter in
                                    unwrappydb.relationship_get_tagid(fid).unwrap().iter()
                                {
                                    if unwrappydb.namespace_contains_id(nidf, tidtofilter) {
                                        cnt += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if cnt > *filter_number {
                info_log(&format!(
                    "Not downloading because unique namespace is greater then limit number. {}",
                    unique_tag.tag
                ));
                return true;
            } else {
                info_log(
                    &"Downloading due to unique namespace not existing or number less then limit number.".to_string(),
                );
            }
        }
        sharedtypes::SkipIf::FileTagRelationship(tag) => {
            let unwrappydb = db.lock().unwrap();
            if let Some(nsid) = unwrappydb.namespace_get(&tag.namespace.name) {
                if unwrappydb
                    .tag_get_name(tag.tag.to_string(), *nsid)
                    .is_some()
                {
                    info_log(&format!(
                        "Skipping file: {} Due to skip tag {} already existing in Tags Table.",
                        file_url_source, tag.tag
                    ));
                    return true;
                }
            }
        }
    }
    false
}

/// Main file checking loop manages the downloads
pub fn main_file_loop(
    file: &mut sharedtypes::FileObject,
    db: &mut Arc<Mutex<database::Main>>,
    ratelimiter_obj: Arc<Mutex<Ratelimiter>>,
    manageeplugin: Arc<Mutex<PluginManager>>,
    client: &Client,
    jobstorage: &mut Arc<Mutex<crate::jobs::Jobs>>,
    scraper: &sharedtypes::SiteStruct,
) {
    if let Some(source) = &file.source_url.clone() {
        // If url exists in db then don't download thread::sleep(Duration::from_secs(10));
        for file_tag in file.skip_if.iter() {
            if parse_skipif(file_tag, source, db) {
                return;
            }
        }

        // Gets the source url namespace id
        let source_url_id = {
            let unwrappydb = &mut db.lock().unwrap();
            unwrappydb.create_default_source_url_ns_id()
        };

        let location = {
            let unwrappydb = &mut db.lock().unwrap();
            unwrappydb.location_get()
        };

        let url_tag;
        {
            let unwrappydb = db.lock().unwrap();
            url_tag = unwrappydb
                .tag_get_name(source.clone(), source_url_id)
                .cloned();
        };

        // Get's the hash & file ext for the file.
        let fileid = match url_tag {
            None => {
                match download_add_to_db(
                    ratelimiter_obj,
                    source,
                    location,
                    manageeplugin,
                    client,
                    db.clone(),
                    file,
                ) {
                    None => return,
                    Some(id) => id,
                }
            }
            Some(url_id) => {
                let file_id;
                {
                    // We've already got a valid relationship
                    let unwrappydb = &mut db.lock().unwrap();
                    file_id = unwrappydb.relationship_get_one_fileid(&url_id).copied();
                    if let Some(fid) = file_id {
                        unwrappydb.file_get_id(&fid).unwrap();
                    }
                }

                // fixes busted links.
                match file_id {
                    Some(file_id) => {
                        info_log(&format!(
                            "Skipping file: {} Due to already existing in Tags Table.",
                            &source
                        ));
                        file_id
                    }
                    None => {
                        match download_add_to_db(
                            ratelimiter_obj,
                            source,
                            location,
                            manageeplugin,
                            client,
                            db.clone(),
                            file,
                        ) {
                            None => return,
                            Some(id) => id,
                        }
                    }
                }
            }
        };

        // We've got valid fileid for reference.
        for tag in file.tag_list.iter() {
            parse_jobs(tag, Some(fileid), jobstorage, db, scraper);
        }
    }
}
