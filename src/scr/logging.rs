#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use fast_log;
use log::{error, info};

/// TODO Needs to make check if I have access to folder before I write db.
pub fn main(loglock: &str) {
    let log_bool = Path::new(loglock).exists();

    if log_bool {
        fs::remove_file(loglock).unwrap();
    }

    fast_log::init(fast_log::Config::new().file(loglock)).unwrap();
    info!("Initing Logger.");
    log::logger().flush();
}

///
/// Dumps error to log and panics.
///
pub fn error_log(error: &String) {
    error!("{}", error);
    panic!("{}", error);
}

pub fn info_log(info: &String) {
    info!("{}", info);
    println!("{}", info);
}
