#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use fast_log;
use log::info;

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
