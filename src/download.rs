use crate::database::database::Main;
use crate::globalload;
use crate::globalload::GlobalLoad;
use crate::logging::error_log;

// extern crate urlparse;
use super::sharedtypes;
use crate::logging;
use bytes::Bytes;
use core::time;
use file_format::FileFormat;
use log::{error, info};
use reqwest::blocking::Client;
use reqwest::blocking::ClientBuilder;
use reqwest::cookie;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;
use sha2::Digest as sha2Digest;
use sha2::Sha256;
use sha2::Sha512;
use std::error::Error;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::str::FromStr;
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
    logging::info_log(format!(
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
                logging::error_log(format!("Failed to make ratelimiter with err: {:?}", err));
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

///
/// Extracts any modifiers from a pluginscraper
///
pub fn get_modifiers(
    scraper: &sharedtypes::GlobalPluginScraper,
) -> Vec<sharedtypes::TargetModifiers> {
    let mut out = Vec::new();

    if let Some(scrapertype) = &scraper.storage_type
        && let sharedtypes::ScraperOrPlugin::Scraper(scraper) = scrapertype
    {
        for modifier in &scraper.modifiers {
            out.push(modifier.clone());
        }
    }

    out
}

///
/// Waits a bit for process control
///
pub fn ratelimiter_wait(ratelimit_object: &Arc<RwLock<Ratelimiter>>) {
    loop {
        let limit;
        {
            let hold = ratelimit_object.read();
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

fn process_modifiers(
    client: ClientBuilder,
    target: Vec<sharedtypes::TargetModifiers>,
    is_text_download: bool,
) -> ClientBuilder {
    let mut client = client;
    for modifer in target {
        let is_text_modifier = modifer.target == sharedtypes::ModifierTarget::Text;
        if is_text_modifier != is_text_download {
            continue;
        }
        match modifer.modifier {
            sharedtypes::ScraperModifiers::Header((key, val)) => {
                let key = key.clone();
                let val = val.clone();
                let mut headers = HeaderMap::new();
                let header_key = HeaderName::from_str(&key).unwrap();
                let header_val = HeaderValue::from_str(&val).unwrap();
                headers.insert(header_key, header_val);
                client = client.default_headers(headers);
            }
            sharedtypes::ScraperModifiers::Useragent(useragent) => {
                client = client.user_agent(useragent);
            }
            sharedtypes::ScraperModifiers::Timeout(timeout) => {
                client = client.timeout(time::Duration::from_secs(timeout));
            }
        }
    }
    client
}

/// Creates Client that the downloader will use.
pub fn client_create(
    modifers: Vec<sharedtypes::TargetModifiers>,
    is_text_download: bool,
) -> Client {
    let useragent = "RustHydrus V1.0".to_string();
    // let useragent =
    //     "User-Agent Mozilla/5.0 (X11; Linux x86_64; rv:141.0) Gecko/20100101 Firefox/141.0"
    //         .to_string();

    // let jar = cookie::Jar::default();

    loop {
        // The client that does the downloading
        let mut client = reqwest::blocking::ClientBuilder::new()
            //.cookie_provider(jar.into())
            .cookie_store(false)
            .user_agent(&useragent)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .zstd(true)
            .connect_timeout(time::Duration::from_secs(15))
            .timeout(time::Duration::from_secs(120));

        client = process_modifiers(client, modifers.clone(), is_text_download);

        match client.build() {
            Ok(out) => {
                return out;
            }
            Err(_) => {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

/// Downloads text into db as responses. Filters responses by default limit if
/// their's anything wrong with request.
pub async fn dltext_new(
    url_string: &String,
    post_data: Option<String>,
    client: Arc<RwLock<Client>>,
    ratelimiter_obj: &Arc<RwLock<Ratelimiter>>,
    worker_id: &usize,
) -> Result<(String, String), Box<dyn Error>> {
    // let mut ret: Vec<AHashMap<String, AHashMap<String, Vec`<String>`>>> =
    // Vec::new(); let ex = Executor::new(); let url =
    // Url::parse("http://www.google.com").unwrap();
    let mut cnt = 0;
    loop {
        let url = match Url::parse(url_string) {
            Ok(out) => out,
            Err(err) => {
                logging::error_log(format!(
                    "Failed to parse URL: {} with error {:?}",
                    url_string, err
                ));
                return Err(Box::new(err));
            }
        };

        // let futureresult = futures::executor::block_on(ratelimit_object.ready())
        // .unwrap()
        ratelimiter_wait(ratelimiter_obj);
        logging::info_log(format!(
            "Worker: {} -- Spawned web reach to: {}",
            worker_id, &url_string
        ));

        let futureresult = match post_data {
            None => client.read().get(url).header("Accept", "text/css").send(),
            Some(ref post_data_string) => client
                .read()
                .post(url)
                .body(post_data_string.clone())
                .header("Origin", "https://furry34.com")
                .header("Content-Type", "application/json")
                .send(),
        };

        // let test = reqwest::get(url).await.unwrap().text(); let futurez =
        // futures::executor::block_on(futureresult); dbg!(&futureresult);
        if cnt >= 3 && futureresult.is_err() {
            if let Some(out) = futureresult.err() {
                return Err(Box::new(out));
            } else {
                // Better error parsing
                error_log(format!(
                    "While parsing text for {url_string} we had a goofy error: We had an error with type: None",
                ));
                return Err(Box::new(url::ParseError::Overflow));
            }
        }
        match futureresult {
            Ok(res) => {
                // Exit for error codes 400
                if let Err(err) = res.error_for_status_ref() {
                    if err.is_timeout() {
                        let time_secs = 5;
                        std::thread::sleep(std::time::Duration::from_secs(time_secs));
                        logging::error_log(format!(
                            "Worker: {} -- While processing job {:?} was unable to download text. Had err {:?} sleeping for {} seconds.",
                            &worker_id, &url_string, err, time_secs
                        ));

                        continue;
                    }
                    return Err(Box::new(err));
                } else {
                    let res_url = res.url().to_string();
                    match res.text() {
                        Ok(text) => {
                            return Ok((text, res_url));
                        }
                        Err(_) => {
                            cnt += 1;
                            continue;
                        }
                    }
                }
            }
            Err(err) => {
                if err.is_timeout() {
                    let time_secs = 5;
                    std::thread::sleep(std::time::Duration::from_secs(time_secs));
                    logging::error_log(format!(
                        "Worker: {} -- While processing job {:?} was unable to download text. Had err {:?} sleeping for {} seconds.",
                        &worker_id, &url_string, err, time_secs
                    ));

                    cnt += 1;
                    continue;
                }
                // return Err(Box::new(futureresult.err().unwrap()));
            }
        }
        cnt += 1;
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
            let hasher = Sha512::digest(bytes);
            let hastring = format!("{:X}", hasher);
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
/// Processes archive files
/// Returns a list of files and tags associated with the files
/// does not search for sidecars
///
pub fn process_archive_files(
    inp_bytes: Cursor<Bytes>,
    filetype: Option<FileFormat>,
    linkto: sharedtypes::SubTag,
) -> Vec<(Vec<u8>, Vec<sharedtypes::TagObject>)> {
    if let Some(filetype) = filetype
        && filetype == FileFormat::Zip
    {
        return process_archive_zip(inp_bytes, linkto);
    }
    Vec::new()
}

///
/// Processes a zip file
///
fn process_archive_zip(
    inp_bytes: Cursor<Bytes>,
    linkto: sharedtypes::SubTag,
) -> Vec<(Vec<u8>, Vec<sharedtypes::TagObject>)> {
    let mut out = Vec::new();
    if let Ok(mut zip) = zip::ZipArchive::new(inp_bytes) {
        for item in 0..zip.len() {
            if let Ok(mut file) = zip.by_index(item) {
                let file_comment = file.comment();
                if !file_comment.is_empty() {
                    dbg!(&file_comment);
                }
                if file.is_file() {
                    let mut tags = Vec::new();

                    if !file_comment.is_empty() {
                        tags.push(sharedtypes::TagObject {
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: "SYSTEM_ARCHIVE_ZIP_FILE_COMMENT".to_string(),
                                description: Some(
                                    "A comment for a file inside of a zip archive.".to_string(),
                                ),
                            },
                            tag: file_comment.to_string(),
                            tag_type: sharedtypes::TagType::Normal,
                            relates_to: None,
                        });
                    }
                    tags.push(sharedtypes::TagObject {
                            namespace: sharedtypes::GenericNamespaceObj {
                                name: "SYSTEM_ARCHIVE_ZIP_FILE_PATH".to_string(),
                                description: Some(
                                    "A full filepath, includes name for a file inside of a zip archive.".to_string(),
                                ),
                            },
                            tag: file.name().to_string(),
                            tag_type: sharedtypes::TagType::Normal,
                            relates_to: Some(linkto.clone()),
                        });

                    let mut filetemp = Vec::new();
                    if std::io::copy(&mut file, &mut filetemp).is_ok() {
                        out.push((filetemp, tags));
                    }
                }
            }
        }
    }
    out
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
    client: Arc<RwLock<Client>>,
    db: Main,
    file: &mut sharedtypes::FileObject,
    globalload: Option<GlobalLoad>,
    ratelimiter_obj: &Arc<RwLock<Ratelimiter>>,
    source_url: &String,
    workerid: &usize,
    jobid: &usize,
    scraper: Option<&sharedtypes::GlobalPluginScraper>,
) -> FileReturnStatus {
    let mut boolloop = true;
    let mut hash = String::new();
    //let mut bytes: bytes::Bytes = Bytes::from(&b""[..]);
    let mut bytes: Option<bytes::Bytes> = None;
    let mut cnt = 0;

    let should_scraper_download = match scraper {
        Some(scraper) => scraper.should_handle_file_download,
        None => false,
    };

    if should_scraper_download {
        if let Some(ref globalload) = globalload
            && let Some(scraper) = scraper
        {
            match globalload.download_from(file, scraper) {
                None => {
                    logging::log(format!("Could not pull info for file {:?}", &file));
                }
                Some(filebytes) => {
                    bytes = Some(Bytes::from(filebytes));
                }
            }
        }
        //return FileReturnStatus::TryLater;
    } else {
        while boolloop {
            let mut hasher = Sha512::new();
            loop {
                let fileurlmatch = match &file.source {
                    None => {
                        panic!(
                            "Tried to call dlfilenew when their was no file :C info: {:?}",
                            file
                        );
                    }
                    Some(fileurl) => fileurl,
                };
                let url = Url::parse(source_url);
                if url.is_err() {
                    error_log(format!("Error while parsing url {} {:?}", source_url, url));
                    return FileReturnStatus::DeadUrl(source_url.to_string());
                }
                let url = url.unwrap();
                ratelimiter_wait(ratelimiter_obj);
                logging::info_log(format!("Downloading: {}", &source_url));
                let mut futureresult = {
                    let client = client.read();
                    client.get(url.as_ref()).send()
                };
                loop {
                    match &futureresult {
                        Ok(result) => {
                            /*if let Err(err) = result.error_for_status_ref() {
                                match ddos_guard_bypass(result, client, source_url) {
                                    Some(bypass_response) => futureresult = Ok(bypass_response),
                                    None => {
                                        if let Some(err_status) = err.status() {
                                            if err_status.is_client_error() {
                                                logging::error_log(&format!(
                                        "Worker: {workerid} JobID: {jobid} -- Stopping file download due to: {:?}",
                                        err
                                    ));
                                                return FileReturnStatus::DeadUrl(
                                                    source_url.clone(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }*/
                            break;
                        }
                        Err(_) => {
                            error!("Worker: {workerid} JobID: {jobid} -- Repeating: {}", &url);
                            dbg!("Worker: {workerid} JobID: {jobid} -- Repeating: {}", &url);
                            let time_dur = Duration::from_secs(10);
                            thread::sleep(time_dur);
                            futureresult = {
                                let client = client.read();
                                client.get(url.as_ref()).send()
                            };
                        }
                    }
                }

                // Downloads file into byte memory buffer
                let byte = futureresult.unwrap().bytes();
                // Error handling for dling a file. Waits 10 secs to retry
                match byte {
                    Ok(out) => {
                        bytes = Some(out);

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

            hasher.update(bytes.as_ref().unwrap().as_ref());

            // Final Hash
            hash = format!("{:X}", hasher.finalize());
            match &file.hash {
                sharedtypes::HashesSupported::None => {
                    boolloop = false;
                    // panic!("DlFileNew: Cannot parse hash info : {:?}", &file);
                }
                _ => {
                    // Check and compare  to what the scraper wants
                    let status = hash_bytes(bytes.as_ref().unwrap(), &file.hash);

                    // Logging
                    if !status.1 {
                        error!(
                            "Worker: {workerid} JobID: {jobid} -- Parser file: {:?} FAILED HASHCHECK: {} {}",
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
    }
    logging::info_log(format!("Downloaded hash: {}", &hash));

    if let Some(ref bytes) = bytes {
        let file_ext = FileFormat::from_bytes(bytes).extension().to_string();
        process_bytes(
            bytes,
            globalload,
            &hash,
            &file_ext,
            db,
            file,
            Some(source_url),
        );
        logging::info_log(format!("Finished processing bytes"));
        return FileReturnStatus::File((hash, file_ext));
    }

    // If we don't donwload anything the default to try again later
    FileReturnStatus::TryLater
}

///
/// Runs external bytes processing and starts enclave work
///
pub fn process_bytes(
    bytes: &Bytes,
    globalload: Option<GlobalLoad>,
    hash: &String,
    file_ext: &String,
    db: Main,
    file: &mut sharedtypes::FileObject,
    source_url: Option<&String>,
) {
    // NOTE run the download / file actions first then run the plugin_on_download second.
    // That way if theirs any data that needs to get processed then we can do it while theirs a
    // valid file hash inside of the db
    {
        let enclave_id_list;
        {
            enclave_id_list = db.enclave_determine_processing(file, bytes, hash, source_url);
        }
    }

    logging::info_log(format!("Finished enclave_determine_processing"));
    // Flushes to disk before we run the plugins on_download hook.

    // If the plugin manager is None then don't do anything plugin wise. Useful for if
    // doing something that we CANNOT allow plugins to run.
    {
        if let Some(globalload) = globalload {
            globalload.plugin_on_download(db.clone(), bytes, hash, file_ext);
        }
    }
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

///
/// Writes a file to disk
///
pub fn write_to_disk(
    location: std::path::PathBuf,
    //file: &sharedtypes::FileObject,
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
                logging::error_log(format!("{} {}", sha512hash_str, err));
            }
        }

        local_location = local_location.join("FILENAMEFILLER");
    }

    // Gives file extension
    let file_ext = FileFormat::from_bytes(bytes).extension().to_string();

    local_location.set_file_name(sha512hash);
    local_location.set_extension(file_ext);

    // Proper error handleing for if we have an error while downloading
    let mut file_path;
    loop {
        let file_path_res = std::fs::File::create(&local_location);
        if let Ok(file_path_fin) = file_path_res {
            file_path = file_path_fin;
            break;
        } else {
            logging::info_log(format!(
                "Cannot create file at path: {} Err: {:?}",
                &local_location.to_string_lossy(),
                file_path_res
            ));
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Creates a content wrapper for bytes object
    let mut content = Cursor::new(bytes);

    // Copies file from memory to disk
    while let Err(err) = std::io::copy(&mut content, &mut file_path) {
        logging::info_log(format!(
            "Cannot copy file at path: {} Err: {:?}",
            &location.to_string_lossy(),
            err
        ));

        thread::sleep(Duration::from_secs(1));
    }
}
