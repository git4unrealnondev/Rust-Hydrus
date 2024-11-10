use crate::logging;

/// This checks if the OS is compatibile with the rest of the code. Currently only
/// UNIX oses and WINDOWS oses are suppored.
pub fn check_os_compatibility() {
    if cfg!(unix) {
        logging::log(&"UNIX OS Detected.".to_string());
    } else if cfg!(windows) {
        logging::log(&"WINDOWS OS Detected.".to_string());
    } else {
        logging::panic_log(&"UNKNOWN OS Detected. PANICING".to_string());
    }
}
