use crate::database::Main;
use crate::globalload::GlobalLoad;

// extern crate urlparse;
use super::sharedtypes;
use crate::logging;
use bytes::Bytes;
use core::time;
use file_format::FileFormat;
use log::{error, info};
use reqwest::blocking::Client;
use reqwest::cookie;
use sha2::Digest as sha2Digest;
use sha2::Sha256;
use sha2::Sha512;
use std::error::Error;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::time::Duration;
use url::Url;

extern crate reqwest;

use ratelimit::Ratelimiter;
use std::fs::File;

// use std::sync::{Arc, Mutex};
use std::sync::Arc;
//use std::sync::Mutex;
use crate::Mutex;
use crate::RwLock;
use std::thread;

/// Makes ratelimiter and example
pub fn ratelimiter_create(
    workerid: &usize,
    jobid: &usize,
    number: u64,
    duration: Duration,
) -> Ratelimiter {
    logging::info_log(&format!(
        "Worker: {} JobId: {} -- Making ratelimiter with: {} Request Per: {:?}",
        workerid, jobid, &number, &duration
    ));
    loop {
        // The wrapper that implements ratelimiting

        match Ratelimiter::builder(number, duration)
            .max_tokens(number)
            .initial_available(number)
            .build()
        {
            Ok(ratelimiter) => {
                return ratelimiter;
            }
            Err(err) => {
                logging::error_log(&format!("Failed to make ratelimiter with err: {:?}", err));
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

pub fn ratelimiter_wait(ratelimit_object: &Arc<Mutex<Ratelimiter>>) {
    loop {
        let limit;
        {
            let hold = ratelimit_object.lock().unwrap();
            limit = hold.try_wait();
        }
        match limit {
            Ok(_) => break,
            Err(sleep) => {
                std::thread::sleep(sleep);
            }
        }
    }
}

/// Creates Client that the downloader will use.
pub fn client_create() -> Client {
    let useragent = "RustHydrusV1 0".to_string();
    let jar = cookie::Jar::default();

    // The client that does the downloading
    reqwest::blocking::ClientBuilder::new()
        .user_agent(useragent)
        .cookie_provider(jar.into())
        //. brotli(true)
        //. deflate(true)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .zstd(true)
        .timeout(time::Duration::from_secs(1200))
        .build()
        .unwrap()
}

/// Downloads text into db as responses. Filters responses by default limit if
/// their's anything wrong with request.
pub async fn dltext_new(
    url_string: &String,
    client: &Client,
    ratelimiter_obj: &Arc<Mutex<Ratelimiter>>,
    worker_id: &usize,
) -> Result<String, Box<dyn Error>> {
    // let mut ret: Vec<AHashMap<String, AHashMap<String, Vec`<String>`>>> =
    // Vec::new(); let ex = Executor::new(); let url =
    // Url::parse("http://www.google.com").unwrap();
    let mut cnt = 0;
    loop {
        let url = match Url::parse(url_string) {
            Ok(out) => out,
            Err(err) => {
                logging::error_log(&format!(
                    "Failed to parse URL: {} with error {:?}",
                    url_string, err
                ));
                return Err(Box::new(err));
            }
        };

        // let requestit = Request::new(Method::GET, url); fut.push();
        logging::info_log(&format!(
            "Worker: {} -- Spawned web reach to: {}",
            worker_id, &url_string
        ));

        // let futureresult = futures::executor::block_on(ratelimit_object.ready())
        // .unwrap()
        ratelimiter_wait(ratelimiter_obj);
        let futureresult = client.get(url).send();

        // let test = reqwest::get(url).await.unwrap().text(); let futurez =
        // futures::executor::block_on(futureresult); dbg!(&futureresult);
        if cnt == 3 {
            return Err(Box::new(futureresult.err().unwrap()));
        }
        cnt += 1;
        match futureresult {
            Ok(res) => match res.text() {
                Ok(text) => {
                    return Ok(text);
                }
                Err(_) => {
                    continue;
                }
            },
            Err(_) => {
                return Err(Box::new(futureresult.err().unwrap()));
            }
        }
    }
}

/// Hashes the bytes and compares it to what the scraper should of recieved.
pub fn hash_bytes(bytes: &Bytes, hash: &sharedtypes::HashesSupported) -> (String, bool) {
    match hash {
        sharedtypes::HashesSupported::Md5(hash) => {
            let digest = md5::compute(bytes);

            // let sharedtypes::HashesSupported(hashe, _) => hash;
            if &format!("{:x}", digest) != hash {
                info!("Parser returned: {} Got: {:?}", &hash, &digest);
            }
            (format!("{:x}", digest), &format!("{:x}", digest) == hash)
        }
        sharedtypes::HashesSupported::Sha1(hash) => {
            let mut hasher = sha1::Sha1::new();
            hasher.update(bytes);
            let hastring = format!("{:X}", hasher.finalize());
            let dune = &hastring == hash;
            if !dune {
                info!("Parser returned: {} Got: {}", &hash, &hastring);
            }
            (hastring, dune)
        }
        sharedtypes::HashesSupported::Sha256(hash) => {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let hastring = format!("{:X}", hasher.finalize());
            let dune = &hastring == hash;
            if !dune {
                info!("Parser returned: {} Got: {}", &hash, &hastring);
            }
            (hastring, dune)
        }
        sharedtypes::HashesSupported::Sha512(hash) => {
            let mut hasher = Sha512::new();
            hasher.update(bytes);
            let hastring = format!("{:X}", hasher.finalize());
            let dune = &hastring == hash;
            if !dune {
                info!("Parser returned: {} Got: {}", &hash, &hastring);
            }
            (hastring, dune)
        }
        sharedtypes::HashesSupported::None => ("".to_string(), false),
    }
}

///
/// Determines the file return status
///
pub enum FileReturnStatus {
    // If a URL is dead (400+) error
    DeadUrl(String),
    // File hash,ext
    File((String, String)),
    // Other issue. Try again later
    TryLater,
}

/// Downloads file to position
pub fn dlfile_new(
    client: &Client,
    db: Arc<Mutex<Main>>,
    file: &mut sharedtypes::FileObject,
    location: &String,
    globalload: Option<Arc<RwLock<GlobalLoad>>>,
    ratelimiter_obj: &Arc<Mutex<Ratelimiter>>,
    source_url: &String,
    workerid: &usize,
    jobid: &usize,
) -> FileReturnStatus {
    let mut boolloop = true;
    let mut hash = String::new();
    let mut bytes: bytes::Bytes = Bytes::from(&b""[..]);
    let mut cnt = 0;
    while boolloop {
        let mut hasher = Sha512::new();
        loop {
            let fileurlmatch = match &file.source_url {
                None => {
                    panic!(
                        "Tried to call dlfilenew when their was no file :C info: {:?}",
                        file
                    );
                }
                Some(fileurl) => fileurl,
            };
            let url = Url::parse(fileurlmatch).unwrap();
            ratelimiter_wait(ratelimiter_obj);
            logging::info_log(&format!("Downloading: {} to: {}", &fileurlmatch, &location));
            let mut futureresult = client.get(url.as_ref()).send();
            loop {
                match &futureresult {
                    Ok(result) => {
                        if let Err(err) = result.error_for_status_ref() {
                            if let Some(err_status) = err.status() {
                                if err_status.is_client_error() {
                                    logging::error_log(&format!(
                                        "Worker: {workerid} JobID: {jobid} -- Stopping file download due to: {:?}",
                                        err
                                    ));
                                    return FileReturnStatus::DeadUrl(source_url.clone());
                                }
                            }
                        }
                        break;
                    }
                    Err(_) => {
                        error!("Worker: {workerid} JobID: {jobid} -- Repeating: {}", &url);
                        dbg!("Worker: {workerid} JobID: {jobid} -- Repeating: {}", &url);
                        let time_dur = Duration::from_secs(10);
                        thread::sleep(time_dur);
                        futureresult = client.get(url.as_ref()).send();
                    }
                }
            }

            // Downloads file into byte memory buffer
            let byte = futureresult.unwrap().bytes();
            // Error handling for dling a file. Waits 10 secs to retry
            match byte {
                Ok(out) => {
                    bytes = out;

                    break;
                }
                Err(_) => {
                    error!(
                        "Worker: {workerid} JobID: {jobid} -- Repeating: {} , Due to: {:?}",
                        &url,
                        &byte.as_ref().err()
                    );
                    dbg!(
                        "Worker: {workerid} JobID: {jobid} -- Repeating: {} , Due to: {:?}",
                        &url,
                        &byte.as_ref().err()
                    );
                    let time_dur = Duration::from_secs(10);
                    thread::sleep(time_dur);
                }
            }
            if cnt >= 3 {
                return FileReturnStatus::TryLater;
            }
            cnt += 1;
        }
        hasher.update(bytes.as_ref());

        // Final Hash
        hash = format!("{:X}", hasher.finalize());
        match &file.hash {
            sharedtypes::HashesSupported::None => {
                boolloop = false;
                // panic!("DlFileNew: Cannot parse hash info : {:?}", &file);
            }
            _ => {
                // Check and compare  to what the scraper wants
                let status = hash_bytes(&bytes, &file.hash);

                // Logging
                if !status.1 {
                    error!(
                        "Worker: {workerid} JobID: {jobid} -- Parser file: {} FAILED HASHCHECK: {} {}",
                        &file.hash, status.0, status.1
                    );
                    cnt += 1;
                } else {
                    // dbg!("Parser returned: {} Got: {}", &file.hash, status.0);
                }
                if cnt >= 3 {
                    return FileReturnStatus::TryLater;
                }
                boolloop = !status.1;
            }
        };
    }

    let file_ext = FileFormat::from_bytes(&bytes).extension().to_string();
    // If the plugin manager is None then don't do anything plugin wise. Useful for if
    // doing something that we CANNOT allow plugins to run.
    {
        if let Some(globalload) = globalload {
            crate::globalload::plugin_on_download(
                globalload,
                db.clone(),
                bytes.as_ref(),
                &hash,
                &file_ext,
            );
        }
    }
    {
        let mut unwrappydb = db.lock().unwrap();
        let source_url_ns_id = unwrappydb.create_default_source_url_ns_id();
        unwrappydb.enclave_determine_processing(file, bytes, &hash, source_url);
    }
    FileReturnStatus::File((hash, file_ext))
}

/// Hashes file from location string with specified hash into the hash of the file.
pub fn hash_file(
    filename: &String,
    hash: &sharedtypes::HashesSupported,
) -> Result<(String, Bytes), Box<dyn std::error::Error>> {
    let f = File::open(filename)?;
    let mut reader = BufReader::new(f);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let b = Bytes::from(buf);
    let hash_self = hash_bytes(&b, hash);
    Ok((hash_self.0, b))
}

pub fn write_to_disk(
    location: std::path::PathBuf,
    file: &sharedtypes::FileObject,
    bytes: &Bytes,
    sha512hash: &String,
) {
    let mut local_location = location.clone();

    // Adds directory name back into the full path
    if local_location.is_dir() && sha512hash.len() > 6 {
        let sha512hash_str = sha512hash.as_str();
        local_location = local_location.join(&sha512hash_str[0..2]);
        local_location = local_location.join(&sha512hash_str[2..4]);
        local_location = local_location.join(&sha512hash_str[4..6]);
        match std::fs::create_dir_all(&local_location) {
            Ok(_) => {}
            Err(err) => {
                logging::error_log(&format!("{} {}", sha512hash_str, err));
            }
        }

        local_location = local_location.join("FILENAMEFILLER");
    }

    // Gives file extension
    //let file_ext = FileFormat::from_bytes(bytes).extension().to_string();

    local_location.set_file_name(sha512hash);
    //local_location.set_extension(file_ext);

    let file_path_res = std::fs::File::create(&local_location);

    while file_path_res.is_err() {
        logging::info_log(&format!(
            "Cannot create file at path: {} Err: {:?}",
            &local_location.to_string_lossy(),
            file_path_res
        ));
        thread::sleep(Duration::from_secs(1));
    }

    if let Ok(mut file_path) = file_path_res {
        // Creates a content wrapper for bytes object
        let mut content = Cursor::new(bytes);

        // Copies file from memory to disk
        while let Err(err) = std::io::copy(&mut content, &mut file_path) {
            logging::info_log(&format!(
                "Cannot copy file at path: {} Err: {:?}",
                &location.to_string_lossy(),
                err
            ));

            thread::sleep(Duration::from_secs(1));
        }
        logging::info_log(&format!("Downloaded hash: {}", &sha512hash));
    }
}
