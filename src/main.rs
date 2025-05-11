// use crate::scr::sharedtypes::jobs_add; use
// crate::scr::sharedtypes::AllFields::EJobsAdd; use crate::scr::tasks;
use log::{error, warn};
//use scraper::db_upgrade_call;
//use scraper::on_start;
use std::sync::Arc;
use std::sync::Mutex;

// use std::sync::{Arc, Mutex};
use std::{thread, time};

pub const VERS: usize = 6;
pub const DEFAULT_LOC_PLUGIN: &str = "plugins";
pub const DEFAULT_LOC_SCRAPER: &str = "scrapers";

extern crate ratelimit;

#[path = "./scr/cli.rs"]
mod cli;
#[path = "./scr/db/database.rs"]
mod database;
#[path = "./scr/download.rs"]
mod download;
#[path = "./scr/file.rs"]
mod file;
#[path = "./scr/globalload.rs"]
mod globalload;
#[path = "./scr/jobs.rs"]
mod jobs;
#[path = "./scr/logging.rs"]
mod logging;
//#[path = "./scr/plugins.rs"]
//mod plugins;
#[path = "./scr/reimport.rs"]
mod reimport;
//#[path = "./scr/scraper.rs"]
//mod scraper;
#[path = "./scr/sharedtypes.rs"]
mod sharedtypes;
#[path = "./scr/tasks.rs"]
mod tasks;
#[path = "./scr/threading.rs"]
mod threading;
#[path = "./scr/time_func.rs"]
mod time_func;

// Needed for the plugin coms system.
#[path = "./scr/intcoms/client.rs"]
mod client;
#[path = "./scr/db/helpers.rs"]
mod helpers;
#[path = "./scr/os.rs"]
mod os;
#[path = "./scr/intcoms/server.rs"]
mod server;

