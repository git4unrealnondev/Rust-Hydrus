use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use ratelimit::Ratelimiter;
use rayon::ThreadPool;
use thread_control::{Control, Flag, make_pair};
use tokio::task::JoinSet;

use crate::download::parse_skipif;
use crate::{RwLock, logging};
use crate::{
    database::database::Main, download, globalload::GlobalLoad, jobs::Jobs, logging::info_log,
    ui::ui::*,
};

pub struct LocalStorage {
    uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIEvent>>,
    pub db: Main,
    pub globalload: GlobalLoad,
    pub jobs: Arc<Jobs>,
    // UI Storage
    files: RwLock<HashMap<u64, HashMap<u64, Vec<FileStorage>>>>,
    //
    pub heavy_processing_pool: Arc<ThreadPool>,
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
    /// Sets up the download manager
    pub fn new(
        uisender: Arc<tokio::sync::mpsc::UnboundedSender<UIEvent>>,
        db: Main,
        globalload: GlobalLoad,
        jobs: Arc<Jobs>,
        tokio_handle: tokio::runtime::Handle,
        heavy_processing_pool: Arc<ThreadPool>,
    ) -> Self {
        db.load_table(&sharedtypes::LoadDBTable::All);

        let ctx = Arc::new(LocalStorage {
            uisender,
            db,
            globalload,
            jobs,
            files: HashMap::new().into(),
            heavy_processing_pool,
        });
        DownloadManager {
            worker_storage: HashMap::new(),
            flag_storage: HashMap::new(),
            scraper_storage: HashMap::new(),
            ctx,
            tokio_handle,
        }
    }

    /// Adds work for a scraper into the db
    pub fn add_work(
        &mut self,
        scraper: sharedtypes::GlobalPluginScraper,
    ) -> Option<ScraperInternal> {
        let ratelimit = match scraper.storage_type {
            Some(sharedtypes::ScraperOrPlugin::Scraper(ref scraper_info)) => scraper_info.ratelimit,
            _ => return None,
        };

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

    /// Checks status of scrapers clears the ones are are finished
    pub fn check_scrapers(&mut self) -> bool {
        let mut dead_worker_ids = Vec::new();

        self.flag_storage.retain(|worker_id, flag| {
            if !flag.is_alive() {
                dead_worker_ids.push(*worker_id);
                false
            } else {
                true
            }
        });

        if !dead_worker_ids.is_empty() {
            for worker_id in &dead_worker_ids {
                self.worker_storage.remove(worker_id);
            }
            // Optimized into a clean single-pass cleanup
            self.scraper_storage
                .retain(|_, id| !dead_worker_ids.contains(id));
        }

        self.worker_storage.is_empty()
    }
}

impl LocalStorage {
    pub fn has_active_downloads(&self, worker_id: &u64) -> bool {
        let files_guard = self.files.read();
        if let Some(job_map) = files_guard.get(&worker_id) {
            for file_list in job_map.values() {
                // If any file is still waiting or currently downloading, the worker isn't done
                if file_list.iter().any(|f| f.status == FilesStatus::Waiting) {
                    return true;
                }
            }
        }
        false
    }

    // Adds file to UI
    pub fn add_files(
        &self,
        worker_id: &u64,
        job_id: &u64,
        scraper: &sharedtypes::GlobalPluginScraper,
        files: Vec<FileStorage>,
    ) {
        {
            let mut files_guard = self.files.write();
            // Secure both levels of the HashMap safely
            files_guard
                .entry(*worker_id)
                .or_default()
                .insert(*job_id, files.clone()); // Explicitly bound to this exact job ID slot
        } // Drop write lock instantly

        // Signal UI immediately that files have been registered
        let _ = self.uisender.send(UIEvent::ScraperStatusChanged {
            worker_id: *worker_id,
            name: scraper.name.clone(),
            status: ScraperStatus::Running,
        });

        for file in files {
            let _ = self.uisender.send(UIEvent::FileStatusChanged {
                worker_id: *worker_id,
                job_id: *job_id,
                file_id: file.internal_id,
                status: FilesStatus::Waiting,
            });
        }
    }

