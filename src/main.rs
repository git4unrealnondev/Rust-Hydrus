#![recursion_limit = "36000"]
#![allow(dead_code)]

use log::{error, warn};
//use tokio::sync::{RwLock, Mutex};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::HashSet,
    io,
    sync::Arc,
    thread::{self, Thread},
    time::{self, Duration},
};
use tokio::time::Interval;

pub const VERS: u64 = 12;
pub const DEFAULT_LOC_NAME: &str = "main.db";
pub const DEFAULT_LOC_LOGNAME: &str = "log.txt";
pub const DEFAULT_LOC_PLUGIN: &str = "./target/release";
pub const DEFAULT_LOC_SCRAPER: &str = "packages";
extern crate ratelimit;

pub mod cli;
pub mod database;
pub use database::*; //
pub mod download;
pub mod file;
pub mod globalload;
pub mod jobs;
pub mod logging;
//#[path = "./scr/plugins.rs"]
//pub mod plugins;
pub mod reimport;
//#[path = "./scr/scraper.rs"]
//pub mod scraper;
pub mod downloadlogic;
pub mod tasks;
//pub mod threading;
pub mod time_func;

// Needed for the plugin coms system.
//#[path = "./scr/bypasses.rs"]
//pub mod bypasses;
pub mod client;
pub mod helpers;
pub mod os;
pub mod server;
pub mod types;
pub mod ui;

use database::database::Main;

use crate::{
    downloadlogic::DownloadManager, helpers::memory_manage, server::PluginIpcInteract, ui::ui::App,
};

