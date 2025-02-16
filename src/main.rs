// use crate::scr::sharedtypes::jobs_add; use
// crate::scr::sharedtypes::AllFields::EJobsAdd; use crate::scr::tasks;
use log::{error, warn};
use plugins::get_loadable_paths;
use scraper::db_upgrade_call;
use std::sync::Arc;
use std::sync::Mutex;

// use std::sync::{Arc, Mutex};
use std::{thread, time};

extern crate ratelimit;

#[path = "./scr/cli.rs"]
mod cli;
#[path = "./scr/database.rs"]
mod database;
#[path = "./scr/download.rs"]
mod download;
#[path = "./scr/file.rs"]
mod file;
#[path = "./scr/jobs.rs"]
mod jobs;
#[path = "./scr/logging.rs"]
mod logging;
#[path = "./scr/plugins.rs"]
mod plugins;
#[path = "./scr/reimport.rs"]
mod reimport;
#[path = "./scr/scraper.rs"]
mod scraper;
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
    let vers: usize = 5;

    // let dbexist = Path::new(&path).exists(); dbcon is database connector let mut
    // dbcon = scr::database::dbinit(&path);
    database::Main::new(path, vers)
    // let dbcon = data.load_mem(&mut data._conn);
}

// opt-level = 3 lto="fat" codegenunits=1 strip = true panic = "abort"
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
    let logloc = "log.txt";

    // Makes Logging work
    logging::main(&logloc.to_string());
    os::check_os_compatibility();
    let mut scraper_manager = scraper::ScraperManager::new();

    // Makes new scraper manager.
    scraper_manager.load(
        "./scrapers".to_string(),
        "/target/release/".to_string(),
        "so".to_string(),
    );

    // TODO NEEDS MAIN INFO PULLER HERE. PULLS IN EVERYTHING INTO DB. if let
    // scr::sharedtypes::AllFields::EJobsAdd(ref tempe) = all_field { dbg!(tempe);
    // dbg!(&tempe.site); } Checks main.db log location.
    db_file_sanity(dbloc);

    // Inits Database.
    let mut data = makedb(dbloc);

    // NOTE ONLY USER FOR LOADING DB DYNAMICALLY
    let alt_connection = database::dbinit(&dbloc.to_string());
    data.load_table(&sharedtypes::LoadDBTable::Settings);
    let plugin_option = data.settings_get_name(&"pluginloadloc".to_string());
    let plugin_loc = match plugin_option {
        None => "".to_string(),
        Some(pluginobj) => pluginobj.param.as_ref().unwrap().clone(),
    };

    let mut arc = Arc::new(Mutex::new(data));

    let mut jobmanager = Arc::new(Mutex::new(jobs::Jobs::new(arc.clone())));
    let mut pluginmanager =
        plugins::PluginManager::new(plugin_loc.to_string(), arc.clone(), jobmanager.clone());
    // Putting this down here after plugin manager because that's when the IPC server
    // starts and we can then inside of the scraper start calling IPC functions

    let mut upgradeversvec = Vec::new();

    // Upgrades the DB by geting version differences.
    // Waits for the entire DB to be upgraded before running scraper upgrades.
    'upgradeloop: loop {
        let repeat;
        {
            repeat = arc
                .lock()
                .unwrap()
                .check_version(&mut scraper_manager)
                .clone();
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
        for (internal_scraper, scraper_library) in scraper_manager.library_get().iter() {
            logging::info_log(&format!(
                "Starting scraper upgrade: {}",
                internal_scraper.name
            ));
            db_upgrade_call(scraper_library, &db_version);
        }
    }

    // Grabs location of db
    let location = arc.lock().unwrap().location_get();
    file::folder_make(&location.to_string());

    // TODO Put code here
    cli::main(arc.clone(), &mut scraper_manager);

    // Checks if we need to load any jobs
    logging::info_log(&format!("Checking if we have any Jobs to run."));
    arc.lock()
        .unwrap()
        .load_table(&sharedtypes::LoadDBTable::Jobs);

    arc.lock().unwrap().transaction_flush();
    jobmanager.lock().unwrap().jobs_load(&scraper_manager);

    // Calls the on_start func for the plugins
    pluginmanager.lock().unwrap().plugin_on_start();

    // Creates a threadhandler that manages callable threads.
    let mut threadhandler = threading::Threads::new();
    jobmanager.lock().unwrap().jobs_run_new(
        &mut arc,
        &mut threadhandler,
        &mut pluginmanager,
        &scraper_manager,
    );
    let arc_jobmanager = jobmanager;
    let arc_scrapermanager = Arc::new(Mutex::new(scraper_manager));
    for (scraper, _) in arc_jobmanager.lock().unwrap()._jobref.clone() {
        // let scraper_library = scraper_manager._library.get(&scraper).unwrap();
        threadhandler.startwork(
            &mut arc_jobmanager.clone(),
            &mut arc,
            scraper,
            &mut pluginmanager,
            arc_scrapermanager.clone(),
        );
    }

    // Anything below here will run automagically. Jobs run in OS threads Waits until
    // all threads have closed.
    let one_sec = time::Duration::from_millis(100);
    loop {
        let brk;
        {
            pluginmanager.lock().unwrap().thread_finish_closed();
            brk = pluginmanager.lock().unwrap().return_thread();
        }
        if brk {
            break;
        }
        thread::sleep(one_sec);
        {
            let mut jobmanager = arc_jobmanager.lock().unwrap();
            if jobmanager.jobs_empty() {
                jobmanager.jobs_load(&arc_scrapermanager.lock().unwrap());
            }
        }
        threadhandler.check_threads();
        for (scraper, _) in arc_jobmanager.lock().unwrap()._jobref.clone() {
            // let scraper_library = scraper_manager._library.get(&scraper).unwrap();
            threadhandler.startwork(
                &mut arc_jobmanager.clone(),
                &mut arc,
                scraper,
                &mut pluginmanager,
                arc_scrapermanager.clone(),
            );
        }
    }

    arc.lock().unwrap().transaction_flush();

    // pluginmanager.lock().unwrap().thread_finish_closed(); while
    // !pluginmanager.lock().unwrap().return_thread() { let one_sec =
    // time::Duration::from_secs(1);
    // pluginmanager.lock().unwrap().thread_finish_closed(); thread::sleep(one_sec); }
    // This wait is done for allowing any thread to "complete" Shouldn't be nessisary
    // but hey. :D
    let mills_fifty = time::Duration::from_millis(5);
    std::thread::sleep(mills_fifty);
    logging::info_log(&"UNLOADING".to_string());
    log::logger().flush();
    // drop(pluginmanager.lock().unwrap()); unsafe { libc::malloc_trim(0) };
}
