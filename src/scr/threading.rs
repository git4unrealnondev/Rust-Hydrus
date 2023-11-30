use crate::database;
use crate::download;

use crate::logging;
//use crate::jobs::JobsRef;
use crate::logging::{error_log, info_log};
use crate::pause;
use crate::plugins::PluginManager;
use crate::scraper;

use crate::sharedtypes;
use crate::sharedtypes::ScraperReturn;

use ahash::AHashMap;
use async_std::task;

use futures;

//use log::{error, info};

use std::collections::HashSet;
use std::ops::Index;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

//use tokio::runtime::Runtime;

pub struct threads {
    _workers: Vec<Worker>,
    //_runtime: Runtime,
}

///
/// Holder for workers.
/// Workers manage their own threads.
///
impl threads {
    pub fn new() -> Self {
        let workers = Vec::new();

        //let rt = Runtime::new().unwrap();

        threads {
            _workers: workers,
            //_runtime: rt,
        }
    }

    ///
    /// Creates a pool of threads. need to add checking.
    ///
    pub fn creates_thread_pool(&mut self, size: usize) {
        for _id in 0..size {
            //self._workers.push(Worker::new(id), );
        }
    }

    ///
    /// Adds a worker to the threadvec.
    ///
    pub fn startwork(
        &mut self,
        scraper: scraper::InternalScraper,
        jobs: Vec<sharedtypes::DbJobsObj>,
        db: &mut Arc<Mutex<database::Main>>,
        scrapermanager: libloading::Library,
        pluginmanager: Arc<Mutex<PluginManager>>,
    ) {
        let worker = Worker::new(
            self._workers.len(),
            scraper,
            jobs,
            //&mut self._runtime,
            db,
            scrapermanager,
            pluginmanager,
        );

        self._workers.push(worker);
    }
}
///
/// Worker holder for data. Will add a scraper processor soon tm.
///
struct Worker {
    id: usize,
    scraper: scraper::InternalScraper,
    jobs: Vec<sharedtypes::DbJobsObj>,
    thread: Option<std::thread::JoinHandle<()>>,
}

///
/// When code get deleted (cleaned up. This code runs.)
///  Cleans thread from pool.  
///
/*impl Drop for threads {
    fn drop(&mut self) {
        for worker in &mut self._workers {
            if let Some(thread) = worker.thread.take() {
                info!("Shutting Down Worker from ThreadManager: {}", worker.id);
                futures::executor::block_on(async { thread.await.unwrap()});
            }
        }
    }
}*/

///
/// closes the thread that the worker contains.
/// Used in easy thread handeling
/// Only reason to do this over doing this with default drop behaviour is the logging.
///
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            info_log(&format!("Shutting Down Worker from Worker: {}", self.id));
            println!("Shutting Down Worker from Worker: {}", self.id);
            futures::executor::block_on(async { thread.join().unwrap() });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct JobScraper {
    pub site: String,
    pub param: Vec<sharedtypes::ScraperParam>,
    pub original_param: String,
}

