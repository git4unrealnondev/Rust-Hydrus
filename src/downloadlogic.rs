use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use ratelimit::Ratelimiter;
use thread_control::{Control, Flag, make_pair};
use tokio::task::JoinSet;
use tokio::time::{Instant, Sleep, sleep_until};

use crate::download::parse_skipif;
use crate::logging::error_log;
use crate::time_func::time_secs;
use crate::{RwLock, logging};
use crate::{
    database::database::Main, download, globalload::GlobalLoad, helpers::memory_manage, jobs::Jobs,
    logging::info_log, ui::ui::*,
};

pub struct LocalStorage {
    uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIEvent>>,
    pub db: Main,
    pub globalload: GlobalLoad,
    pub jobs: Arc<RwLock<Jobs>>,
    // UI Storage
    files: RwLock<HashMap<u64, HashMap<u64, Vec<FileStorage>>>>,
}

#[derive(Clone)]
pub struct ScraperInternal {
    id: u64,
    thread_control: Control,
    scraper: sharedtypes::GlobalPluginScraper,
    ratelimiter: Arc<Ratelimiter>,
    ctx: Arc<LocalStorage>,
}

pub struct DownloadManager {
    worker_storage: HashMap<u64, ScraperInternal>,
    flag_storage: HashMap<u64, Flag>,

    scraper_storage: HashMap<sharedtypes::GlobalPluginScraper, u64>,
    ctx: Arc<LocalStorage>,
    tokio_handle: tokio::runtime::Handle,
}

impl DownloadManager {
    ///
    /// Sets up the download manager
    ///
    pub fn new(
        uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIEvent>>,
        db: Main,
        globalload: GlobalLoad,
        jobs: Arc<RwLock<Jobs>>,
        tokio_handle: tokio::runtime::Handle,
    ) -> Self {
        // Going to do this here as we should be safe to do so
        db.load_table(&sharedtypes::LoadDBTable::All);

        let ctx = Arc::new(LocalStorage {
            uisender,
            db,
            globalload,
            jobs,
            files: HashMap::new().into(),
        });
        DownloadManager {
            worker_storage: HashMap::new(),
            flag_storage: HashMap::new(),
            scraper_storage: HashMap::new(),
            ctx,
            tokio_handle,
        }
    }

    ///
    /// Adds work for a scraper into the db
    ///
    pub fn add_work(
        &mut self,
        scraper: sharedtypes::GlobalPluginScraper,
    ) -> Option<ScraperInternal> {
        let ratelimit = match scraper.storage_type {
            Some(sharedtypes::ScraperOrPlugin::Scraper(ref scraper_info)) => scraper_info.ratelimit,
            _ => return None,
        };

        // Scan the existing keys in the map to see if we already have a scraper with this name
        let already_exists = self.scraper_storage.keys().any(|k| k.name == scraper.name);

        if !already_exists {
            let id: u64 = self.worker_storage.len().try_into().unwrap();
            let (thread_flag, thread_control) = make_pair();

            let scraperobject = ScraperInternal {
                id,
                thread_control,
                scraper: scraper.clone(),
                ratelimiter: Arc::new(download::ratelimiter_create(
                    &id,
                    &id,
                    ratelimit.0,
                    ratelimit.1,
                )),
                ctx: self.ctx.clone(),
            };

            logging::info_log(format!(
                "Adding background task for scraper: {}",
                scraper.name
            ));

            // Insert using the original type without changing your struct fields!
            self.scraper_storage.insert(scraper.clone(), id);
            self.flag_storage.insert(id, thread_flag);
            self.worker_storage.insert(id, scraperobject.clone());

            return Some(scraperobject);
        }

        if let Some(wid) = self.scraper_storage.get(&scraper) {
            self.worker_storage.get(wid).cloned()
        } else {
            None
        }
    }

    ///
    /// Checks status of scrapers clears the ones are are finished and
    /// RETURNS True if we have no running scrapers
    ///
    pub fn check_scrapers(&mut self) -> bool {
        // 1. Identify and remove dead workers from flag_storage in a single pass
        // (Assuming flag_storage is a HashMap)
        let mut dead_worker_ids = Vec::new();

        self.flag_storage.retain(|worker_id, flag| {
            if !flag.is_alive() {
                dead_worker_ids.push(*worker_id);
                false // Remove from flag_storage
            } else {
                true // Keep in flag_storage
            }
        });

        // 2. Clean up ALL other associated collections cleanly
        for worker_id in &dead_worker_ids {
            self.worker_storage.remove(worker_id);
            self.scraper_storage.retain(|_, id| id != worker_id);
        }

        // Returns true only if we have 0 workers remaining
        self.worker_storage.is_empty()
    }
}

