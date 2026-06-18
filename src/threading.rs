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
use reqwest::Client;
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
            /*    let _ = self.uisender.send(UIScraper {
                worker: self.id,
                name: self.scraper.name.clone(),
                status: ScraperStatus::Completed,
                files: vec![],
            });*/

            memory_manage();
        }
    }
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
        /*   uisender.send(UIScraper {
            worker: id,
            name: scraper_orig.name.clone(),
            status: ScraperStatus::Idle,
            files: vec![],
        });*/

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
//uisender.send(UIScraper { worker: id, name: scraper.name.clone() ,status: ScraperStatus::Running, files: vec![] });
            /*        'urlloop: for (scraperparam, scraperdata) in urlload {
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
                           //             uiscraper.send(UIScraper { worker: id, name: scraper.name.clone() ,status: ScraperStatus::Running, files: (*filestorage.read().clone()).to_vec()  });

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
                    }*/
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