impl Worker {
    fn new(
        id: usize,
        scraper: scraper::InternalScraper,
        jobs: Vec<sharedtypes::DbJobsObj>,
        //rt: &mut Runtime,
        dba: &mut Arc<Mutex<database::Main>>,
        libloading: libloading::Library,
        pluginmanager: Arc<Mutex<PluginManager>>,
    ) -> Worker {
        info_log(&format!(
            "Creating Worker for id: {} Scraper Name: {} With a jobs length of: {}",
            &id,
            &scraper._name,
            &jobs.len()
        ));
        let mut db = dba.clone();
        let jblist = jobs.clone();
        let manageeplugin = pluginmanager;
        let scrap = scraper.clone();

        let thread = thread::spawn(move || {
            let mut job_params: HashSet<JobScraper> = HashSet::new();

            // Loops through jobs and adds them into the job_params
            for job in jblist {
                let mut par_vec: Vec<sharedtypes::ScraperParam> = Vec::new();
                let parpms: Vec<String> = job
                    .param
                    .as_ref()
                    .unwrap()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                for par in parpms {
                    let temp = sharedtypes::ScraperParam {
                        param_data: par,
                        param_type: sharedtypes::ScraperParamType::Normal,
                    };
                    par_vec.push(temp)
                }

                {
                    // Gets info from DB. If it exists then insert into params hashset.
                    let unwrappydb = &mut db.lock().unwrap();
                    let datafromdb = unwrappydb
                        .settings_get_name(&format!("{}_{}", scrap._type, scrap._name.to_owned()))
                        .unwrap()
                        .param
                        .clone();
                    match datafromdb {
                        None => {}
                        Some(param) => {
                            // Adds database tag if applicable.
                            let scrap_data = sharedtypes::ScraperParam {
                                param_data: param,
                                param_type: sharedtypes::ScraperParamType::Database,
                            };

                            par_vec.push(scrap_data);
                        }
                    }
                }

                let sc = JobScraper {
                    site: job.site.unwrap(),
                    param: par_vec,
                    original_param: job.param.unwrap(),
                };

                job_params.insert(sc);
            }
            dbg!(&job_params);

            // Main loop for processing
            // All queries have been deduplicated.
            let mut job_loop = true;
            while job_loop {
                for job in &job_params.clone() {
                    let urlload = scraper::url_dump(&libloading, &job.param);

                    dbg!(&urlload);
                    let mut ratelimit =
                        download::ratelimiter_create(scrap._ratelimit.0, scrap._ratelimit.1);

                    let mut client = download::client_create();

                    'urlloop: for urll in urlload {
                        'errloop: loop {
                            download::ratelimiter_wait(&mut ratelimit);
                            let resp = task::block_on(download::dltext_new(
                                urll.to_string(),
                                &mut ratelimit,
                                &mut client,
                                manageeplugin.clone(),
                            ));
                            let st = match resp {
                                Ok(respstring) => scraper::parser_call(&libloading, &respstring),
                                Err(_) => continue,
                            };

                            let out_st = match st {
                                Ok(objectscraper) => objectscraper,
                                Err(ScraperReturn::Nothing) => {
                                    job_params.remove(job);
                                    dbg!("Exiting loop due to nothing.");
                                    break 'urlloop;
                                }
                                Err(ScraperReturn::EMCStop(emc)) => {
                                    panic!("EMC STOP DUE TO: {}", emc);
                                }
                                Err(ScraperReturn::Stop(stop)) => {
                                    logging::error_log(&format!(
                                        "Stopping job: {:?} due to {}",
                                        job.param, stop
                                    ));
                                    job_params.remove(job);
                                    continue;
                                }
                                Err(ScraperReturn::Timeout(time)) => {
                                    let time_dur = Duration::from_secs(time);
                                    thread::sleep(time_dur);
                                    continue;
                                }
                            };

                            // debug loop for dumping all the gubbins.
                            for fi in out_st.file.iter() {
                                for ta in fi.tag_list.iter() {
                                    //println!("dbgloop {:?}", ta);
                                }
                            }
                            for fog in out_st.tag.iter() {
                                println!("dbgtagloop {:?}", fog);
                            }

                            // Represents our tag's to be associated with a file. per file
                            for file in out_st.file {
                                //dbg!(&file.source_url);
                                //let source_url = ;
                                //let hash = file.1.hash;
                                //let tag_list = file.1.tag_list;

                                // If we have a url source then check if it exists and download it.
                                if let Some(ref source) = file.source_url {
                                    {
                                        let mut source_url_id = {
                                            let unwrappydb = &mut db.lock().unwrap();
                                            unwrappydb
                                                .namespace_get(&"source_url".to_string())
                                                .cloned() // defaults to 0 due to unknown.
                                        };

                                        if source_url_id.is_none() {
                                            // Namespace doesn't exist. Will create
                                            let unwrappydb = &mut db.lock().unwrap();
                                            unwrappydb.namespace_add(
                                                "source_url".to_string(),
                                                Some("Source URL for a file.".to_string()),
                                                true,
                                            );
                                            log::info!(
                                                "Adding namespace {} with an id {} due to not existing.",
                                                "source_url",
                                                "0"
                                            );
                                            source_url_id = unwrappydb
                                                .namespace_get(&"source_url".to_string())
                                                .cloned();
                                            // defaults to 0 due to unknown.
                                        }

                                        // If url exists in db then don't download
                                        let url_tag = {
                                            let unwrappydb = db.lock().unwrap();
                                            unwrappydb
                                                .tag_get_name(
                                                    source.clone(),
                                                    source_url_id.unwrap().clone(),
                                                )
                                                .cloned()
                                        };

                                        let location = {
                                            let unwrappydb = &mut db.lock().unwrap();
                                            unwrappydb
                                                .settings_get_name(&"FilesLoc".to_string())
                                                .unwrap()
                                                .param
                                                .as_ref()
                                                .unwrap()
                                                .to_owned()
                                        };

                                        // Get's the hash & file ext for the file.
                                        let fileid = match url_tag {
                                            None => {
                                                // Download file doesn't exist.
                                                download::ratelimiter_wait(&mut ratelimit);
                                                // URL doesn't exist in DB Will download
                                                info_log(&format!(
                                                    "Downloading: {} to: {}",
                                                    &source, &location
                                                ));

                                                let (hash, file_ext) =
                                                    task::block_on(download::dlfile_new(
                                                        &client,
                                                        &file,
                                                        &location,
                                                        manageeplugin.clone(),
                                                    ));
                                                let unwrappydb = &mut db.lock().unwrap();
                                                let fileid = unwrappydb.file_add(
                                                    None, &hash, &file_ext, &location, true,
                                                );
                                                let tagid = unwrappydb.tag_add(
                                                    source.to_string(),
                                                    source_url_id.unwrap().clone(),
                                                    true,
                                                    None,
                                                );
                                                unwrappydb.relationship_add(fileid, tagid, true);
                                                fileid
                                            }
                                            Some(url_id) => {
                                                // We've already got a valid relationship
                                                let unwrappydb = &mut db.lock().unwrap();
                                                let file_id = unwrappydb
                                                    .relationship_get_one_fileid(&url_id)
                                                    .unwrap();
                                                // TODO nned to check if file exists by id incase something breaks.
                                                unwrappydb.file_get_id(file_id).unwrap();

                                                info_log(&format!(
                                                    "Skipping file: {} Due to already existing in Tags Table.",
                                                    &source
                                                ));

                                                file_id.clone()
                                            }
                                        };

                                        // We've got valid fileid for reference.
                                        for taz in file.tag_list {
                                            //dbg!(&taz);
                                            let tag = taz;

                                            // Adds namespace if not exist
                                            let namespace_id = {
                                                let unwrappydb = &mut db.lock().unwrap();
                                                unwrappydb.namespace_add(
                                                    tag.namespace.name.clone(),
                                                    tag.namespace.description.clone(),
                                                    true,
                                                )
                                            };
                                            let urls = parse_tags(&mut db, tag, Some(fileid));
                                            if !urls.is_empty() {
                                                //let url_job = JobScraper {};
                                            }
                                            dbg!(&urls);
                                        }
                                    }
                                }
                                // End of err catching loop.
                                // break 'errloop;
                            }
                            break 'errloop;
                        }

                        //dbg!("End of URL Loop");
                        //let unwrappydb = &mut db.lock().unwrap();
                        //unwrappydb.transaction_flush();
                    }
                    dbg!("End of liip");
                    let unwrappydb = &mut db.lock().unwrap();
                    dbg!(&job);
                    unwrappydb.del_from_jobs_table(&"param".to_owned(), &job.original_param);
                    job_params.remove(&job);
                    unwrappydb.transaction_flush();
                }

                if job_params.is_empty() {
                    job_loop = false;
                }
            }
        });

        return Worker {
            id,
            thread: Some(thread),
            scraper,
            jobs,
        };
    }
}

