use crate::logging;

///
/// This checks if the OS is compatibile with the rest of the code.
/// Currently only UNIX oses and WINDOWS oses are suppored.
///
pub fn check_os_compatibility() {
    if cfg!(unix) {
        logging::log(&format!("UNIX OS Detected."));
    } else if cfg!(windows) {
        logging::log(&format!("WINDOWS OS Detected."));
    } else {
        logging::panic_log(&format!("UNKNOWN OS Detected. PANICING"));
    }
}
