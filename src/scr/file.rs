use std::fs;
use std::io::Error;

use crate::logging;

/// Returns OK or Err if file size is eq to inint.
pub fn size_eq(input: String, inint: u64) -> std::io::Result<()> {
    let size = fs::metadata(input)?;
    if inint == size.len() {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
    // assert_eq!(inint, size.len());
}

/// Removes a file from the folder.
pub fn remove_file(input: String) -> std::io::Result<()> {
    fs::remove_file(input)?;
    Ok(())
}

/// Make Folder
pub fn folder_make(location: &String) {
    if let Err(err) = std::fs::create_dir_all(location) {
        logging::error_log(&format!("Failed to make folder at path: {}", location));
        logging::error_log(&format!("folder_make: err {}", err));
    }
}
