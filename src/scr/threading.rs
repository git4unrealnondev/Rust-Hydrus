use crate::database;
use crate::download;
use crate::jobs::Jobs;
use crate::jobs::JobsRef;
use crate::logging::error_log;
use crate::plugins::PluginManager;
use crate::scraper;
use crate::scraper::ScraperManager;
use crate::sharedtypes;

use ahash::AHashMap;
use async_std::task;
use file_format::{FileFormat, Kind};
use futures;
use futures::future::join_all;
use log::{error, info};
use std::borrow::Borrow;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::{collections::HashMap, hash::BuildHasherDefault};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::runtime::Handle;
use tokio::runtime::Runtime;
use tokio::time::Timeout;
use url::Url;

pub struct threads {
    _workers: Vec<Worker>,
    _runtime: Runtime,
}

///
/// Holder for workers.
/// Workers manage their own threads.
///
impl threads {
    pub fn new() -> Self {
        let mut workers = Vec::new();

        let mut rt = Runtime::new().unwrap();

        threads {
            _workers: workers,
            _runtime: rt,
        }
    }

    ///
    /// Creates a pool of threads. need to add checking.
    ///
    pub fn creates_thread_pool(&mut self, size: usize) {
        for id in 0..size {
            //self._workers.push(Worker::new(id), );
        }
    }

    ///
    /// Adds a worker to the threadvec.
    ///
    pub fn startwork(
        &mut self,
        scraper: scraper::InternalScraper,
        jobs: Vec<JobsRef>,
        db: &mut Arc<Mutex<database::Main>>,
        scrapermanager: libloading::Library,
        pluginmanager: Arc<Mutex<PluginManager>>,
    ) {
        let worker = Worker::new(
            self._workers.len(),
            scraper,
            jobs,
            &mut self._runtime,
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
    jobs: Vec<JobsRef>,
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
            info!("Shutting Down Worker from Worker: {}", self.id);
            println!("Shutting Down Worker from Worker: {}", self.id);
            futures::executor::block_on(async { thread.join().unwrap() });
        }
    }
}

