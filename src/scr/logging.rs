#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use log::{error, info};

/// TODO Needs to make check if I have access to folder before I write db.
pub fn main(log_location: &String) {
    let log_bool = Path::new(log_location).exists();

    if log_bool {
        fs::remove_file(log_location).unwrap();
    }

    fast_log::init(fast_log::Config::new().file(log_location)).unwrap();
    info!("Initing Logger.");
    log::logger().flush();
}

///
/// Dumps error to log and panics.
///
pub fn panic_log(error: &String) {
    error!("{}", error);
    panic!("{}", error);
}
///
/// Dumps error to log and doesn't panic.
///
pub fn error_log(error: &String) {
    println!("{}", error);
    error!("{}", error);
}

///
/// Dumps info to log and prints it.
///
pub fn info_log(info: &String) {
    info!("{}", info);
    println!("{}", info);
}

///
/// Dumps info to log and prints it.
///
pub fn log(info: &String) {
    info!("{}", info);
}
