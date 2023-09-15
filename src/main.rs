//use crate::scr::sharedtypes::jobs_add;
//use crate::scr::sharedtypes::AllFields::EJobsAdd;
//use crate::scr::tasks;
use log::{error, info, warn};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task;
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
#[path = "./scr/scraper.rs"]
mod scraper;
#[path = "./scr/sharedtypes.rs"]
mod sharedtypes;
#[path = "./scr/tasks.rs"]
mod tasks;
#[path = "./scr/threading.rs"]
mod threading;
#[path = "./scr/time.rs"]
mod time;
/*
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
}*/

///
/// This code is trash. lmao.
/// Has threading and plugins soon tm
/// Will probably work :D
///

/// Creates DB from database.rs allows function calls.
fn makedb(dbloc: &str) -> database::Main {
    // Setting up DB VARS
    let path = dbloc.to_string();
    let vers: u64 = 2;

    //let dbexist = Path::new(&path).exists();

    // dbcon is database connector

    //let mut dbcon = scr::database::dbinit(&path);

    database::Main::new(path, vers.try_into().unwrap())

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

/// Makes logging happen
fn makelog(logloc: &str) {
    //Inits logging::main at log.txt
    logging::main(logloc)
}

/// Main function.
fn main() {
    let dbloc = "./main.db";

    // Makes Logging work
    makelog("./log.txt");

    //TODO NEEDS MAIN INFO PULLER HERE. PULLS IN EVERYTHING INTO DB.
    let all_field = cli::main();
    //if let scr::sharedtypes::AllFields::EJobsAdd(ref tempe) = all_field {
    //    dbg!(tempe);
    //    dbg!(&tempe.site);
    //}
    // Checks main.db log location.
    db_file_sanity(dbloc);

    //Inits Database.
    let mut data = makedb(dbloc);
    let mut alt_connection = database::dbinit(&dbloc.to_string()); // NOTE ONLY USER FOR LOADING DB DYNAMICALLY
    data.load_table(&sharedtypes::LoadDBTable::Settings, &mut alt_connection);

    data.transaction_flush();
    data.check_version();

    //dbg!(data.settings_get_name(&"pluginloadloc".to_string()));
    //Inits ScraperManager
    let plugin_loc = data
        .settings_get_name(&"pluginloadloc".to_string())
        .unwrap()
        .name;

    let location = data
        .settings_get_name(&"FilesLoc".to_string())
        .unwrap()
        .param
        .unwrap();
    file::folder_make(&format!("./{}", &location));

    //TODO Put code here

    // Makes new scraper manager.
    let mut scraper_manager = scraper::ScraperManager::new();
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

    data.load_table(&sharedtypes::LoadDBTable::Jobs, &mut alt_connection);
    let mut jobmanager = jobs::Jobs::new(scraper_manager);

    data.transaction_flush();

    match all_field {
        sharedtypes::AllFields::JobsAdd(ref jobs_add) => {
            data.jobs_add_new(
                &jobs_add.site,
                &jobs_add.query,
                &jobs_add.time,
                &jobs_add.committype,
                true,
            );

            //let positive = AllField;
        }

        sharedtypes::AllFields::JobsRemove(jobs_remove) => {}
        sharedtypes::AllFields::Search(search) => {
            match &search {
                sharedtypes::Search::Fid(file) => {}
                sharedtypes::Search::Tid(tagid) => {}
                sharedtypes::Search::Tag(tag) => {
                    if tag.len() == 1 {
                        logging::info_log(
                            &"One item was passed into tag search. Will search only based off tag."
                                .to_string(),
                        );
                    } else if tag.len() == 2 {
                        logging::info_log(&"Normal tag search was done. Searching for 2nd item in namespace to get tag id.".to_string());
                        dbg!(tag.get(0));
                        dbg!(tag.get(1));

                        data.load_table(&sharedtypes::LoadDBTable::Files, &mut alt_connection);
                        data.load_table(&sharedtypes::LoadDBTable::Tags, &mut alt_connection);
                        data.load_table(
                            &sharedtypes::LoadDBTable::Relationship,
                            &mut alt_connection,
                        );
                        data.load_table(&sharedtypes::LoadDBTable::Namespace, &mut alt_connection);

                        let tag_namespace = data.namespace_get(&tag.get(1).unwrap());

                        match tag_namespace {
                            None => {
                                logging::info_log(&format!("Couldn't fine namespace from search"));
                            }
                            Some(namespace_id) => {
                                let tag_option = data
                                    .tag_get_name(tag.get(0).unwrap().to_string(), namespace_id);
                                match tag_option {
                                    None => {
                                        logging::info_log(&format!("Couldn't find any tag id's that use namespace_id: {} with tag: {}", namespace_id, tag.get(0).unwrap()));
                                        logging::info_log(&"Will try a generic search".to_string());
                                    }
                                    Some(tag_id) => {
                                        logging::info_log(&format!("Found a tag id's that use namespace_id: {} with tag: {} tagid {}", namespace_id, tag.get(0).unwrap(), tag_id));
                                        let file_ids = data.relationship_get_fileid(&tag_id);
                                        for each in file_ids {
                                            //dbg!(each);
                                            let file_info = data.file_get_id(&each).unwrap();
                                            logging::info_log(&format!(
                                                "Found File: {} {} {}",
                                                file_info.0, file_info.1, file_info.2
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    } else if tag.len() >= 3 {
                        logging::info_log(&"Three items were passed into the tag search using first and second only.".to_string());
                    } else if tag.len() == 0 {
                        logging::info_log(&"Tag was passed into search but no info was provided. THIS SHOULDN'T HAPPEN.".to_string());
                    }
                }
                sharedtypes::Search::Hash(hash) => {}
            }

            dbg!(search);
            panic!();
        }

        sharedtypes::AllFields::Tasks(task) => match task {
            sharedtypes::Tasks::Csv(location, csvdata) => {
                tasks::import_files(&location, csvdata, &mut data);
            }
        },

        sharedtypes::AllFields::Nothing => {}
    }

    data.transaction_flush();

    jobmanager.jobs_get(&data);

    // Converts db into Arc for multithreading
    let mut arc = Arc::new(Mutex::new(data));

    let pluginmanager = Arc::new(Mutex::new(plugins::PluginManager::new(
        plugin_loc,
        arc.clone(),
    )));

    // Creates a threadhandler that manages callable threads.
    let mut threadhandler = threading::threads::new();

    jobmanager.jobs_run_new(
        &mut arc,
        &mut threadhandler,
        &mut alt_connection,
        pluginmanager,
    );
    // Anything below here will run automagically.
    // Jobs run in OS threads

    //let test =

    //Finalizing wrapup.

    //jobmanager.jobs_cleanup();

    // Not needed due to sqlite closing db on close.

    //arc.lock().unwrap().transaction_flush();
    //arc.lock().unwrap().transaction_close();
    info!("UNLOADING");
    log::logger().flush();
}