impl Worker {
    fn new(
        id: usize,
        scraper: scraper::InternalScraper,
        jobs: Vec<JobsRef>,
        rt: &mut Runtime,
        dba: &mut Arc<Mutex<database::Main>>,
        libloading: libloading::Library,
        pluginmanager: Arc<Mutex<PluginManager>>,
    ) -> Worker {
        info!(
            "Creating Worker for id: {} Scraper Name: {} With a jobs length of: {}",
            &id,
            &scraper._name,
            &jobs.len()
        );
        let mut db = dba.clone();
        let jblist = jobs.clone();
        let scrap = scraper.clone();
        //
        let rta = rt;

        //let handle = rt.handle().clone();
        //let handle = Handle::current();
        let insidert = Runtime::new().unwrap();

        // Download code goes inside of thread spawn.
        // All urs that Have been pushed into checking.
        // NOTE: If a url is already present for scraping then subsequent URL's will be ignored.

        let manageeplugin = pluginmanager;

        let thread = thread::spawn(move || {
            let liba = libloading; // in memory reference to library.

            let mut toparse: AHashMap<sharedtypes::CommitType, Vec<String>> = AHashMap::new();
            let mut jobvec = Vec::new();
            //let mut allurls: Vec<String> = Vec::new();
            let mut allurls: AHashMap<String, u8> = AHashMap::new();

            let u64andduration = &scraper._ratelimit;

            // Have to lock DB from Arc & Mutex. Forces DB to lock in the meantime to avoid any data races.
            /*let mut ratelimiter = download::ratelimiter_create(
                u64andduration.0,
                u64andduration.1,
                &db.lock().unwrap(),
            );*/

            let mut scrap_data = String::new();
            {
                let mut unwrappydb = &mut db.lock().unwrap();
                //let t = scrap._type;
                //println!("{}",t);
                let datafromdb = unwrappydb
                    .settings_get_name(&format!("{}_{}", scrap._type, scrap._name.to_owned()))
                    .unwrap()
                    .param;

                scrap_data = datafromdb.unwrap();
                // drops mutex for other threads to use.
            }

            // Dedupes URL's to search for.
            // Groups all URLS into one vec to search through later.
            // Changing to vec of vec of strings. Needed for advanced job cancellation.
            for each in jblist {
                //dbg!(&each);

                let mut parpms = each._params.clone();
                parpms.push(scrap_data.clone());

                let urlload = scraper::url_dump(&liba, parpms);
                let commit = &each._committype;
                let mut hashtemp: AHashMap<sharedtypes::CommitType, Vec<String>> = AHashMap::new();
                for eachs in urlload {
                    // Checks if the hashmap contains the committype & its vec contains the data.
                    match hashtemp.get_mut(&commit) {
                        Some(ve) => {
                            if !allurls.contains_key(&eachs) {
                                ve.push(eachs.clone());
                                allurls.insert(eachs, 0);
                            }
                        }
                        None => {
                            if !allurls.contains_key(&eachs) {
                                hashtemp.insert(commit.clone(), vec![eachs.clone()]);
                                allurls.insert(eachs, 0);
                            }
                        }
                    }
                }
                jobvec.push((hashtemp, each._params.clone()))
            }

            // This is literlly just for debugging. Keep me here.
            // May use this for the plugins system.
            for each in &toparse {
                info!(
                    "Type: {} Has {} URLS Loaded to scrape.",
                    each.0,
                    each.1.len()
                );
            }

            // Ratelimit object gets created here.
            // Used accross multiple jobs that share host
            let mut ratelimit =
                download::ratelimiter_create(scrap._ratelimit.0, scrap._ratelimit.1);
            let mut ratelimit_counter = 0;
            let mut ratelimit_total = 10;

            let mut client = download::client_create();
            for each in jobvec {
                //handle.enter();
                //let resp = insidert.spawn(async move {
                //    download::dltext_new(each.1, &mut ratelimit).await
                //});
                for eachy in each.0 {
                    for urlstring in eachy.1 {
                        let mut loopbool = true;
                        let mut respstring = String::new();
                        while loopbool {
                            download::ratelimiter_wait(&mut ratelimit);
                            let resp = task::block_on(download::dltext_new(
                                urlstring.to_string(),
                                &mut ratelimit,
                                &mut client,
                                manageeplugin.clone(),
                            ));
                            match resp {
                                Ok(_) => {
                                    respstring = resp.unwrap();
                                    loopbool = false;
                                }
                                Err(_) => {
                                    error!(
                                        "Scraper: {} GAVE ERROR: {}",
                                        scrap._name,
                                        &resp.err().unwrap()
                                    );
                                }
                            }
                        }

                        //dbg!(&resp);

                        //Matches response from web request into db.

                        let mut st: Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> =
                            Result::Err(sharedtypes::ScraperReturn::Nothing);

                        let mut parserloopbool = true;

                        while parserloopbool {
                            st = scraper::parser_call(&liba, &respstring);

                            match st {
                                Err(ref sterror) => match sterror {
                                    sharedtypes::ScraperReturn::EMCStop(error) => {
                                        error_log(error);
                                    }
                                    sharedtypes::ScraperReturn::Nothing => break,
                                    sharedtypes::ScraperReturn::Stop(error) => {
                                        error!("{}", error);
                                        break;
                                    }
                                    sharedtypes::ScraperReturn::Timeout(time) => {
                                        let time_dur = Duration::from_secs(*time);
                                        info!("Sleeping: {} Secs due to ratelimit.", time);
                                        info!("ST: {:?} RESP: {}", &st, &respstring);
                                        dbg!("Sleeping: {} Secs due to ratelimit.", time);

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
                                        &"source_url".to_string(),
                                        &"Source URL for a file.".to_string(),
                                        true,
                                    );
                                    log::info!(
                                        "Adding namespace {} with an id {} due to not existing.",
                                        "source_url",
                                        "0"
                                    );
                                    source_url_id =
                                    unwrappydb.namespace_get(&"source_url".to_string()); // defaults to 0 due to unknown.
                                }
                                let url_tag = unwrappydb.tag_get_name(
                                    each.1.source_url.to_string(),
                                    source_url_id.unwrap(),
                                );
                                does_url_exist = url_tag.is_some();
                            }

                            let mut location = String::new();
                            {
                                let unwrappydb = &mut db.lock().unwrap();
                                location = unwrappydb
                                    .settings_get_name(&"FilesLoc".to_string())
                                    .unwrap()
                                    .param.unwrap();
                            }

                            //let file = each.1;
                            //temp.push(task::block_on(download::test(url)));
                            let mut hash: String = String::new();
                            let mut file_ext: String = String::new();
                            if !does_url_exist {
                                download::ratelimiter_wait(&mut ratelimit);
                                // URL doesn't exist in DB Will download
                                info!("Downloading: {} to: {}", &each.1.source_url, &location);
                                (hash, file_ext) = task::block_on(download::dlfile_new(
                                    &client,
                                    &each.1,
                                    &location,
                                    manageeplugin.clone(),
                                    db.clone(),
                                ));
                            } else {
                                let unwrappydb = &mut db.lock().unwrap();

                                // File already has been downlaoded. Skipping download.
                                info!(
                                    "Skipping file: {} Due to already existing in Tags Table.",
                                    &each.1.source_url
                                );
                                let source_url_id =
                                    unwrappydb.namespace_get(&"source_url".to_string()); // defaults to 0 due to unknown.

                                let url_tag = unwrappydb.tag_get_name(
                                    each.1.source_url.to_string(),
                                    source_url_id.unwrap(),
                                );

                                //NOTE: Not the best way to do it. Only allows for one source for multiple examples.
                                //let fileid =
                                //    unwrappydb.relationship_get_fileid(&url_tag.0)[0];
                                let fileid =
                                    unwrappydb.relationship_get_one_fileid(&url_tag.unwrap());
                                match fileid {
                                    Some(_) => {}
                                    None => {
                                        panic!(
                                            "url has info but no file data. {}",
                                            &url_tag.unwrap()
                                        );
                                    }
                                }
                                let fileinfo = unwrappydb.file_get_id(&fileid.unwrap());
                                dbg!(&fileinfo);
                                match fileinfo {
                                    None => {
                                        error!("ERROR: File: {} has url but no file info in db table Files. PANICING", &url_tag.unwrap());
                                        panic!("ERROR: File: {} has url but no file info in db table Files. PANICING", &url_tag.unwrap());
                                    }
                                    Some(_) => {
                                        hash = fileinfo.as_ref().unwrap().0.to_string();
                                        file_ext = fileinfo.as_ref().unwrap().1.to_string();
                                    }
                                }
                            }
                            {
                                let unwrappydb = &mut db.lock().unwrap();

                                let source_namespace_url_id =
                                    unwrappydb.namespace_get(&"source_url".to_string()).unwrap();

                                // Adds file's source URL into DB
                                let file_id = unwrappydb.file_add(
                                    None,
                                    hash.to_string(),
                                    file_ext.to_string(),
                                    location.to_string(),
                                    true,
                                );
                                let source_url_id = unwrappydb.tag_add(
                                    each.1.source_url.to_string(),
                                    "".to_string(),
                                    source_namespace_url_id,
                                    true,
                                    None,
                                );
                                unwrappydb.relationship_add(file_id, source_url_id, true);

                                // Loops through all tags
                                for every in &each.1.tag_list {
                                    // Matches tag type. Changes depending on what type of tag (metadata)
                                    match &every.1.tag_type {
                                        sharedtypes::TagType::Normal => {
                                            match every.1.relates_to {
                                                None => {
                                                    // Normal tag no relationships. IE Tag to file
                                                    let tag_namespace_id = unwrappydb
                                                        .namespace_add(
                                                            &every.1.namespace,
                                                            &"".to_string(),
                                                            true,
                                                        );
                                                    let tag_id = unwrappydb.tag_add(
                                                        every.1.tag.to_string(),
                                                        "".to_string(),
                                                        tag_namespace_id,
                                                        true,
                                                        None,
                                                    );
                                                    
                                                    dbg!(&tag_namespace_id, &tag_id);
                                                    
                                                    unwrappydb
                                                        .relationship_add(file_id, tag_id, true);
                                                }
                                                Some(_) => {
                                                    // Tag with relationship info. IE Tag to pool
                                                    // Adds tag and namespace if not exist.

                                                    let relate_info =
                                                        every.1.relates_to.clone().unwrap();

                                                    let tag_namespace_id = unwrappydb
                                                        .namespace_add(
                                                            &every.1.namespace,
                                                            &"".to_string(),
                                                            true,
                                                        );
                                                    let tag_id = unwrappydb.tag_add(
                                                        every.1.tag.to_string(),
                                                        "".to_string(),
                                                        tag_namespace_id,
                                                        true,
                                                        None,
                                                    );

                                                    let relate_namespace_id = unwrappydb
                                                        .namespace_add(
                                                            &relate_info.0,
                                                            &"".to_string(),
                                                            true,
                                                        );
                                                    let relate_tag_id = unwrappydb.tag_add(
                                                        every.1.tag.to_string(),
                                                        "".to_string(),
                                                        relate_namespace_id,
                                                        true,
                                                        None,
                                                    );

                                                    unwrappydb.parents_add(
                                                        tag_namespace_id,
                                                        tag_id,
                                                        relate_namespace_id,
                                                        relate_tag_id,
                                                        true,
                                                    );
                                                }
                                            }
                                        }
                                        sharedtypes::TagType::Special => {}
                                    }
                                }
                            }
                        }
                        //let st = scraper::parser_call(&liba, &beans);
                        //dbg!(&st);
                        //dbg!(rt.block_on(resps));
                        //break;
                    }

                    let mut stringofvec = String::new();
                    let last2 = &each.1[each.1.len() - 1];
                    for item in each.1.iter() {
                        if item == last2 {
                            stringofvec += item;
                            break;
                        }
                        stringofvec += item;
                        stringofvec += " ";
                    }
                    info!("Looped: {}", &stringofvec);
                    dbg!("Looped: {}", &stringofvec);
                    dbg!(&stringofvec);
                    let unwrappydb = &mut db.lock().unwrap();
                    unwrappydb.del_from_jobs_table(&"param".to_string(), &stringofvec);
                    unwrappydb.transaction_flush();
                }

                //dbg!(resps)
            }

            //let dur = Duration::from_millis(1);
            //thread::sleep(dur);
            //for each in resps {
            //    let st = scraper::parser_call(&liba, &each.text().await.unwrap().to_string());
            //dbg!(st);
            //}

            //dbg!(toparse);
            //dbg!(ratelimiter);

            //thread::sleep(Duration::from_millis(10000));
            dbg!("SPAWNED2");
        });
        //dbg!(&id, &thread, &scraper, &jobs );
        Worker {
            id,
            thread: Some(thread),
            scraper,
            jobs,
        }
    }
}