    // Updates a files info in UI
    // Updates a file's info in local storage AND alerts UI channel immediately
    pub fn update_file(&self, worker_id: &u64, job_id: &u64, file: &FileStorage) {
        {
            let mut files_guard = self.files.write();
            if let Some(job_map) = files_guard.get_mut(worker_id) {
                if let Some(file_list) = job_map.get_mut(job_id) {
                    // Find the specific file by its unique internal id and update its state
                    if let Some(target_file) = file_list
                        .iter_mut()
                        .find(|f| f.internal_id == file.internal_id)
                    {
                        target_file.status = file.status.clone();
                        target_file.hash = file.hash.clone();
                    }
                }
            }
        } // Drop write guard immediately

        // Emit event down the pipe so downstream UI loop receives it instantly
        let _ = self.uisender.send(UIEvent::FileStatusChanged {
            worker_id: *worker_id,
            job_id: *job_id,
            file_id: file.internal_id,
            status: file.status.clone(),
        });
    }
}

impl ScraperInternal {
    fn finish_scraper(&self) {
        info_log(format!("Shutting Down Worker from Worker: {}", self.id));

        if self.ctx.files.read().is_empty() {
            let _ = self.ctx.uisender.send(UIEvent::ScraperStatusChanged {
                worker_id: self.id,
                name: self.scraper.name.clone(),
                status: ScraperStatus::Completed,
            });
        }

        self.thread_control.stop();
    }

    fn setup_scraper(&self) {
        let _ = self.ctx.uisender.send(UIEvent::ScraperStatusChanged {
            worker_id: self.id,
            name: self.scraper.name.clone(),
            status: ScraperStatus::Idle,
        });
    }

    fn process_recursion_time(&self, job: &sharedtypes::DbJobsObj) -> bool {
        if let Some(ref recursion) = job.jobmanager.recreation {
            if let sharedtypes::DbJobRecreation::AlwaysTime(timestamp, count) = recursion {
                let mut data = job.clone();
                data.time = crate::time_func::time_secs();
                data.reptime = *timestamp;
                if count.is_some() {
                    self.ctx
                        .jobs
                        .jobs_decrement_count(&data, &self.scraper, &job.id.unwrap_or(0));
                }
                self.ctx.db.jobs_update_db(data);
            }
            return true;
        }
        false
    }

