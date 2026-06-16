use crate::Main;
use crate::RwLock;
use crate::download;
use crate::download::hash_bytes;
use crate::download::process_bytes;
use crate::globalload::GlobalLoad;
use crate::helpers::memory_manage;
use crate::logging;
use crate::logging::info_log;
use sharedtypes;

use crate::ui::ui::*;
use async_std::task;
use file_format::FileFormat;
use ratelimit::Ratelimiter;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use reqwest::blocking::Client;
use rusty_pool::ThreadPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use thread_control::*;

pub struct Threads {
    _workers: u64,
    worker: HashMap<u64, Worker>,
    worker_control: HashMap<u64, Flag>,
    scraper_storage: HashMap<sharedtypes::GlobalPluginScraper, u64>,
    uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIScraper>>,
}

impl Threads {
    pub fn new(uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIScraper>>) -> Self {
        let workers = 0;
        Threads {
            _workers: workers,
            worker: HashMap::new(),
            worker_control: HashMap::new(),
            scraper_storage: HashMap::new(),
            uisender,
        }
    }

    /// Adds a worker to the threadvec.
    pub fn startwork(
        &mut self,
        jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
        db: Main,
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
                self.uisender.clone(),
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
    id: u64,
    thread: Option<std::thread::JoinHandle<()>>,
    scraper: sharedtypes::GlobalPluginScraper,
    uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIScraper>>,
}

/// closes the thread that the worker contains. Used in easy thread handeling Only
/// reason to do this over doing this with default drop behaviour is the logging.
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            futures::executor::block_on(async { thread.join().unwrap() });
            info_log(format!("Shutting Down Worker from Worker: {}", self.id));

            // Sets up a default UI for the old scraper
            self.uisender.send(UIScraper {
                worker: self.id,
                name: self.scraper.name.clone(),
                status: ScraperStatus::Completed,
                files: vec![],
            });

            memory_manage();
        }
    }
}
///
/// Creates a relelimiter object
pub fn create_ratelimiter(
    input: (u64, Duration),
    worker_id: &u64,
    job_id: &u64,
) -> Arc<RwLock<Ratelimiter>> {
    Arc::new(RwLock::new(download::ratelimiter_create(
        worker_id, job_id, input.0, input.1,
    )))
}

