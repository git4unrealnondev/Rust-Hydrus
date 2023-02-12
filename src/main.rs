use crate::scr::sharedtypes::jobs_add;
use crate::scr::sharedtypes::AllFields::EJobsAdd;
use crate::scr::tasks;
use log::{error, info, warn};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task;
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
    pub mod tasks;
    pub mod threading;
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

    //let dbexist = Path::new(&path).exists();

    // dbcon is database connector

    //let mut dbcon = scr::database::dbinit(&path);

    scr::database::Main::new(path, vers.try_into().unwrap())

    //let dbcon =
    //data.load_mem(&mut data._conn);
}
/*
opt-level = 3
lto="fat"
codegenunits=1
strip = true
panic = "abort"
*/

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

    //dbg!(data.settings_get_name(&"pluginloadloc".to_string()));
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
            dbg!(
                0.to_string(),
                //&puts[2].to_string(),
                //puts[0].to_string(),
                //puts[1].to_string(),
                //trig,
                //puts[4].to_string(),
                &jobs_add.committype
            );
            data.jobs_add_new(
                &jobs_add.site,
                &jobs_add.query,
                &jobs_add.time,
                &jobs_add.committype,
                true,
            );
            dbg!(jobs_add);

            //let positive = AllField;
        }

        scr::sharedtypes::AllFields::EJobsRemove(jobs_remove) => {}
        scr::sharedtypes::AllFields::ESearch(search) => {
            dbg!(search);
            panic!();
        }

        scr::sharedtypes::AllFields::ETasks(task) => match task {
            scr::sharedtypes::Tasks::csv(location, csvdata) => {
                tasks::import_files(&location, csvdata);
            }
        },

        scr::sharedtypes::AllFields::ENothing => {}
    }
    panic!();
    data.transaction_flush();
    //let mut line = String::new();
    //let b1 = std::io::stdin().read_line(&mut line).unwrap();

    /*data.jobs_add_main(
        0.to_string(),
        &puts[2].to_string(),
        puts[0].to_string(),
        puts[1].to_string(),
        trig,
        puts[4].to_string(),
    );*/

    jobmanager.jobs_get(&data);

    // Converts db into Arc for multithreading
    let mut arc = Arc::new(Mutex::new(data));

    // Creates a threadhandler that manages callable threads.
    let mut threadhandler = scr::threading::threads::new();
    dbg!("a");
    jobmanager.jobs_run_new(&mut arc, &mut threadhandler);
    // Anything below here will run automagically.
    // Jobs run in OS threads

    //let test =

    //Finalizing wrapup.

    //jobmanager.jobs_cleanup();

    // Not needed due to sqlite closing db on close.

    //arc.lock().unwrap().transaction_flush();
    //arc.lock().unwrap().transaction_close();
    //info!("UNLOADING");
    log::logger().flush();
}
