use super::database;
use super::jobs::Jobs;
use super::jobs::JobsRef;
use super::scraper::ScraperManager;
use crate::scr::download;
use crate::scr::scraper;
use crate::scr::sharedtypes;
use crate::scr::sharedtypes::CommitType;
use ahash::AHashMap;
use futures;
use std::thread;
use log::{error, info};
use std::borrow::Borrow;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::runtime::Runtime;
use url::Url;
use tokio::runtime::Handle;

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
        let db = dba.clone();
        let jblist = jobs.clone();
        let scrap = scraper.clone();
        //
        let rta = rt;
        
        
        //let handle = rt.handle().clone();
        //let handle = Handle::current();
let insidert  =Runtime::new().unwrap();
        // Download code goes inside of thread spawn.
        // All urs that Have been pushed into checking.
        // NOTE: If a url is already present for scraping then subsequent URL's will be ignored.
        
        let thread = thread::spawn( move || 
            {
                let liba = libloading;
                
                
                
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
                //dbg!(&each);
                
                
                
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
                //handle.enter();
                let resps = 
                    insidert.spawn(async move {download::dltext_new(scrap._ratelimit, each.1, &liba).await});
                //resps.poll();
                let beans = futures::executor::block_on(resps);
                dbg!(beans);
                //dbg!(rt.block_on(resps));
                break

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