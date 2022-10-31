use crate::scr::sharedtypes::jobs_add;
use crate::scr::sharedtypes::AllFields::EJobsAdd;
use log::{error, info, warn};
use std::path::Path;
extern crate ratelimit;

mod scr {
    pub mod cli;
    pub mod database;
    pub mod download;
    pub mod file;
    pub mod jobs;
    pub mod logging;
    pub mod plugins;
    pub mod scraper;
    pub mod sharedtypes;
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
        data.db_commit_man_set();
    } else {
        println!("Database Exists: {} : Skipping creation.", dbexist);
        info!("Database Exists: {} : Skipping creation.", dbexist);
    }

    data.load_mem();

    data
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
                Ok(_fileret) => _fileret,
            }
        }
        Err(_dbzero) => {}
    }
}

/// Makes logging happen
fn makelog(logloc: &str) {
    //Inits logging::main at log.txt
    scr::logging::main(logloc)
}

/// Main function.
fn main() {
    let dbloc = "./main.db";

    // Makes Logging work
    makelog("./log.txt");

    //TODO NEEDS MAIN INFO PULLER HERE. PULLS IN EVERYTHING INTO DB.
    let all_field = scr::cli::main();
    if let scr::sharedtypes::AllFields::EJobsAdd(ref tempe) = all_field {
        dbg!(tempe);
        dbg!(&tempe.site);
    }
    // Checks main.db log location.
    db_file_sanity(dbloc);

    //Inits Database.
    let mut data = makedb(dbloc);

    data.transaction_flush();
    data.check_version();

    //Inits ScraperManager
    let plugin_loc = data
        .settings_get_name(&"pluginloadloc".to_string())
        .unwrap()
        .1;
    let pluginmanager = scr::plugins::PluginManager::new(plugin_loc);

    let location = data.settings_get_name(&"FilesLoc".to_string()).unwrap().1;
    scr::file::folder_make(&format!("./{}", &location));

    //TODO Put code here

    // Makes new scraper manager.
    let mut scraper_manager = scr::scraper::ScraperManager::new();
    scraper_manager.load(
        "./scrapers".to_string(),
        "/target/release/".to_string(),
        "so".to_string(),
    );

    // Looks like some search functionality. From before when I had searching as an option.
    /*if name == "id" {
        dbg!(&puts);
        let uid = puts[0].parse::<usize>().unwrap();
        let a = data.relationship_get_tagid(&uid);

        for each in &a {
            println!("{:?}", data.tag_id_get(each));
        }
    }*/

    let mut jobmanager = scr::jobs::Jobs::new(scraper_manager);

    data.transaction_flush();

    match all_field {
        scr::sharedtypes::AllFields::EJobsAdd(ref jobs_add) => {
            //let positive = AllField;
        }
        scr::sharedtypes::AllFields::EJobsRemove(jobs_remove) => {}
        scr::sharedtypes::AllFields::ESearch(search) => {}
        scr::sharedtypes::AllFields::ENothing => {}
    }

    /* if run {
        data.jobs_add_main(
            0.to_string(),
            &puts[2].to_string(),
            puts[0].to_string(),
            puts[1].to_string(),
            trig,
            puts[4].to_string(),
        );
    }*/
    jobmanager.jobs_get(&data);
    jobmanager.jobs_run(&mut data);

    //Finalizing wrapup.
    data.transaction_flush();
    data.transaction_close();
    info!("UNLOADING");
    log::logger().flush();
}