// mod scr { pub mod cli; pub mod database; pub mod download; pub mod file; pub
// mod jobs; pub mod logging; pub mod plugins; pub mod scraper; pub mod
// sharedtypes; pub mod tasks; pub mod threading; pub mod time; }
/// This code is trash. lmao. Has threading and plugins soon tm Will probably work
/// :D
fn pause() {
    use std::io::{stdin, stdout, Read, Write};

    let mut stdout = stdout();
    stdout.write_all(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read_exact(&mut [0]).unwrap();
}

/// Creates DB from database.rs allows function calls.
fn makedb(dbloc: &str) -> database::Main {
    // Setting up DB VARS
    let path = dbloc.to_string();

    // let dbexist = Path::new(&path).exists(); dbcon is database connector let mut
    // dbcon = scr::database::dbinit(&path);
    database::Main::new(Some(path), VERS)
    // let dbcon = data.load_mem(&mut data._conn);
}

/// Gets file setup out of main. Checks if null data was written to data.
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
fn main() {
    let dbloc = "main.db";
    {
        let logloc = "log.txt";

        // Makes Logging work
        logging::main(&logloc.to_string());
    }
    os::check_os_compatibility();

    // Inits Database.
    let mut data = makedb(dbloc);
    data.load_table(&sharedtypes::LoadDBTable::Settings);

    let mut arc = Arc::new(Mutex::new(data));

    let jobmanager = Arc::new(Mutex::new(jobs::Jobs::new(arc.clone())));

    let mut globalload_arc = globalload::GlobalLoad::new(arc.clone(), jobmanager.clone());
    {
        let scraper_folder;
        let plugin_folder;
        {
            scraper_folder = arc.lock().unwrap().loaded_scraper_folder();
            plugin_folder = arc.lock().unwrap().loaded_plugin_folder();
        }
        globalload_arc.lock().unwrap().load_folder(&scraper_folder);
        globalload_arc.lock().unwrap().load_folder(&plugin_folder);
    }

    //let mut globalload_arc =
    //    plugins::globalload_arc::new(plugin_loc.to_string(), arc.clone(), jobmanager.clone());

    // Adds plugin and scraper callback capability from inside the db
    // Things like callbacks and the like
    arc.lock().unwrap().setup_globalload(globalload_arc.clone());

    globalload_arc.lock().unwrap().setup_ipc(
        globalload_arc.clone(),
        arc.clone(),
        jobmanager.clone(),
    );

    // Putting this down here after plugin manager because that's when the IPC server
    // starts and we can then inside of the scraper start calling IPC functions

    let mut upgradeversvec = Vec::new();

    // Upgrades the DB by geting version differences.
    // Waits for the entire DB to be upgraded before running scraper upgrades.
    'upgradeloop: loop {
        let repeat;
        {
            repeat = arc.lock().unwrap().check_version();
        }
        if !repeat {
            let lck = arc.lock().unwrap();
            upgradeversvec.push(lck.db_vers_get());
        } else {
            break 'upgradeloop;
        }
    }

    // Actually upgrades the DB from scraper calls
    for db_version in upgradeversvec {
        for (internal_scraper, scraper_library) in
            globalload_arc.lock().unwrap().library_get_raw().iter()
        {
            logging::info_log(&format!(
                "Starting scraper upgrade: {}",
                internal_scraper.name
            ));
            globalload::db_upgrade_call(scraper_library, &db_version, internal_scraper);
        }
    }

    // Processes any CLI input here
    cli::main(arc.clone(), globalload_arc.clone());

    // Calls the on_start func for the plugins
    globalload_arc.lock().unwrap().plugin_on_start();

    // A way to get around a mutex lock but it works lol
    let one_sec = time::Duration::from_millis(100);
    loop {
        if !globalload_arc.lock().unwrap().plugin_on_start_should_wait() {
            break;
        } else {
            thread::sleep(one_sec);
        }
    }

    // Checks if we need to load any jobs
    jobmanager.lock().unwrap().jobs_load(globalload_arc.clone());

    // One flush after all the on_start unless needed
    arc.lock().unwrap().transaction_flush();

    // Creates a threadhandler that manages callable threads.
    let mut threadhandler = threading::Threads::new();

    // just determines if we have any loaded jobs
    jobmanager.lock().unwrap().jobs_run_new();
    let arc_jobmanager = jobmanager;

    /* //
    // Loads the scrapers for their on_start function
    for (scraper, libloading) in globalload_arc.lock().unwrap().library_get_raw() {
        {
            dbg!(scraper);
            globalload::on_start(libloading, scraper);
        }
    }*/

    for scraper in arc_jobmanager.lock().unwrap().job_scrapers_get() {
        threadhandler.startwork(
            &mut arc_jobmanager.clone(),
            &mut arc,
            scraper.clone(),
            &mut globalload_arc,
        );
    }

    // Anything below here will run automagically. Jobs run in OS threads Waits until
    // all threads have closed.
    loop {
        let brk;
        {
            globalload_arc.lock().unwrap().thread_finish_closed();
            brk = globalload_arc.lock().unwrap().return_thread();
        }

        if brk && threadhandler.check_empty() {
            break;
        }
        {
            let mut jobmanager = arc_jobmanager.lock().unwrap();
            jobmanager.jobs_load(globalload_arc.clone());
        }
        for scraper in arc_jobmanager.lock().unwrap().job_scrapers_get() {
            // let scraper_library = scraper_manager._library.get(&scraper).unwrap();
            threadhandler.startwork(
                &mut arc_jobmanager.clone(),
                &mut arc,
                scraper.clone(),
                &mut globalload_arc,
            );
        }
        thread::sleep(one_sec);
        threadhandler.check_threads();
    }

    // This wait is done for allowing any thread to "complete" Shouldn't be nessisary
    // but hey. :D
    arc.lock().unwrap().transaction_flush();
    let mills_fifty = time::Duration::from_millis(5);
    std::thread::sleep(mills_fifty);
    logging::info_log(&"UNLOADING".to_string());
    log::logger().flush();
}