/// This code is trash. lmao. Has threading and plugins soon tm Will probably work
/// :D
fn pause() {
    use std::io::{Read, Write, stdin, stdout};

    let mut stdout = stdout();
    stdout.write_all(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read_exact(&mut [0]).unwrap();
}

/// Creates DB from database.rs allows function calls.
fn makedb(dbloc: &str) -> Main {
    // Setting up DB VARS
    let path = dbloc.to_string();

    // let dbexist = Path::new(&path).exists(); dbcon is database connector let mut
    // dbcon = scr::database::dbinit(&path);
    Main::new(Some(path), VERS)
    // let dbcon = database.load_mem(&mut data._conn);
}

/// Gets file setup out of main. Checks if null data was written to database.
fn db_file_sanity(dbloc: &str) {
    let _dbzero = file::size_eq(dbloc.to_string(), 0);
    match _dbzero {
        Ok(_dbzero) => {
            println!("File is zero: {} will remove and create.", dbloc);
            warn!("File is zero: {} will remove and create.", dbloc);
            let _fileret = file::remove_file(dbloc.to_string());
            match _fileret {
                Err(_fileret) => {
                    error!("ERROR CANT DELETE FILE!!! CLOSING RAPIDLY.");
                    panic!("ERROR CANT DELETE FILE!!! CLOSING RAPIDLY.");
                }
                Ok(_fileret) => _fileret,
            }
        }
        Err(_dbzero) => {}
    }
}

/// Main function.
#[tokio::main]
async fn main() -> io::Result<()> {
    //console_subscriber::init();
    memory_manage();

    #[cfg(debug_assertions)]
    thread::spawn(move || {
        loop {
            use parking_lot::deadlock;

            thread::sleep(Duration::from_secs(1));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }

            logging::info_log("=== 🚨 DEADLOCK DETECTED 🚨 ===");
            for (i, threads) in deadlocks.iter().enumerate() {
                logging::info_log(format!("Deadlock #{}:", i));
                for t in threads {
                    logging::info_log(format!("  Thread ID: {:?}", t.thread_id()));
                    logging::info_log(format!("  Backtrace:\n{:?}", t.backtrace()));
                }
            }
        }
    });

    {
        // Makes Logging work
        logging::main(&DEFAULT_LOC_LOGNAME.to_string());
    }
    os::check_os_compatibility();

    // Inits Database.
    let mut database = makedb(DEFAULT_LOC_NAME);

    let jobmanager = jobs::Jobs::new(database.clone());

    // Needed to do big processing like thumbnails and stuff
    let heavy_processing_pool = Arc::new(rayon::ThreadPoolBuilder::new().build().unwrap());

    let mut globalload = globalload::GlobalLoad::new(database.clone(), jobmanager.clone());
    {
        database.load_table(&sharedtypes::LoadDBTable::Settings);
        database.load_table(&sharedtypes::LoadDBTable::Jobs);

        //let mut globalload_data =
        //    plugins::globalload_data::new(plugin_loc.to_string(), database.clone(), jobmanager.clone());

        // Adds plugin and scraper callback capability from inside the db
        // Things like callbacks and the like
        database.setup_globalload(globalload.clone());

        let mut ipc_interact = PluginIpcInteract::new(
            database.clone(),
            globalload.clone(),
            jobmanager.clone(),
            heavy_processing_pool.clone(),
        );

        // Spawn the entire asynchronous router/listener pool in the background
        // without locking up execution flow or using OS thread hacks
        let database_for_ipc = database.clone();
        tokio::spawn(async move {
            if let Err(e) = ipc_interact.spawn_listener(database_for_ipc).await {
                eprintln!("Critical breakdown in IPC listener framework loop: {}", e);
            }
        });

        // Putting this down here after plugin manager because that's when the IPC server
        // starts and we can then inside of the scraper start calling IPC functions

        let mut upgradeversvec = Vec::new();

        // Upgrades the DB by geting version differences.
        // Waits for the entire DB to be upgraded before running scraper upgrades.
        'upgradeloop: loop {
            let repeat;
            {
                repeat = database.check_version();
            }
            if !repeat {
                upgradeversvec.push(database.db_vers_get());
            } else {
                break 'upgradeloop;
            }
        }

        // Actually upgrades the DB from scraper calls
        for db_version in upgradeversvec.iter() {
            globalload.run_upgrade_logic(db_version);
        }

        // Processes any CLI input here
        //cli::main(database.clone(), globalload);
        cli::main(database.clone());
    }

    let mut terminal = ratatui::init();

    let (uisender, uireciever) = tokio::sync::mpsc::unbounded_channel();

    let mut app = App::new(uireciever);

    {
        globalload.reload_regex();
        // Calls the on_start func for the plugins
        globalload.pluginscraper_on_start();
    }

    // A way to get around a mutex lock but it works lol
    let one_sec = time::Duration::from_millis(1000);
    loop {
        if !globalload.plugin_on_start_should_wait() {
            break;
        } else {
            std::thread::sleep(one_sec);
        }
    }

    {
        let sites = globalload.return_all_sites().clone();
        // Checks if we need to load any jobs
        let jm = jobmanager.clone();
        let _ = tokio::task::spawn_blocking(move || {
            jm.jobs_load(sites);
        })
        .await;
    }

    // One flush after all the on_start unless needed
    //

    // Creates a threadhandler that manages callable threads.
    //let mut threadhandler = threading::Threads::new(Arc::new(uisender.clone()));
    let tokio_handle = tokio::runtime::Handle::current();
    let mut threadhandler = Arc::new(Mutex::new(DownloadManager::new(
        uisender.into(),
        database,
        globalload.clone(),
        jobmanager.clone(),
        tokio_handle,
        heavy_processing_pool,
    )));

    //let mut conn = database.read().get_database_connection();
    //let tn = conn.transaction().unwrap();
    // just determines if we have any loaded jobs
    jobmanager.jobs_run_new();

    {
        for scraper in jobmanager.job_scrapers_get() {
            threadhandler.lock().add_work(scraper.clone());
        }
    }

    let jobmanager_loop = jobmanager.clone();
    let threadhandler_loop = threadhandler.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            interval.tick().await;

            // 1. Get the scrapers, making sure we extract OWNED values, not references
            let scrapers: Vec<_> = {
                // Ensure .clone() here creates owned Scraper items, not references to them
                jobmanager_loop
                    .job_scrapers_get()
                    .iter()
                    .map(|scraper| (*scraper).clone()) // Explicitly clone the underlying value
                    .collect()
            };

            // 2. Loop through and spawn each job concurrently
            for scraper in scrapers {
                let handler = threadhandler_loop.clone();

                // 1. Lock the manager just long enough to register the new worker state
                let maybe_worker = {
                    let mut handler_guard = handler.lock();
                    handler_guard.add_work(scraper.clone())
                }; // The guard drops right here! The manager is free for other threads.

                // 2. If it's a new scraper, spin up its long-running execution in parallel
                if let Some(worker) = maybe_worker {
                    let scraper_arc = Arc::new(worker);
                    scraper_arc.start_scraper().await;
                }
            }
        }
    });

    let app_result = app.run(&mut terminal).await;

    //thread_handle.join().unwrap();
    ratatui::restore();
    app_result
    /*

            // Anything below here will run automagically. Jobs run in OS threads Waits until
            // all threads have closed.


            // This wait is done for allowing any thread to "complete" Shouldn't be nessisary
            // but hey. :D
            //database.transaction_flush(tn);

            let mills_fifty = time::Duration::from_millis(50);
            std::thread::sleep(mills_fifty);
            logging::info_log("UNLOADING".to_string());
            log::logger().flush();

        Ok(())
    */
}
