use super::database;
use super::jobs::Jobs;
use super::jobs::JobsRef;
use super::scraper::ScraperManager;
use crate::scr::download;
use crate::scr::scraper;
use crate::scr::sharedtypes;
use crate::scr::sharedtypes::CommitType;
use ahash::AHashMap;
use async_std::task;
use futures;
use futures::future::join_all;
use log::{error, info};
use std::borrow::Borrow;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::runtime::Handle;
use tokio::runtime::Runtime;
use file_format::{FileFormat, Kind};
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
    ) {
        let worker = Worker::new(
            self._workers.len(),
            scraper,
            jobs,
            &mut self._runtime,
            db,
            scrapermanager,
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

        let thread = thread::spawn(move || {
            let liba = libloading; // in memory reference to library.

            let mut toparse: AHashMap<CommitType, Vec<String>> = AHashMap::new();
            let mut allurls: Vec<String> = Vec::new();

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
                    .settings_get_name(&format!(
                        "{}_{}",
                        scrap._type.to_string(),
                        scrap._name.to_owned()
                    ))
                    .unwrap()
                    .1;

                scrap_data = datafromdb;
                // drops mutex for other threads to use.
            }

            // Dedupes URL's to search for.
            // Groups all URLS into one vec to search through later.
            for each in jblist {
                //dbg!(&each);

                let mut parpms = each._params;
                parpms.push(scrap_data.clone());

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

            //dbg!(&toparse);
            //let onesearch = allurls[0].to_string();

            // Ratelimit object gets created here.
            // Used accross multiple jobs that share host
            let mut ratelimit =
                download::ratelimiter_create(scrap._ratelimit.0, scrap._ratelimit.1);

            let mut client = download::client_create();

            for each in toparse {
                //handle.enter();
                //let resp = insidert.spawn(async move {
                //    download::dltext_new(each.1, &mut ratelimit).await
                //});

                for urlstring in each.1 {
                    ratelimit.wait();
                    let resp = task::block_on(download::dltext_new(
                        urlstring,
                        &mut ratelimit,
                        &mut client,
                    ));
                    //dbg!(&resp);

                    //resps.poll();
                    //let beans = resp.unwrap();
                    match resp {
                        Ok(_) => {
                            let st = scraper::parser_call(&liba, &resp.unwrap());
                            //let mut temp = Vec::new();
                            for each in st.unwrap().file {
                                //ratelimit.wait();
                                // Determine if we need to download file.
                                let mut does_url_exist = false;
                                {
                                    let unwrappydb = &mut db.lock().unwrap();
                                    let source_url_id =
                                        unwrappydb.namespace_get(&"source_url".to_string()); // defaults to 0 due to unknown.
                                    if !source_url_id.1 {
                                        // Namespace doesn't exist. Will create
                                        unwrappydb.namespace_add(
                                            &"source_url".to_string(),
                                            &"Source URL for a file.".to_string(),
                                            true,
                                        );
                                        log::info!("Adding namespace {} with an id {} due to not existing.", "source_url", "0");
                                    }
                                    let url_tag = unwrappydb.tag_get_name(
                                        each.1.source_url.to_string(),
                                        source_url_id.0,
                                    );
                                    does_url_exist = url_tag.1;
                                }
                                if !does_url_exist {
                                    let mut location = String::new();
                                    {
                                        let unwrappydb = &mut db.lock().unwrap();
                                        location = unwrappydb.settings_get_name(&"FilesLoc".to_string()).unwrap().1;
                                    }
                                    ratelimit.wait();
                                    //let file = each.1;
                                    //temp.push(task::block_on(download::test(url)));
                                    let (hash, file_ext) = task::block_on(download::dlfile_new(&client, &each.1, &location));
                                    {
                                    let unwrappydb = &mut db.lock().unwrap();
                                    
                                    let source_namespace_url_id =
                                        unwrappydb.namespace_get(&"source_url".to_string()).0;
                                        
                                    // Adds file's source URL into DB
                                    let file_id = unwrappydb.file_add(hash.to_string(), file_ext.to_string(), location.to_string(), true);
                                    let source_url_id = unwrappydb.tag_add(each.1.source_url.to_string(), "".to_string(), source_namespace_url_id, true);
                                      unwrappydb.relationship_add(file_id, source_url_id, true);
                                    
                                    // Loops through all tags
                                    for every in &each.1.tag_list {
                                    // Matches tag type. Changes depending on what type of tag (metadata)
                                    match &every.1.tag_type {
                                        sharedtypes::TagType::Normal => match every.1.relates_to {
                                            None => {
                                                // Normal tag no relationships. IE Tag to file
                                                let tag_namespace_id = unwrappydb.namespace_add(&every.1.namespace, &"".to_string(), true);
                                                let tag_id = unwrappydb.tag_add(every.1.tag.to_string(), "".to_string(), tag_namespace_id, true);
                                                unwrappydb.relationship_add(file_id, tag_id, true);
                                            }
                                            Some(_) => {
                                                // Tag with relationship info. IE Tag to pool
                                                //dbg!("Relationship", &every.1);
                                                
                                                let tag_namespace_id = unwrappydb.namespace_add(&every.1.namespace, &"".to_string(), true);
                                                let tag_id = unwrappydb.tag_add(every.1.tag.to_string(), "".to_string(), tag_namespace_id, true);
                                                
                                                
                                            }
                                        },
                                        sharedtypes::TagType::Hash(_) => {
                                            // dbg!("Hash", &every.1);
                                        }
                                        sharedtypes::TagType::Special => {}
                                    }
                                        
                                    }
                                    

                                    //println!("{:?}", every);
                                }
                                    
                                    
                                }
                                
                                /*for every in &each.1.tag_list {
                                    // Matches tag type. Changes depending on what type of tag (metadata)
                                    match &every.1.tag_type {
                                        sharedtypes::TagType::Normal => match every.1.relates_to {
                                            None => {
                                                // Normal tag no relationships. IE Tag to file
                                                // dbg!("Tag", &every.1);
                                            }
                                            Some(_) => {
                                                // Tag with relationship info. IE Tag to pool
                                                //dbg!("Relationship", &every.1);
                                            }
                                        },
                                        sharedtypes::TagType::Hash(_) => {
                                            // dbg!("Hash", &every.1);
                                        }
                                        sharedtypes::TagType::Special => {}
                                    }

                                    //println!("{:?}", every);
                                }*/
                                //println!("");
                            }
                            
                        }
                        Err(_) => {}
                    }
                    //let st = scraper::parser_call(&liba, &beans);
                    //dbg!(&st);
                    //dbg!(rt.block_on(resps));
                    //break;
                }
                //dbg!(resps)
            }
            
            
            let unwrappydb = &mut db.lock().unwrap();
            unwrappydb.transaction_flush();
            
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