///
/// Parses tags and adds the tags into the db.
///
fn parse_tags(
    db: &mut Arc<Mutex<database::Main>>,
    tag: sharedtypes::TagObject,
    file_id: Option<usize>,
) -> HashSet<String> {
    let mut url_return: HashSet<String> = HashSet::new();

    let unwrappy = &mut db.lock().unwrap();

    //dbg!(&tag);

    let namespace_id = unwrappy.namespace_add(tag.namespace.name, tag.namespace.description, true);

    match tag.tag_type {
        sharedtypes::TagType::Normal => {
            //println!("Adding tag: {} {:?}", tag.tag, &file_id);
            // We've recieved a normal tag. Will parse.
            let tag_id = unwrappy.tag_add(tag.tag, namespace_id, true, None);
            match tag.relates_to {
                None => {
                    /*let relate_ns_id = unwrappy.namespace_add(
                        relate.namespace.name.clone(),
                        relate.namespace.description,
                        true,
                    );*/
                }
                Some(relate) => {
                    let relate_ns_id = unwrappy.namespace_add(
                        relate.namespace.name.clone(),
                        relate.namespace.description,
                        true,
                    );
                    let relate_id = unwrappy.tag_add(relate.tag, relate_ns_id, true, None);
                    unwrappy.parents_add(namespace_id, tag_id, relate_ns_id, relate_id, true);
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
        sharedtypes::TagType::ParseUrl => {
            // Returns the url that we need to parse.
            url_return.insert(tag.tag);
            url_return
        }
        sharedtypes::TagType::Special => {
            // Do nothing will handle this later lol.
            url_return
        }
    }
}

/*
let thread = thread::spawn(move || {
    let liba = libloading; // in memory reference to library.

    let toparse: AHashMap<sharedtypes::CommitType, Vec<String>> = AHashMap::new();
    let mut jobvec = Vec::new();
    //let mut allurls: Vec<String> = Vec::new();
    let mut allurls: AHashMap<String, u8> = AHashMap::new();

    let _u64andduration = &scraper._ratelimit;

    // Have to lock DB from Arc & Mutex. Forces DB to lock in the meantime to avoid any data races.
    /*let mut ratelimiter = download::ratelimiter_create(
        u64andduration.0,
        u64andduration.1,
        &db.lock().unwrap(),
    );*/

    // Dedupes URL's to search for.
    // Groups all URLS into one vec to search through later.
    // Changing to vec of vec of strings. Needed for advanced job cancellation.
    let jobcnt: u32 = 0;
    let mut dnpjob: Vec<String> = Vec::new();
    for each in jblist {
        //dbg!(&each);
        let mut params: Vec<sharedtypes::ScraperParam> = Vec::new();

        let string_params = each.param.as_ref().unwrap().clone();
        let parpms: Vec<String> = string_params
            .split_whitespace()
            .map(str::to_string)
            .collect();

        let scrap_data;
        {
            let unwrappydb = &mut db.lock().unwrap();
            //let t = scrap._type;
            //println!("{}",t);
            let datafromdb = unwrappydb
                .settings_get_name(&format!("{}_{}", scrap._type, scrap._name.to_owned()))
                .unwrap()
                .param
                .clone();

            scrap_data = datafromdb.unwrap();
            // drops mutex for other threads to use.
        }

        for par in parpms {
            params.push(sharedtypes::ScraperParam {
                param_data: par,
                param_type: sharedtypes::ScraperParamType::Normal,
            });
        }

        params.push(sharedtypes::ScraperParam {
            param_data: scrap_data,
            param_type: sharedtypes::ScraperParamType::Database,
        });

        let urlload = scraper::url_dump(&liba, &params);
        let commit = each.committype.clone().unwrap();
        let mut hashtemp: AHashMap<sharedtypes::CommitType, Vec<(String, u32)>> =
            AHashMap::new();
        for eachs in urlload {
            // Checks if the hashmap contains the committype & its vec contains the data.
            match hashtemp.get_mut(&commit) {
                Some(ve) => {
                    if !allurls.contains_key(&eachs) {
                        ve.push((eachs.clone(), jobcnt));
                        allurls.insert(eachs, 0);
                    }
                }
                None => {
                    if !allurls.contains_key(&eachs) {
                        hashtemp.insert(commit, vec![(eachs.clone(), jobcnt)]);
                        allurls.insert(eachs, 0);
                    }
                }
            }
        }
        jobvec.push((hashtemp, each.param));
    }

    // This is literlly just for debugging. Keep me here.
    // May use this for the plugins system.
    for each in &toparse {
        info_log(&format!(
            "Type: {} Has {} URLS Loaded to scrape.",
            each.0,
            each.1.len()
        ));
    }

    // Ratelimit object gets created here.
    // Used accross multiple jobs that share host
    let mut ratelimit =
        download::ratelimiter_create(scrap._ratelimit.0, scrap._ratelimit.1);
    let mut ratelimit_counter = 0;
    let ratelimit_total = 10;

    let mut client = download::client_create();
    for each in jobvec {
        dbg!(&each);

        //for loo in dnpjob {

        //}

        //handle.enter();
        //let resp = insidert.spawn(async move {
        //    download::dltext_new(each.1, &mut ratelimit).await
        //});
        let mut cnt = 0;
        'mainloop: for mut eachy in each.0 {
            for urlstring in &eachy.1.clone() {
                let mut loopbool = true;
                let mut respstring = String::new();
                'mainloop: while loopbool {
                    //if dnpjob.contains(&urlstring.1.try_into().unwrap()) {continue 'mainloop;}
                    //dbg!(&dnpjob, &urlstring.1);
                    download::ratelimiter_wait(&mut ratelimit);
                    let resp = match dnpjob.is_empty() {
                        false => {
                            let urlzero = dnpjob.index(0).clone();
                            dbg!(&urlzero);
                            pause();
                            dnpjob.remove(0);
                            task::block_on(download::dltext_new(
                                urlzero.to_string(),
                                &mut ratelimit,
                                &mut client,
                                manageeplugin.clone(),
                            ))
                        }
                        true => task::block_on(download::dltext_new(
                            urlstring.0.to_string(),
                            &mut ratelimit,
                            &mut client,
                            manageeplugin.clone(),
                        )),
                    };

                    //cnt += 1;
                    match resp {
                        Ok(_) => {
                            respstring = resp.unwrap();
                            loopbool = false;
                        }
                        Err(_) => {
                            error_log(&format!(
                                "Scraper: {} GAVE ERROR: {}",
                                scrap._name,
                                &resp.err().unwrap()
                            ));
                        }
                    }
                }

                //dbg!(&resp);

                //Matches response from web request into db.

                let mut st: Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> =
                    Result::Err(sharedtypes::ScraperReturn::Nothing);

                let parserloopbool = true;

                while parserloopbool {
                    st = scraper::parser_call(&liba, &respstring);

                    match st {
                        Err(ref sterror) => match sterror {
                            sharedtypes::ScraperReturn::EMCStop(error) => {
                                error_log(error);
                            }
                            sharedtypes::ScraperReturn::Nothing => {
                                //dnpjob.push();
                                //jobcnt += 1;
                                break;
                            }
                            sharedtypes::ScraperReturn::Stop(error) => {
                                info_log(&format!("{}", error));
                                break;
                            }
                            sharedtypes::ScraperReturn::Timeout(time) => {
                                let time_dur = Duration::from_secs(*time);
                                info_log(&format!(
                                    "Sleeping: {} Secs due to ratelimit.",
                                    time
                                ));
                                info_log(&format!("ST: {:?} RESP: {}", &st, &respstring));
                                dbg!("Sleeping: {} Secs due to ratelimit.", time);

                                {
                                    let unwrappydb = &mut db.lock().unwrap();
                                    unwrappydb.transaction_flush();
                                }

                                thread::sleep(time_dur);
                                ratelimit_counter += 1;
                                if ratelimit_counter >= ratelimit_total {
                                    {
                                        let unwrappydb = &mut db.lock().unwrap();
                                        unwrappydb.transaction_flush();
                                    }

                                    ratelimit = download::ratelimiter_create(
                                        scrap._ratelimit.0,
                                        scrap._ratelimit.1,
                                    );
                                    ratelimit_counter = 0;

                                    client = download::client_create();

                                    thread::sleep(time_dur * 12);
                                }
                            }
                        },
                        Ok(_) => break,
                    }
                }

                // Only way I could find to do this somewhat cleanly :C
                if let Err(sharedtypes::ScraperReturn::Nothing) = st.as_ref() {
                    {
                        let unwrappydb = &mut db.lock().unwrap();
                        unwrappydb.transaction_flush();
                    }
                    break;
                }

                for each in st.unwrap().file {
                    //dbg!(&each);
                    //ratelimit.wait();
                    // Determine if we need to download file.
                    let mut does_url_exist = false;
                    {
                        let unwrappydb = &mut db.lock().unwrap();
                        let mut source_url_id =
                            unwrappydb.namespace_get(&"source_url".to_string()); // defaults to 0 due to unknown.
                        if source_url_id.is_none() {
                            // Namespace doesn't exist. Will create
                            unwrappydb.namespace_add(
                                "source_url".to_string(),
                                Some("Source URL for a file.".to_string()),
                                true,
                            );
                            log::info!(
                                "Adding namespace {} with an id {} due to not existing.",
                                "source_url",
                                "0"
                            );
                            source_url_id =
                                unwrappydb.namespace_get(&"source_url".to_string());
                            // defaults to 0 due to unknown.
                        }

                        let sourceurl = match each.1.source_url {
                            None => {
                                panic!("Threading: Cannot find source URL in each.1 info: {:?}", each.1);
                            }
                            Some(ref urlpassed) => urlpassed,
                        };

                        let url_tag = unwrappydb.tag_get_name(
                            sourceurl.clone(),
                            source_url_id.unwrap().clone(),
                        );
                        does_url_exist = url_tag.is_some();
                    }

                    let mut location = String::new();
                    {
                        let unwrappydb = &mut db.lock().unwrap();
                        location = unwrappydb
                            .settings_get_name(&"FilesLoc".to_string())
                            .unwrap()
                            .param
                            .as_ref()
                            .unwrap()
                            .to_owned();
                    }

                    let sourceurl = match each.1.source_url {
                        None => {
                            panic!(
                                "Threading: Cannot find source URL in each.1 info: {:?}",
                                each.1
                            );
                        }
                        Some(ref urlpassed) => urlpassed,
                    };

                    //let file = each.1;
                    //temp.push(task::block_on(download::test(url)));
                    let mut hash: String = String::new();
                    let mut file_ext: String = String::new();
                    if !does_url_exist {
                        download::ratelimiter_wait(&mut ratelimit);
                        // URL doesn't exist in DB Will download
                        info_log(&format!("Downloading: {} to: {}", &sourceurl, &location));
                        (hash, file_ext) = task::block_on(download::dlfile_new(
                            &client,
                            &each.1,
                            &location,
                            manageeplugin.clone(),
                        ));
                    } else {
                        let fileid;
                        let unwrappydb = db.lock().unwrap();
                        {
                            // File already has been downlaoded. Skipping download.
                            info_log(&format!(
                                "Skipping file: {} Due to already existing in Tags Table.",
                                &sourceurl
                            ));
                            let source_url_id =
                                unwrappydb.namespace_get(&"source_url".to_string()); // defaults to 0 due to unknown.

                            let url_tag = unwrappydb.tag_get_name(
                                sourceurl.clone(),
                                source_url_id.unwrap().clone(),
                            );

                            //NOTE: Not the best way to do it. Only allows for one source for multiple examples.
                            //let fileid =
                            //    unwrappydb.relationship_get_fileid(&url_tag.0)[0];
                            fileid =
                                unwrappydb.relationship_get_one_fileid(&url_tag.unwrap());
                        }
                        match fileid {
                            Some(fileid_use) => {
                                // We have a TAG id but not a relationship. Checking the file info.
                                //let fileinfo = unwrappydb.file_get_id(&fileid_use);
                                //panic!("{:?}", fileinfo);
                            }
                            None => {
                                info_log(&format!(
                                    "URL Tag was unexpected. downloading file."
                                ));
                                info_log(&format!(
                                    "Downloading: {} to: {}",
                                    &sourceurl, &location
                                ));

                                (hash, file_ext) = task::block_on(download::dlfile_new(
                                    &client,
                                    &each.1,
                                    &location,
                                    manageeplugin.clone(),
                                ));
                            }
                        }
                    }
                    {
                        let unwrappydb = &mut db.lock().unwrap();

                        let source_namespace_url_id = unwrappydb
                            .namespace_get(&"source_url".to_string())
                            .unwrap()
                            .to_owned();

                        // Adds file's source URL into DB
                        let file_id =
                            unwrappydb.file_add(None, &hash, &file_ext, &location, true);
                        let source_url_id = unwrappydb.tag_add(
                            sourceurl.clone(),
                            source_namespace_url_id.clone(),
                            true,
                            None,
                        );
                        unwrappydb.relationship_add(
                            file_id.to_owned(),
                            source_url_id.clone(),
                            true,
                        );

                        // Loops through all tags
                        for every in &each.1.tag_list {
                            //println!("threading every: {:?}", &every);
                            // Matches tag type. Changes depending on what type of tag (metadata)
                            match &every.1.tag_type {
                                sharedtypes::TagType::ParseUrl => {
                                    println!("Recieved Parseable tag will search it at end of loop.");

                                    //dnpjob.push(every.1.tag.to_string());
                                    //eachy.1.push((every.1.tag.to_string(), jobcnt));
                                }
                                sharedtypes::TagType::Normal => {
                                    match every.1.relates_to {
                                        None => {
                                            // Normal tag no relationships. IE Tag to file
                                            let tag_namespace_id = unwrappydb
                                                .namespace_add(
                                                    every.1.namespace.to_owned(),
                                                    None,
                                                    true,
                                                );

                                            let tag_id = unwrappydb.tag_add(
                                                every.1.tag.to_string(),
                                                tag_namespace_id,
                                                true,
                                                None,
                                            );
                                            unwrappydb.relationship_add(
                                                file_id.to_owned(),
                                                tag_id,
                                                true,
                                            );
                                        }
                                        Some(_) => {
                                            // Tag with relationship info. IE Tag to pool
                                            // Adds tag and namespace if not exist.

                                            let relate_info =
                                                every.1.relates_to.clone().unwrap();

                                            let tag_namespace_id = unwrappydb
                                                .namespace_add(
                                                    every.1.namespace.to_owned(),
                                                    None,
                                                    true,
                                                );
                                            let tag_id = unwrappydb.tag_add(
                                                every.1.tag.to_string(),
                                                tag_namespace_id,
                                                true,
                                                None,
                                            );

                                            let relate_namespace_id = unwrappydb
                                                .namespace_add(relate_info.0, None, true);
                                            let relate_tag_id = unwrappydb.tag_add(
                                                every.1.tag.to_string(),
                                                relate_namespace_id,
                                                true,
                                                None,
                                            );

                                            unwrappydb.parents_add(
                                                tag_namespace_id,
                                                tag_id.clone(),
                                                relate_namespace_id,
                                                relate_tag_id.clone(),
                                                true,
                                            );
                                        }
                                    }
                                }
                                sharedtypes::TagType::Special => {
                                    dbg!(&every);
                                }
                            }
                        }
                    }
                }
                //let st = scraper::parser_call(&liba, &beans);
                //dbg!(&st);
                //dbg!(rt.block_on(resps));
                //break;
            }

            info_log(&format!("Looped: {}", &each.1.as_ref().unwrap()));
            dbg!("Looped: {}", &each.1.as_ref().unwrap());
            dbg!(&each.1.as_ref().unwrap());
            let unwrappydb = &mut db.lock().unwrap();
            unwrappydb.del_from_jobs_table(&"param".to_string(), &each.1.as_ref().unwrap());
            unwrappydb.transaction_flush();
        }
    }
    dbg!("SPAWNED2");
});
//dbg!(&id, &thread, &scraper, &jobs );
Worker {
    id,
    thread: Some(thread),
    scraper,
    jobs,
}*/