    async fn run_job(&self, job: sharedtypes::DbJobsObj) {
        let mut should_remove_job = true;

        {
            self.ctx.jobs.job_set_is_running(&self.scraper, &job);
        }
        // FIX: Insert safely using entry to prevent erasing other parallel jobs sharing this worker ID
        // Safely ensure the nested JobId slot exists without touching any other parallel jobs
        self.ctx
            .files
            .write()
            .entry(self.id)
            .or_default()
            .entry(job.id.unwrap_or(0))
            .or_default(); // Uses .or_default() instead of .insert() so it never overwrites!

        {
            let mut files_guard = self.ctx.files.write();
            let job_id = job.id.unwrap_or(0);
            files_guard
                .entry(self.id)
                .or_default()
                .insert(job_id, Vec::new());
        }
        if self.process_recursion_time(&job) {
            should_remove_job = false;
        }

        let scraper = self.scraper.clone();
        let mut job = job.clone();

        let modifiers = download::get_modifiers(&self.scraper);
        let client_text = Arc::new(download::client_create(modifiers.clone(), true));
        let client_file = Arc::new(download::client_create(modifiers, false));

        if let Some(ref _stored_info) = scraper.stored_info {
            match _stored_info {
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
                    }
                    Err(err) => {
                        logging::error_log(format!(
                            "Worker: {} JobId: {} -- Parameter parsing error: {:?}",
                            self.id, self.id, err
                        ));
                        self.ctx.jobs.jobs_remove_job(&scraper, &job);
                        should_remove_job = false;
                    }
                }
                out
            }
            sharedtypes::DbJobType::Plugin => return,
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
                                    "Worker: {} -- Text download failed: {:?}",
                                    self.id, err
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
                        );
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
                                "Worker: {} -- POST download failed: {:?}",
                                self.id, err
                            ));
                            break 'urlloop;
                        }
                    };
                }
                _ => break 'urlloop,
            }
            logging::info_log(format!(
                "WORKER: {} JOB: {:?} : after parsing ",
                &self.id, &job.id
            ));

            for scrap in scraper_return {
                match scrap {
                    sharedtypes::ScraperReturn::Data(scrap_data) => {
                        let ctx = self.ctx.clone();
                        let scrap_data_clone = scrap_data.clone(); // If cheap enough, or pass parts
                        let id = self.id;
                        let job_id = job.id.unwrap_or(0);
                        let scraper_for_blocking = scraper.clone();

                        tokio::task::spawn_blocking(move || {
                            ctx.db.tag_add_tagobject_multiple(&scrap_data_clone.tags);

                            'jobloop: for scraper_data_return in scrap_data_clone.jobs.iter() {
                                for skip_condition in scraper_data_return.skip_conditions.iter() {
                                    if parse_skipif(
                                        skip_condition,
                                        &"Job link skipped".to_string(),
                                        ctx.db.clone(),
                                        &id,
                                        &job_id,
                                        &ctx,
                                    )
                                    .is_some()
                                    {
                                        continue 'jobloop;
                                    }
                                }

                                // 2. Clone it HERE per iteration so the loop doesn't steal ownership
                                let scraper_iteration_clone = scraper_for_blocking.clone();

                                ctx.jobs.jobs_add(
                                    scraper_iteration_clone,
                                    scraper_data_return.job.clone(),
                                );
                            }
                        });

                        logging::info_log(format!(
                            "WORKER: {} JOB: {:?} : after jobs ",
                            &self.id, &job.id
                        ));
                        let scrap_files = scrap_data
                            .files
                            .clone()
                            .into_iter()
                            .enumerate()
                            .map(|(internal_id, file_raw)| {
                                let file: sharedtypes::FileObjectMain = file_raw.clone().into();
                                let file_storage = file.source.as_ref().map(|_| FileStorage {
                                    internal_id: internal_id.try_into().unwrap(),
                                    status: FilesStatus::Waiting,
                                    hash: file.hash,
                                });
                                (file_raw, file_storage)
                            })
                            .collect::<Vec<_>>();

                        let files_storage =
                            scrap_files.iter().filter_map(|f| f.1.clone()).collect();
                        self.ctx.add_files(
                            &self.id,
                            &job.id.unwrap_or(0),
                            &self.scraper,
                            files_storage,
                        );

                        logging::info_log(format!(
                            "WORKER: {} JOB: {:?} : after files ",
                            &self.id, &job.id
                        ));
                        let mut set = JoinSet::new();
                        for (file, file_storage) in scrap_files {
                            let file = file.clone();
                            let scraper = scraper.clone();
                            let ctx = self.ctx.clone();
                            let client_file = client_file.clone();
                            let ratelimiter = self.ratelimiter.clone();
                            let worker_id = self.id;
                            let job_id = job.id.unwrap_or(0);
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
                        logging::info_log(format!(
                            "WORKER: {} JOB: {:?} : after file downloading ",
                            &self.id, &job.id
                        ));
                    }
                    sharedtypes::ScraperReturn::Nothing => {
                        logging::info_log(format!(
                            "Worker: {} -- Exiting loop due to Nothing.",
                            self.id
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
                        tokio::time::sleep(Duration::from_secs(timeout_time)).await;
                        continue;
                    }
                    sharedtypes::ScraperReturn::RetryLater(try_later_time) => {
                        let mut data = job.clone();
                        data.time = crate::time_func::time_secs();
                        data.reptime = try_later_time.as_secs();
                        self.ctx.jobs.jobs_add(self.scraper.clone(), data);
                        should_remove_job = false;
                    }
                }
            }
        }

        if should_remove_job {
            self.ctx
                .jobs
                .jobs_remove_dbjob(&self.scraper, &job, &self.id);

            let worker_id = self.id;
            let job_id = job.id.unwrap_or(0);
            let uisender = self.ctx.uisender.clone();
            let ctx_files = self.ctx.clone(); // Clone the Arc pointer to files

            // Clears UI stuff
            tokio::spawn(async move {
                // Hold the completed state on the terminal screen for 3 seconds
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                // Evicts data from internal structure and sends UI job
                ctx_files
                    .files
                    .write()
                    .get_mut(&worker_id)
                    .map(|job_map| job_map.remove(&job_id));

                let _ = uisender.send(UIEvent::ClearJob { worker_id, job_id });
            });
        }
    }

    pub async fn start_scraper(self: Arc<Self>) {
        'mainloop: loop {
            let jobstorage = self.ctx.jobs.jobs_get_priority_order(&self.scraper);

            if jobstorage.is_empty() {
                self.setup_scraper();
                break 'mainloop;
            }

            let mut spawned_any_work = false;

            let mut set = JoinSet::new();
            for job in jobstorage {
                if !job.isrunning {
                    let scraper_worker = Arc::clone(&self);
                    set.spawn(async move { scraper_worker.run_job(job).await });
                    spawned_any_work = true;
                }
            }

            if !spawned_any_work && set.is_empty() {
                logging::info_log("All found jobs are already running. Exiting loop safely.");
                logging::info_log(format!(
                    "SETSPAWNED {} SET {}",
                    spawned_any_work,
                    set.is_empty()
                ));
                //   self.finish_scraper();
                break 'mainloop;
            }

            // Waits for all current parallel batches to finish
            while let Some(res) = set.join_next().await {
                if let Err(e) = res {
                    logging::error_log(format!("A parallel job panicked: {:?}", e));
                }
            }
        }
    }
}