impl LocalStorage {
    // Adds file to UI
    pub fn add_files(
        &self,
        worker_id: &u64,
        job_id: &u64,
        scraper: &sharedtypes::GlobalPluginScraper,
        files: Vec<FileStorage>,
    ) {
        // Acquire the write lock here to gain mutability over the collection
        let mut files_guard = self.files.write();

        let jobid_map = files_guard.entry(*worker_id).or_default();

        jobid_map.insert(*job_id, files.clone());

        /*  let _ = self.uisender.send(UIScraper {
            worker: *worker_id,
            name: scraper.name.clone(),
            status: ScraperStatus::Running,
            files: jobid_map.clone(),
        });*/
        self.uisender.send(UIEvent::ScraperStatusChanged {
            worker_id: *worker_id,
            name: scraper.name.clone(),
            status: ScraperStatus::Running,
        });

        for file in files {
            self.uisender.send(UIEvent::FileStatusChanged {
                worker_id: *worker_id,
                job_id: *job_id,
                file_id: file.internal_id,
                status: FilesStatus::Waiting,
            });
        }
    }

    // Updates a files info in UI
    pub fn update_file(&self, worker_id: &u64, job_id: &u64, file: &FileStorage) {
        self.uisender.send(UIEvent::FileStatusChanged {
            worker_id: *worker_id,
            job_id: *job_id,
            file_id: file.internal_id,
            status: file.status.clone(),
        });
    }
}

impl ScraperInternal {
    ///
    /// Marks the end of a scraper
    ///
    async fn finish_scraper(&self) {
        info_log(format!("Shutting Down Worker from Worker: {}", self.id));

        // Sets up a default UI for the old scraper
        /*let _ = self.ctx.uisender.send(UIScraper {
            worker: self.id,
            name: self.scraper.name.clone(),
            status: ScraperStatus::Completed,
            files: HashMap::new(),
        });*/

        self.ctx.uisender.send(UIEvent::ScraperStatusChanged {
            worker_id: self.id,
            name: self.scraper.name.clone(),
            status: ScraperStatus::Completed,
        });

        self.thread_control.stop();

        //memory_manage();
    }

    ///
    /// Runs on startup of scraper
    ///
    async fn setup_scraper(&self) {
        // Initial UI Send
        /*let _ = self.ctx.uisender.send(UIScraper {
            worker: self.id,
            name: self.scraper.name.clone(),
            status: ScraperStatus::Idle,
            files: HashMap::new(),
        });*/
        self.ctx.uisender.send(UIEvent::ScraperStatusChanged {
            worker_id: self.id,
            name: self.scraper.name.clone(),
            status: ScraperStatus::Idle,
        });
    }

    ///
    /// Handles recursion for AlwaysTime in db
    ///
    async fn process_recursion_time(&self, job: &sharedtypes::DbJobsObj) {
        if let Some(ref recursion) = job.jobmanager.recreation
            && let sharedtypes::DbJobRecreation::AlwaysTime(timestamp, count) = recursion
        {
            let mut temp_job = job.clone();
            temp_job.time = time_secs();
            temp_job.reptime = *timestamp;

            if let Some(count) = count {
                if *count == 0 {
                    self.ctx
                        .jobs
                        .write()
                        .jobs_remove_dbjob(&self.scraper, &job, &self.id);
                } else {
                    temp_job.jobmanager.recreation = Some(
                        sharedtypes::DbJobRecreation::AlwaysTime(*timestamp, Some(count - 1)),
                    );
                    self.ctx.jobs.write().jobs_update(temp_job, &self.scraper);
                }
            }
        }
    }

