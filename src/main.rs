use log::{error, info, warn};
use std::path::Path;

extern crate ratelimit;

mod scr {
    pub mod database;
    pub mod file;
    pub mod logging;
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

    data.transaction_start();

    if !dbexist {
        data.first_db();
    } else {
        println!("Database Exists: {} : Skipping creation.", dbexist);
        info!("Database Exists: {} : Skipping creation.", dbexist);
    }

    data.updatedb();
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

/// Makes logging happen
fn makelog(logloc: &str) {
    //Inits logging::main at log.txt
    scr::logging::main(&logloc);
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

    //let mut vec = vec![1, 2, 3, "4"];
    data.transaction_flush();
    for a in 1..999999 {
        data.add_setting(
            "Test".to_string(),
            "None".to_string(),
            a,
            "./Files/".to_string(),
        );
    }

    //Finalizing wrapup.
    data.transaction_close();

    log::logger().flush();
}
