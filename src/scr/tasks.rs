#![allow(dead_code)]
#![allow(unused_variables)]

use super::database;
use super::download;
use super::sharedtypes;
use crate::helpers;
use crate::logging;
use ahash::AHashMap;
use file_format::FileFormat;
use log::{error, info};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Row {
    path: String,
    tag: String,
    namespace: String,
    parent: Option<String>,
    id: usize,
}

/// Just a holder for tasks. Can be called from here or any place really. :D
/// Currently supports only one file to tag assiciation. Need to add support for
/// multiple tags. But this currently works for me.
pub fn import_files(
    location: &String,
    csvdata: sharedtypes::CsvCopyMvHard,
    db: &mut database::Main,
) {
    if !Path::new(&location).exists() {
        error!("Path: {} Doesn't exist. Exiting. Check logs", &location);
        panic!("Path: {} Doesn't exist. Exiting. Check logs", &location);
    }
    let mut rdr = csv::ReaderBuilder::new().from_path(location).unwrap();
    let mut headers: Vec<String> = Vec::new();
    let headerrecord = rdr.headers().unwrap().clone();
    for head in headerrecord.iter() {
        headers.push(head.to_string());
    }

    // Checks if path is missing
    if !headers.contains(&"path".to_string()) {
        error!("CSV ERROR, issue with csv file. No path header.");
        panic!("CSV ERROR, issue with csv file. No path header.");
    }
    let location = db.location_get();
    println!("Importing Files to: {}", &location);
    let mut delfiles: AHashMap<String, String> = AHashMap::new();
    for line in rdr.records() {
        let row: Row = line
            .as_ref()
            .unwrap()
            .deserialize(Some(&headerrecord))
            .unwrap();
        if !Path::new(&row.path).exists() {
            error!("Path: {} Doesn't exist. Exiting. Check logs", &row.path);
            println!("Path: {} Doesn't exist. Exiting. Check logs", &row.path);
            continue;
        }

        // If we can hash the file then go on if we can't hash the file for some weird reason then
        // ignore the file and continue downloading
        let (hash, _b) = match download::hash_file(
            &row.path,
            &sharedtypes::HashesSupported::Sha512("".to_string()),
        ) {
            Err(err) => {
                logging::info_log(format!("Cannot hash file {} err: {:?}", &row.path, err));
                continue;
            }
            Ok(out) => out,
        };
        let hash_exists = db.file_get_hash(&hash);
        if hash_exists.is_some() {
            // delfiles.insert(row.path.to_string(), "".to_owned()); Removes file that's
            // already in DB.
            fs::remove_file(&row.path).unwrap();
            println!("File: {} already in DB. Skipping import.", &row.path);
            info!("File: {} already in DB. Skipping import.", &row.path);
            continue;
        }
        let path = helpers::getfinpath(&location, &hash);
        let final_path = format!("{}/{}", path, &hash);
        let file_ext = FileFormat::from_file(&row.path)
            .unwrap()
            .extension()
            .to_string();

        // Completes file actions.
        match csvdata {
            sharedtypes::CsvCopyMvHard::Copy => {
                fs::copy(&row.path, &final_path).unwrap();
            }
            sharedtypes::CsvCopyMvHard::Move => {
                fs::copy(&row.path, &final_path).unwrap();
                delfiles.insert(row.path.to_string(), "".to_owned());
            }
            sharedtypes::CsvCopyMvHard::Hardlink => {
                fs::hard_link(&row.path, &final_path).unwrap();
            }
        }
        println!("Copied to path: {}", &final_path);

        db.storage_put(&location);
        let storage_id = db.storage_get_id(&location).unwrap();

        // Gets the extension id from a string
        let ext_id = db.extension_put_string(&file_ext);

        // Adds into DB
        let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
            hash,
            ext_id,
            storage_id,
        });
        let file_id = db.file_add(file);
        let namespace_id = db.namespace_add(&row.namespace, &None);
        let tag_id = db.tag_add(&row.tag, namespace_id, true, Some(row.id));
        db.relationship_add(file_id.to_owned(), tag_id.to_owned(), true);
    }
    db.transaction_flush();
    println!("Clearing any files from any move ops.");
    info!("Clearing any files from any move ops.");
    for each in delfiles.keys() {
        fs::remove_file(each).unwrap();
    }
    dbg!("Done!");
    info!("Done!");
}
