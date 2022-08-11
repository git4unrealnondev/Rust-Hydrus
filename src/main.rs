use log::{error, info, warn};
use std::path::Path;

extern crate ratelimit;

mod scr {
    pub mod cli;
    pub mod database;
    pub mod file;
    pub mod jobs;
    pub mod logging;
    pub mod scraper;
    pub mod time;
}

///
/// I dont want to keep writing .to_string on EVERY vector of strings.
/// Keeps me lazy.
/// vec_of_strings["one", "two"];
///
#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

///
/// This code is trash. lmao.
/// Has threading and plugins soon tm
/// Will probably work :D
///

/// Creates DB from database.rs allows function calls.
fn makedb(dbloc: &str) -> scr::database::Main {
    // Setting up DB VARS
    let path = dbloc.to_string();
    let vers: u64 = 1;

    let dbexist = Path::new(&path).exists();

    // dbcon is database connector

    let dbcon = scr::database::dbinit(&path);

    let mut data = scr::database::Main::new(dbcon, path, vers.try_into().unwrap());
    data.db_open();

    data.vacuum();

    data.transaction_start();

    if !dbexist {
        data.first_db();
        data.updatedb();
    } else {
        println!("Database Exists: {} : Skipping creation.", dbexist);
        info!("Database Exists: {} : Skipping creation.", dbexist);
    }

    return data;
}

/// Gets file setup out of main.
/// Checks if null data was written to data.
fn db_file_sanity(dbloc: &str) {
    let _dbzero = scr::file::size_eq(dbloc.to_string(), 0);
    match _dbzero {
        Ok(_dbzero) => {
            println!("File is zero: {} will remove and create.", dbloc);
            warn!("File is zero: {} will remove and create.", dbloc);
            let _fileret = scr::file::remove_file(dbloc.to_string());
            match _fileret {
                Err(_fileret) => {
                    error!("ERROR CANT DELETE FILE!!! CLOSING RAPIDLY.");
                    panic!("ERROR CANT DELETE FILE!!! CLOSING RAPIDLY.");
                }
                Ok(_fileret) => (_fileret),
            }
        }
        Err(_dbzero) => {}
    }
}

fn cli_parser() {
    println!("Not yet implements");
}

/// Makes logging happen
fn makelog(logloc: &str) {
    //Inits logging::main at log.txt
    scr::logging::main(&logloc)
}

/// Main function.
fn main() {
    let dbloc = "./main.db";

    // Makes Logging work
    makelog("./log.txt");

    // Checks main.db log location.
    db_file_sanity(dbloc);

    //Inits Database.
    let mut data = makedb(dbloc);
    data.load_mem();

    data.transaction_flush();
    data.check_version();
    //TODO Put code here

    // Makes new scraper manager.
    let mut scraper_manager = scr::scraper::ScraperManager::new();
    scraper_manager.load(
        "./scrapers".to_string(),
        "/target/release/".to_string(),
        "so".to_string(),
    );

    //TODO NEEDS MAIN INFO PULLER HERE. PULLS IN EVERYTHING INTO DB.
    let (puts, name, trig, run) = scr::cli::main();

    let mut jobmanager = scr::jobs::Jobs::new();

    if run {
        data.jobs_add(
            0.to_string(),
            puts[2].to_string(),
            puts[0].to_string(),
            puts[1].to_string(),
            trig,
        );
    }
    jobmanager.jobs_get(&data, scraper_manager);
    jobmanager.jobs_run();

    //Finalizing wrapup.
    data.transaction_flush();
    data.transaction_close();

    log::logger().flush();
}