    ///
    /// Actually runs a job
    ///
    async fn run_job(&self, job: sharedtypes::DbJobsObj) {
        self.ctx
            .jobs
            .write()
            .job_set_is_running(&self.scraper, &job);

        // Inits local files mapping to zero
        let mut map = HashMap::new();
        map.insert(job.id.unwrap_or(0), Vec::new());
        self.ctx.files.write().insert(self.id, map);

        self.process_recursion_time(&job).await;

        let mut scraper = self.scraper.clone();
        let mut job = job.clone();

        let modifiers = download::get_modifiers(&self.scraper);
        let client_text = Arc::new(download::client_create(modifiers.clone(), true));
        let client_file = Arc::new(download::client_create(modifiers, false));

        // Loads stuff from compile time into user_data
        if let Some(ref stored_info) = scraper.stored_info {
            match stored_info {
                sharedtypes::StoredInfo::Storage(storage) => {
                    for (key, val) in storage.iter() {
                        job.user_data.insert(key.to_string(), val.to_string());
                    }
                }
            }
        }

        let scraper_data_return_default = sharedtypes::ScraperDataReturn {
            job: job.clone(),
            ..Default::default()
        };

        let urlload = match job.jobmanager.jobtype {
            sharedtypes::DbJobType::Params => {
                let mut out = Vec::new();

                match self.ctx.globalload.url_dump(
                    &job.param,
                    &scraper_data_return_default,
                    &scraper,
                ) {
                    Ok(temp) => {
                        for scraper_data_return in temp {
                            for param in scraper_data_return.job.param.iter() {
                                if let sharedtypes::ScraperParam::Url(_)
                                | sharedtypes::ScraperParam::UrlPost(_) = param
                                {
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
                            "Worker: {} JobId: {} -- While trying to parse parameters we got this error: {:?}",
                            self.id, self.id, err
                        ));
                        logging::error_log(format!(
                            "Worker: {} JobId: {} -- Telling system to keep job due to previous error.",
                            self.id, self.id
                        ));
                        self.ctx.jobs.write().jobs_remove_job(&scraper, &job);
                    }
                }
                out
            }
            sharedtypes::DbJobType::Plugin => {
                return;
            }
            sharedtypes::DbJobType::NoScrape => {
                let mut out = Vec::new();
                for param in job.param.iter() {
                    if let sharedtypes::ScraperParam::Url(_) = param {
                        out.push((param.clone(), scraper_data_return_default.clone()));
                    }
                }
                out
            }
            sharedtypes::DbJobType::FileUrl => Vec::new(),
            sharedtypes::DbJobType::Scraper => {
                let mut out = Vec::new();
                for param in job.param.iter() {
                    if let sharedtypes::ScraperParam::Url(_)
                    | sharedtypes::ScraperParam::UrlPost(_) = param
                    {
                        out.push((param.clone(), scraper_data_return_default.clone()));
                    }
                }
                out
            }
        };

        'urlloop: for (scraperparam, scraperdata) in urlload {
            // Temp variables for text parsing
            let resp;
            let scraper_return;

            match &scraperparam {
                sharedtypes::ScraperParam::Url(url_string) => {
                    if !scraper.should_handle_text_scraping {
                        resp = download::dltext_new(
                            url_string,
                            None,
                            client_text.clone(),
                            &self.ratelimiter,
                            &self.id,
                            &job.id.unwrap_or(0),
                        )
                        .await;
                        scraper_return = match resp {
                            Ok((respstring, resp_url)) => self.ctx.globalload.parser_call(
                                &respstring,
                                &resp_url,
                                &scraperdata,
                                &scraper,
                            ),
                            Err(err) => {
                                logging::error_log(format!(
                                    "Worker: {} -- While processing job {:?} was unable to download text. Had err {:?}",
                                    &self.id, &job, err
                                ));
                                break 'urlloop;
                            }
                        };
                    } else {
                        scraper_return = self.ctx.globalload.text_scraping(
                            url_string,
                            &scraperdata.job.param,
                            &scraperdata,
                            &scraper,
                        )
                    }
                }
                sharedtypes::ScraperParam::UrlPost(url_string) => {
                    resp = download::dltext_new(
                        &url_string.url,
                        Some(url_string.post_data.clone()),
                        client_text.clone(),
                        &self.ratelimiter,
                        &self.id,
                        &job.id.unwrap_or(0),
                    )
                    .await;
                    scraper_return = match resp {
                        Ok((respstring, resp_url)) => self.ctx.globalload.parser_call(
                            &respstring,
                            &resp_url,
                            &scraperdata,
                            &scraper,
                        ),
                        Err(err) => {
                            logging::error_log(format!(
                                "Worker: {} -- While processing job {:?} was unable to download text. Had err {:?}",
                                &self.id, &job, err
                            ));
                            break 'urlloop;
                        }
                    };
                }
                _ => {
                    break 'urlloop;
                }
            }

            for scrap in scraper_return {
                match scrap {
                    sharedtypes::ScraperReturn::Data(scrap_data) => {
                        // Loop thru jobs and add them if we have no skip conditions
                        'jobloop: for scraper_data_return in scrap_data.jobs.iter() {
                            for skip_condition in scraper_data_return.skip_conditions.iter() {
                                if parse_skipif(
                                    skip_condition,
                                    &"Job too lazy to parse the site link if any".to_string(),
                                    self.ctx.db.clone(),
                                    &self.id,
                                    &job.id.unwrap_or(0),
                                    &self.ctx.clone(),
                                )
                                .is_some()
                                {
                                    continue 'jobloop;
                                }
                            }
                            let mut job_storage = self.ctx.jobs.write();
                            job_storage.jobs_add(scraper.clone(), scraper_data_return.job.clone());
                        }

                        // Start parsing files
                        let scrap_files = scrap_data
                            .files
                            .clone()
                            .into_iter()
                            .enumerate()
                            .map(|(internal_id, file_raw)| {
                                let file: sharedtypes::FileObjectMain = file_raw.clone().into();

                                let file_storage = match file.source {
                                    None => None,
                                    Some(_) => Some(FileStorage {
                                        internal_id: internal_id.try_into().unwrap(),
                                        status: FilesStatus::Waiting,
                                        hash: file.hash,
                                    }),
                                };

                                (file_raw, file_storage)
                            })
                            .collect::<Vec<_>>();

                        let files_storage = scrap_files.iter().filter_map(|f| f.1.clone()).collect();

                        /*   logging::info_log(format!(
                            "SCRAP RETURNS: {:?} {:?}",
                            &scrap_files, &files_storage
                        ));*/
                        self.ctx.add_files(
                            &self.id,
                            &job.id.unwrap_or(0),
                            &self.scraper,
                            files_storage,
                        );

                        //logging::info_log(format!("SCRAPER DATA RETURN DATA: {:?}", scrap_data));

                        let mut set = JoinSet::new();
                        for (file, file_storage) in scrap_files {
                            let file = file.clone();
                            let scraper = scraper.clone();
                            let ctx = self.ctx.clone();
                            let client_file = client_file.clone();
                            let ratelimiter = self.ratelimiter.clone();
                            let worker_id = self.id;
                            let job_id = job.id.unwrap_or(0);
                            let file_storage = file_storage.clone();
                            set.spawn(async move {
                                download::main_file_loop(
                                    &mut file.into(),
                                    client_file,
                                    &scraper,
                                    &worker_id,
                                    &job_id,
                                    ctx,
                                    &ratelimiter,
                                    file_storage,
                                )
                                .await
                            });
                        }
                        set.join_all().await;
                    }
                    sharedtypes::ScraperReturn::Nothing => {
                        logging::info_log(format!(
                            "Worker: {}  -- Exiting loop due to nothing.",
                            self.id,
                        ));
                        break 'urlloop;
                    }
                    sharedtypes::ScraperReturn::Stop(stop_string) => {
                        logging::error_log(format!("Stopping job: {:?}", stop_string));
                        break 'urlloop;
                    }
                    sharedtypes::ScraperReturn::Fatal(fatal_string) => {
                        panic!("EMC STOP DUE TO: {}", fatal_string);
                    }
                    sharedtypes::ScraperReturn::Timeout(timeout_time) => {
                        let time_dur = Duration::from_secs(timeout_time);
                        tokio::time::sleep(time_dur).await;
                        //thread::sleep(time_dur);
                        continue;
                    }
                    sharedtypes::ScraperReturn::RetryLater(try_later_time) => {
                        let mut data = job.clone();
                        data.id = None;
                        data.time = crate::time_func::time_secs();
                        data.reptime = try_later_time.as_secs();
                        self.ctx.jobs.write().jobs_add(self.scraper.clone(), data);
                    }
                }
            }
        }

