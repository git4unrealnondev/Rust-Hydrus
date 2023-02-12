use super::sharedtypes;
use csv;
use log::{error, info};
use std::fs;
use std::path::Path;

///
/// Just a holder for tasks. Can be called from here or any place really. :D
///
pub fn import_files(location: &String, csvdata: sharedtypes::CsvCopyMvHard) {
    if !Path::new(&location).exists() {
        error!("Path: {} Doesn't exist. Exiting. Check logs", &location);
        panic!("Path: {} Doesn't exist. Exiting. Check logs", &location);
    }

    let mut rdr = csv::ReaderBuilder::new().from_path(&location).unwrap();

    let headers = rdr.headers().unwrap();
    dbg!(headers);
    

    let mut record = csv::ByteRecord::new();
    for line in rdr.records() {
        dbg!(line.unwrap().to_owned());
        panic!();
    }
}
