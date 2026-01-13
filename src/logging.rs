#![forbid(unsafe_code)]

use fast_log::consts::LogSize;
use fast_log::plugin::file_split::{KeepType, Rolling};
use fast_log::plugin::packer::LZ4Packer;
use log::{LevelFilter, error, info};
use std::fmt::Display;
use std::fs;
use std::path::Path;

/// TODO Needs to make check if I have access to folder before I write db.
pub fn main(log_location: &String) {
    let log_bool = Path::new(log_location).exists();
    if log_bool {
        fs::remove_file(log_location).unwrap();
    }
    let fastlog = fast_log::Config::new()
        .chan_len(Some(100000))
        .file(log_location)
        .file_split(
            "logs/",
            Rolling::new(fast_log::plugin::file_split::RollingType::BySize(
                LogSize::GB(2),
            )),
            KeepType::KeepNum(2),
            LZ4Packer {},
        )
        .level(LevelFilter::Info);
    fast_log::init(fastlog).unwrap();
    info!("Initing Logger.");
    log::logger().flush();
}

/// Dumps error to log and panics.
pub fn panic_log<T: Display>(error: T) {
    error!("{}", error);
    panic!("{}", error);
}

/// Dumps error to log and doesn't panic.
pub fn error_log<T: Display>(error: T) {
    println!("{}", error);
    error!("{}", error);
}

/// Dumps error to log and doesn't panic.
/// Does NOT print anything to the screen
pub fn error_log_silent<T: Display>(error: T) {
    error!("{}", error);
}

/// Dumps info to log and prints it.
pub fn info_log<T: Display>(info: T) {
    info!("{}", info);
    println!("{}", info);
}

/// Dumps info to log and DOES NOT prints it.
pub fn log<T: Display>(info: T) {
    info!("{}", info);
}
