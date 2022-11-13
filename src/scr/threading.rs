use super::database;
use super::jobs::JobsRef;
use crate::scr::download;
use crate::scr::scraper;
use futures;
use log::{error, info};
use std::borrow::Borrow;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

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
    ) {
        self._workers.push(Worker::new(
            self._workers.len(),
            scraper,
            jobs,
            &mut self._runtime,
            db,
        ));
    }
}
///
/// Worker holder for data. Will add a scraper processor soon tm.
///
struct Worker {
    id: usize,
    scraper: scraper::InternalScraper,
    jobs: Vec<JobsRef>,
    thread: Option<tokio::task::JoinHandle<()>>,
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
            futures::executor::block_on(async { thread.await.unwrap() });
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
    ) -> Worker {
        info!(
            "Creating Worker for id: {} Scraper Name: {} With a jobs length of: {}",
            &id,
            &scraper._name,
            &jobs.len()
        );
        let db = dba.clone();

        // Download code goes inside of thread spawn.
        let thread = rt.spawn(async move {
            dbg!("SPAWNED");

            let u64andduration = &scraper._ratelimit;

            // Have to lock DB from Arc & Mutex. Forces DB to lock in the meantime to avoid any data races.
            let ratelimiter = download::ratelimiter_create(
                u64andduration.0,
                u64andduration.1,
                &db.lock().unwrap(),
            );

            dbg!(ratelimiter);

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
