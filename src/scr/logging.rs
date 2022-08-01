use fast_log;
use log::info;

/// TODO Needs to make check if I have access to folder before I write db.
pub fn main(loglock: &str) {
    fast_log::init(fast_log::Config::new().file(&loglock)).unwrap();
    info!("Initing Logger.");
    log::logger().flush();
}