/* let thread = rt.spawn( async {

        let mut toparse: AHashMap<CommitType, Vec<String>> = AHashMap::new();
        let mut allurls: Vec<String> = Vec::new();

        let u64andduration = &scraper._ratelimit;

        // Have to lock DB from Arc & Mutex. Forces DB to lock in the meantime to avoid any data races.
        /*let mut ratelimiter = download::ratelimiter_create(
            u64andduration.0,
            u64andduration.1,
            &db.lock().unwrap(),
        );*/

        let unwrappydb = &mut db.lock().unwrap();
        //let t = scrap._type;
        //println!("{}",t);
        let datafromdb = unwrappydb.settings_get_name(&format!("{}_{}",scrap._type.to_string(), scrap._name.to_owned())).unwrap().1;

        // Dedupes URL's to search for.
        // Groups all URLS into one vec to search through later.
        for each in jblist {
            dbg!(&each);



            let mut parpms = each._params ;
            parpms.push(datafromdb.clone());

            let urlload = scraper::url_dump(&liba, parpms);
            let commit = each._committype;
            for eachs in urlload {
                // Checks if the hashmap contains the committype & its vec contains the data.
                match toparse.get_mut(&commit) {
                    Some(ve) => {
                        if !allurls.contains(&eachs) {
                            ve.push(eachs.clone());
                            allurls.push(eachs);
                        }
                    }
                    None => {
                        if !allurls.contains(&eachs) {
                            dbg!(&eachs);
                            toparse.insert(commit.clone(), vec![eachs.clone()]);
                            allurls.push(eachs);
                        }
                    }
                }
            }
        }

        // This is literlly just for debugging. Keep me here.
        // May use this for the plugins system.
        for each in &toparse {
            info!(
                "Type: {} Has {} URLS Loaded to scrape.",
                each.0,
                each.1.len()
            );
        }
        //dbg!(toparse);
        //let onesearch = allurls[0].to_string();
        for each in toparse {
            let resps = "";
                //rt.block_on(download::dltext_new(scrap._ratelimit, each.1, &liba));
            dbg!(resps);
            break

            //dbg!(resps)
        }
        //for each in resps {
        //    let st = scraper::parser_call(&liba, &each.text().await.unwrap().to_string());
        //dbg!(st);
        //}

        //dbg!(toparse);
        //dbg!(ratelimiter);

        //thread::sleep(Duration::from_millis(10000));
        dbg!("SPAWNED2");
});*/