impl Worker {
    fn new(
        id: u64,
        jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
        database: Main,
        scraper_orig: sharedtypes::GlobalPluginScraper,
        globalload: GlobalLoad,
        threadflagcontrol: Control,
        uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIScraper>>,
    ) -> Worker {
        database.load_table(&sharedtypes::LoadDBTable::All);
        // info_log(&format!( "Creating Worker for id: {} Scraper Name: {} With a jobs
        // length of: {}", &id, &scraper._name, &jobstorage..len() ));
        let jobstorage = jobstorage.clone();
        let ratelimiter_main;
        if let Some(sharedtypes::ScraperOrPlugin::Scraper(ref scraper_info)) =
            scraper_orig.storage_type
        {
            ratelimiter_main = create_ratelimiter(scraper_info.ratelimit, &id, &0);
        } else {
            return Worker {
                id,
                thread: None,
                scraper: scraper_orig,
                uisender,
            };
        }
        let scraper = scraper_orig.clone();
        let ui_scraper = uisender.clone();

        // Sets up a default UI for the new scraper
        uisender.send(UIScraper {
            worker: id,
            name: scraper_orig.name.clone(),
            status: ScraperStatus::Idle,
            files: vec![],
        });

        let uiscraper = uisender.clone();
        let thread = thread::spawn(move || {
            let ratelimiter_obj = ratelimiter_main.clone();

            let modifiers = download::get_modifiers(&scraper);

            let client_text = Arc::new(RwLock::new(download::client_create(
                modifiers.clone(),
                true,
            )));
            let client_file = Arc::new(RwLock::new(download::client_create(modifiers, false)));
            'bigloop: loop {
                let mut jobsstorage;
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

                //for mut job in jobsstorage {
                jobsstorage.par_iter_mut().for_each(| job|{
            let mut should_remove_original_job;
                    let jobid = job.id.unwrap();
                    should_remove_original_job = true;
                    let currentjob = job.clone();


                    // Makes recursion possible
                    if let Some(recursion) = &job.jobmanager.recreation {
                        should_remove_original_job = false;
                        if let sharedtypes::DbJobRecreation::AlwaysTime(timestamp, _count) =
                            recursion
                        {
                            {
                                let mut data = job.clone();
                                data.time = crate::time_func::time_secs();
                                data.reptime = *timestamp;
                                {
                                jobstorage
                                    .write()
                                    .jobs_decrement_count(&data, &scraper, &id);
                                }
                                // Updates the database with the "new" object. Will have the same ID
                                // but time and reptime will be consistient to when we should run this
                                // job next
                                database.jobs_update_db(data);
                            }
                        }
                    }
                    // Loads anything passed from the scraper at compile time into the user_data
                    // field
                    if let Some(ref stored_info) = scraper.stored_info {
                        match stored_info {
                            sharedtypes::StoredInfo::Storage(storage) => {
                                for (key, val) in storage.iter() {
                                    job
                                        .user_data
                                        .insert(key.to_string(), val.to_string());
                                }
                            }
                        }
                    }let scraper_data_return_default = sharedtypes::ScraperDataReturn {
                                job: job.clone(),
                                ..Default::default()
                            };

                    let urlload = match job.jobmanager.jobtype {
                        sharedtypes::DbJobType::Params => {
                            let mut out = Vec::new();

                            match globalload.url_dump(&job.param, &scraper_data_return_default, &scraper) {
                                Ok(temp) => {
                                    for scraper_data_return in temp {
                                        for param in scraper_data_return.job.param.iter() {
                                            if let sharedtypes::ScraperParam::Url(_) | sharedtypes::ScraperParam::UrlPost(_) = param {
                                                out.push((param.clone(), scraper_data_return.clone()));
                                            }
                                        }
                                    }
                                    /*for (url, scraperdata) in temp {
                                        out.push((
                                            sharedtypes::ScraperParam::Url(url),
                                            scraperdata,
                                        ));
                                    }*/
                                }
                                Err(err) => {
                                    logging::error_log(format!(
                                        "Worker: {id} JobId: {} -- While trying to parse parameters we got this error: {:?}",
                                        jobid, err
                                    ));
                                    logging::error_log(format!(
                                        "Worker: {} JobId: {} -- Telling system to keep job due to previous error.",
                                        id, jobid
                                    ));
                                    jobstorage.write().jobs_remove_job(&scraper, job);
                                    should_remove_original_job = false;
                                }
                            }
                            out
                        }
                        sharedtypes::DbJobType::Plugin => {
                            return;
                        }
                        sharedtypes::DbJobType::NoScrape => {
                            let mut out = Vec::new();
                            for param in job.param.iter() {if let sharedtypes::ScraperParam::Url(_) = param {
                                                out.push((param.clone(), scraper_data_return_default.clone()));
                                            }

                            }
                            out
                        }
                        sharedtypes::DbJobType::FileUrl => Vec::new(),
                        // sharedtypes::DbJobType::FileUrl => { let parpms: Vec<(String, ScraperData)> = (
                        // job.param .clone() .unwrap() .split_whitespace() .map(str::to_string)
                        // .collect(), scraper_data_holder, ); parpms }
                        sharedtypes::DbJobType::Scraper => {
                            let mut out = Vec::new();
                            for param in job.param.iter() {
                                if let sharedtypes::ScraperParam::Url(_) | sharedtypes::ScraperParam::UrlPost(_) = param {
                                                out.push((param.clone(), scraper_data_return_default.clone()));
                                            }

                            }
                            out
                        }
                    };

                    // Changes state to running with no files
uisender.send(UIScraper { worker: id, name: scraper.name.clone() ,status: ScraperStatus::Running, files: vec![] });
                    'urlloop: for (scraperparam, scraperdata) in urlload {
                        'errloop: loop {
                            let resp;
                            let out_sts;
                            if let sharedtypes::ScraperParam::Url(ref url_string) = scraperparam {
                                if !scraper.should_handle_text_scraping {
                                    resp = task::block_on(download::dltext_new(
                                        url_string,
                                        None,
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
                            } else if let sharedtypes::ScraperParam::UrlPost(ref url_post) = scraperparam {
                                resp = task::block_on(download::dltext_new(
                                    &url_post.url,
                                    Some(url_post.post_data.clone()),
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
                                        for flag in out_st.flags.iter() {
                                            match flag {
                                                &sharedtypes::ScraperFlags::Redo => {
                                                    should_remove_original_job = false;
                                                }
                                            }
                                        }

                                        // Loop thru jobs and add them if we have no skip conditions
                                        'jobloop: for scraper_data_return in out_st.jobs.iter() {

                                            for skip_condition in scraper_data_return.skip_conditions.iter() {
                                                if parse_skipif(skip_condition, &"Job too lazy to parse the site link if any".to_string(), database.clone(), &id, &jobid).is_some() {
                                                    continue 'jobloop;
                                                }
                                            }
                                            let mut job_storage = jobstorage.write();
                                            job_storage.jobs_add(scraper.clone(), scraper_data_return.job.clone());
                                        }


                                        // Spawns the multithreaded pool
                                        let pool = ThreadPool::default();

                                        // Adds tags into db
                                        database.tag_add_tagobject_multiple(&out_st.tags);

        log::info!("{:?}", &out_st.files);
                                        let files = out_st.files.clone().into_iter().enumerate().map(|(internal_id, file_raw)| {
    let file: sharedtypes::FileObjectMain = file_raw.clone().into();

                ( file_raw,FileStorage {
                                                internal_id: internal_id.try_into().unwrap(),
        status: FilesStatus::Waiting,
        hash: file.hash
    })
}).collect::<Vec<_>>();
                                        let mut  filestorage:Arc<RwLock<Vec<FileStorage>>> =Arc::new( RwLock::new( files.clone().into_iter().map(|f| f.1).collect()));

                                        // Adds files to scraper in UI
                                        uiscraper.send(UIScraper { worker: id, name: scraper.name.clone() ,status: ScraperStatus::Running, files: (*filestorage.read().clone()).to_vec()  });

                                        // Parses files from urls
                                        let ui_scraper = uisender.clone();
                                        let value = ui_scraper.clone();
                                        for ( file,file_ui )  in files.clone() {
                                            let ratelimiter_obj = ratelimiter_main.clone();
                                            let globalload = globalload.clone();
                                            let db = database.clone();
                                            let client = client_file.clone();
                                            let jobstorage = jobstorage.clone();
                                            let scraper = scraper.clone();let value = value.clone();
                                            let mut filestorage = filestorage.clone();
                                            let file = file.clone();
                                            pool.execute(move || {
                                                main_file_loop(
                                                    &mut file.into(),
                                                    db,
                                                    ratelimiter_obj,
                                                    globalload,
                                                    client,
                                                    jobstorage,
                                                    &scraper,
                                                    &id,
                                                    &jobid,
                                                    value,
                                                    &file_ui,
                                                   filestorage.clone()
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
                                    sharedtypes::ScraperReturn::Fatal(emc) => {
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
                                    // Puts the job back into the queu x seconds later
                                    sharedtypes::ScraperReturn::RetryLater(time) => {
                                        let mut data = job.clone();
                                        data.time = crate::time_func::time_secs();
                                        data.reptime = time.as_secs() ;
                                        database.jobs_update_db(data);
                                        should_remove_original_job = false;
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
                } );
            }
            threadflagcontrol.stop();
            jobstorage.write().clear_previously_seen_cache(&scraper);
        });
        Worker {
            id,
            thread: Some(thread),
            scraper: scraper_orig,
            uisender: ui_scraper,
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
    SkipExistingFile(u64),
    // Download that sucket
    Download,
}

///
/// Downloads a file into the db if needed
///
fn download_add_to_db(
    ratelimiter_obj: Arc<RwLock<Ratelimiter>>,
    source: &String,
    _location: String,
    globalload: GlobalLoad,
    client: Arc<RwLock<Client>>,
    database: Main,
    file: &mut sharedtypes::FileObjectMain,
    worker_id: &u64,
    job_id: &u64,
    scraper: &sharedtypes::GlobalPluginScraper,
    file_ui: FileStorage,
    file_ui_list: Arc<RwLock<Vec<FileStorage>>>,
    uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIScraper>>,
) -> Option<u64> {
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
            &source,
            worker_id,
            job_id,
            Some(scraper),
            file_ui,
            file_ui_list.clone(),
            uisender,
        );
    }

    match blopt {
        download::FileReturnStatus::File((_hash, _file_ext, file_id)) => {
            //let fileid;

            /* {
                        let ext_id = database.extension_put_string(&file_ext);

                        let storage_id = database.storage_put(&location);

                        let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                            hash,
                            ext_id,
                            storage_id,
                        });
                        fileid = database.file_add(file);
                        let source_url_ns_id = database.create_default_source_url_ns_id();
                        let tagid = database.tag_add(source, source_url_ns_id, None);
                        database.relationship_add(fileid, tagid);
                    }
            */
            return Some(file_id);
        }
        download::FileReturnStatus::DeadUrl(dead_url) => {
            database.add_dead_url(&dead_url);
        }
        _ => {}
    }

    None
}
/*
/// Simple code to add jobs from a tag object
fn parse_jobs(
    tag: &sharedtypes::TagObject,
    fileid: Option<u64>,
    jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
    database: Main,
    scraper: &sharedtypes::GlobalPluginScraper,

    worker_id: &u64,
    job_id: &u64,
    manager: GlobalLoad,
) {
    return;
    //let urls_to_scrape = parse_tags(database.clone(), tag, fileid, worker_id, job_id, manager);

    //let mut joblock = jobstorage.write();
    //for data in urls_to_scrape {
    //    joblock.jobs_add(scraper.clone(), data.job);
    //}
}
*/
/// Parses weather we should skip downloading the file
/// Returns a Some(u64) if the fileid exists
fn parse_skipif(
    skip_condition: &sharedtypes::SkipIf,
    file_url_source: &String,
    database: Main,
    worker_id: &u64,
    job_id: &u64,
) -> Option<u64> {
    match skip_condition {
        sharedtypes::SkipIf::NoFilesDownloaded => {}
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
                let vec: Vec<u64> = fids.iter().cloned().collect();
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
    file: &mut sharedtypes::FileObjectMain,
    database: Main,
    ratelimiter_obj: Arc<RwLock<Ratelimiter>>,
    globalload: GlobalLoad,
    client: Arc<RwLock<Client>>,
    jobstorage: Arc<RwLock<crate::jobs::Jobs>>,
    scraper: &sharedtypes::GlobalPluginScraper,
    worker_id: &u64,
    job_id: &u64,
    uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIScraper>>,
    file_ui: &FileStorage,
    file_ui_list: Arc<RwLock<Vec<FileStorage>>>,
) {
    let mut fileid = None;
    let source_url_id = database.create_default_source_url_ns_id();

    match file.source.clone() {
        Some(source) => match source {
            sharedtypes::FileSource::Url(source_url_list) => {
                if source_url_list.is_empty() {
                    return;
                }

                'source_list: for source_url in source_url_list {
                    if fileid.is_some() {
                        continue;
                    }

                    for file_tag in file.skip_if.iter() {
                        if let Some(file_id) =
                            parse_skipif(file_tag, &source_url, database.clone(), worker_id, job_id)
                        {
                            database.add_tags_to_fileid(Some(file_id), &file.tag_list);
                            return;
                        }
                    }

                    let location = database.location_get();
                    let url_tag = database.tag_get_name(source_url.clone(), source_url_id);

                    // === FIX: Clone our shared list handle for safely escaping the iteration block ===
                    let loop_ui_list_clone = Arc::clone(&file_ui_list);

                    fileid = match url_tag {
                        None => {
                            // Update UI State to Downloading using idiomatic .find() execution
                            {
                                let mut list_guard = loop_ui_list_clone.write();
                                if let Some(target) =
                                    list_guard.iter_mut().find(|f| **f == *file_ui)
                                {
                                    target.status = FilesStatus::Downloading(0.0);
                                }
                            }

                            let current_snapshot = loop_ui_list_clone.read().to_vec();
                            let _ = uisender.send(UIScraper {
                                worker: *worker_id,
                                name: scraper.name.clone(),
                                status: ScraperStatus::Running,
                                files: current_snapshot,
                            });

                            match download_add_to_db(
                                ratelimiter_obj.clone(),
                                &source_url,
                                location,
                                globalload.clone(),
                                client.clone(),
                                database.clone(),
                                file,
                                worker_id,
                                job_id,
                                scraper,
                                file_ui.clone(),
                                loop_ui_list_clone, // Safely forward the iteration local handle
                                uisender.clone(),
                            ) {
                                None => continue 'source_list,
                                Some(out) => Some(out),
                            }
                        }
                        Some(url_id) => {
                            // Update UI State to Done
                            {
                                let mut list_guard = loop_ui_list_clone.write();
                                if let Some(target) =
                                    list_guard.iter_mut().find(|f| **f == *file_ui)
                                {
                                    target.status = FilesStatus::Done;
                                }
                            }

                            let current_snapshot = loop_ui_list_clone.read().to_vec();
                            let _ = uisender.send(UIScraper {
                                worker: *worker_id,
                                name: scraper.name.clone(),
                                status: ScraperStatus::Running,
                                files: current_snapshot,
                            });

                            let file_id = database.relationship_get_one_fileid(&url_id);

                            match file_id {
                                Some(f_id) => {
                                    info_log(format!(
                                        "Worker: {worker_id} JobId: {job_id} -- Skipping file: {} Due to already existing in Tags Table.",
                                        &source_url
                                    ));
                                    Some(f_id)
                                }
                                None => {
                                    match download_add_to_db(
                                        ratelimiter_obj.clone(),
                                        &source_url,
                                        location,
                                        globalload.clone(),
                                        client.clone(),
                                        database.clone(),
                                        file,
                                        worker_id,
                                        job_id,
                                        scraper,
                                        file_ui.clone(),
                                        loop_ui_list_clone, // Safely forward the iteration local handle
                                        uisender.clone(),
                                    ) {
                                        None => continue 'source_list,
                                        Some(id) => Some(id),
                                    }
                                }
                            }
                        }
                    };
                    break 'source_list;
                }
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
                    uisender.clone(),
                    file_ui,
                    file_ui_list.clone(),
                );

                fileid = database.file_get_hash(&sha512.0);
                database.add_tags_to_fileid(fileid, &file.tag_list);
            }
        },
        None => return,
    }
}