        self.ctx
            .jobs
            .write()
            .jobs_remove_dbjob(&self.scraper, &job, &self.id);
    }
    pub async fn start_scraper(self: Arc<Self>) {
        let mut set = JoinSet::new();

        'mainloop: loop {
            let jobstorage = self.ctx.jobs.read().jobs_get_priority_order(&self.scraper);

            if jobstorage.is_empty() {
                self.setup_scraper().await;
                break 'mainloop;
            }

            let mut spawned_any_work = false;

            for job in jobstorage {
                if !job.isrunning {
                    let scraper_worker = Arc::clone(&self);
                    set.spawn(async move { scraper_worker.run_job(job).await });
                    spawned_any_work = true;
                }
            }

            // FIX: If jobs exist in the DB cache, but ALL of them are already
            // running, don't spin wildly. Back off or exit the current tick loop.
            if !spawned_any_work && set.is_empty() {
                // No new jobs were spawned, and no old background tasks are running to wait on.
                // Break out to avoid an infinite high-CPU loop.
                logging::info_log("All found jobs are already running. Exiting loop safely.");
                break 'mainloop;

                //return;
            }

            // Waits for all current parallel batches to finish
            while let Some(res) = set.join_next().await {
                if let Err(e) = res {
                    error_log(format!("A parallel job panicked: {:?}", e));
                }
            }
            self.finish_scraper().await;
        }
    }
}
