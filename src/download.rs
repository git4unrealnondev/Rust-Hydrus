use crate::Main;
use crate::downloadlogic::LocalStorage;
use crate::logging::error_log;

// extern crate urlparse;
use crate::logging;
use crate::logging::info_log;
use bytes::Bytes;
use core::time;
use file_format::FileFormat;
use log::{error, info};
use reqwest::Client;
use reqwest::ClientBuilder;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;
use sha2::Digest as sha2Digest;
use sha2::Sha256;
use sha2::Sha512;
use sharedtypes;
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

use crate::RwLock;
use crate::ui::ui::*;
use std::sync::Arc;
use std::thread;

/// Makes ratelimiter and example
pub fn ratelimiter_create(
    workerid: &u64,
    jobid: &u64,
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
pub fn ratelimiter_wait(ratelimit_object: &Arc<Ratelimiter>) {
    loop {
        let limit;
        {
            limit = ratelimit_object.try_wait();
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
                client = client.timeout(timeout.unwrap_or(Duration::from_secs(0)));
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
        let mut client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(100)
            //.cookie_provider(jar.into())
            .cookie_store(false)
            .user_agent(&useragent)
            .gzip(true)
            //            .brotli(true)
            .deflate(true)
            //        .zstd(true)
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
    client: Arc<Client>,
    ratelimiter_obj: &Arc<Ratelimiter>,
    worker_id: &u64,
    job_id: &u64,
) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
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
            "Worker: {} JobId: {} -- Spawned web reach to: {}",
            worker_id, job_id, &url_string
        ));

        let futureresult = match post_data {
            None => client.get(url).header("Accept", "text/css").send(),
            Some(ref post_data_string) => client.post(url).body(post_data_string.clone()).send(),
        }
        .await;

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
                            "Worker: {} JobId: {} -- While processing job {:?} was unable to download text. Had err {:?} sleeping for {} seconds.",
                            &worker_id, &job_id, &url_string, err, time_secs
                        ));

                        continue;
                    }
                    return Err(Box::new(err));
                } else {
                    let res_url = res.url().to_string();
                    match res.text().await {
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
                        "Worker: {} JobId: {} -- While processing job {:?} was unable to download text. Had err {:?} sleeping for {} seconds.",
                        &worker_id, &job_id, &url_string, err, time_secs
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
) -> Vec<(Vec<u8>, Vec<sharedtypes::FileTagAction>)> {
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
) -> Vec<(Vec<u8>, Vec<sharedtypes::FileTagAction>)> {
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
                        out.push((
                            filetemp,
                            vec![sharedtypes::FileTagAction {
                                operation: sharedtypes::TagOperation::Add,
                                tags,
                            }],
                        ));
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
    File((String, String, u64)),
    // Other issue. Try again later
    TryLater,
}

/// Downloads file to position asynchronously
pub async fn dlfile_new(
    client: Arc<reqwest::Client>,
    file: &mut sharedtypes::FileObjectMain,
    source_url: &String,
    workerid: &u64,
    jobid: &u64,
    scraper: Option<&sharedtypes::GlobalPluginScraper>,
    ctx: Arc<LocalStorage>,
    ratelimiter_obj: &Arc<Ratelimiter>,
    file_storage: Option<FileStorage>,
) -> FileReturnStatus {
    let mut boolloop = true;
    let mut hash = String::new();
    let mut bytes: Option<bytes::Bytes> = None;
    let throttle_duration = tokio::time::Duration::from_millis(200); // 5 times a second max per file
    let mut last_ui_update = tokio::time::Instant::now();
    let should_scraper_download = match scraper {
        Some(scraper) => scraper.should_handle_file_download,
        None => false,
    };

    if should_scraper_download && let Some(scraper) = scraper {
        match ctx.globalload.download_from(file.clone(), scraper) {
            None => {
                logging::log(format!("Could not pull info for file {:?}", &file));
            }
            Some(filebytes) => {
                bytes = Some(Bytes::from(filebytes));
            }
        }
    } else {
        let mut cnt = 0;
        while boolloop {
            let mut hasher = Sha512::new();

            let mut response = loop {
                if cnt >= 3 {
                    return FileReturnStatus::TryLater;
                }
                let _fileurlmatch = match &file.source {
                    None => {
                        panic!(
                            "Tried to call dlfilenew when there was no file :C info: {:?}",
                            file
                        );
                    }
                    Some(fileurl) => fileurl,
                };
                let url = Url::parse(&source_url);
                if url.is_err() {
                    error_log(format!("Error while parsing url {} {:?}", source_url, url));
                    return FileReturnStatus::DeadUrl(source_url.to_string());
                }
                let url = url.unwrap();

                // FIX: Offload synchronous ratelimiter to a blocking thread pool safely
                let limiter = Arc::clone(ratelimiter_obj);
                tokio::task::spawn_blocking(move || {
                    ratelimiter_wait(&limiter);
                })
                .await
                .unwrap();

                logging::info_log(format!("Downloading: {}", &source_url));

                // Assuming post_data logic exists based on your compiler error snippet
                let response_result = { client.get(url.as_ref()).send().await };

                // FIX: Check errors against our evaluated result variable
                if cnt >= 3 && response_result.is_err() {
                    return FileReturnStatus::TryLater;
                }

                match response_result {
                    Ok(res) => {
                        if let Err(err) = res.error_for_status_ref() {
                            if let Some(status) = err.status() {
                                if status.is_server_error() {
                                    logging::error_log(&format!(
                                        "Worker: {workerid} JobID: {jobid} -- Repeating job due to server err {:?} url: {}",
                                        err,
                                        &url.to_string()
                                    ));
                                    tokio::time::sleep(Duration::from_secs(10)).await;
                                    cnt += 1;
                                    continue;
                                }
                                if status.is_client_error() {
                                    logging::error_log(&format!(
                                        "Worker: {workerid} JobID: {jobid} -- Stopping file download due to: {:?}",
                                        err
                                    ));
                                    return FileReturnStatus::DeadUrl(source_url.clone());
                                }
                            }
                        }
                        break res;
                    }
                    Err(_) => {
                        error!("Worker: {workerid} JobID: {jobid} -- Repeating: {}", &url);
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        cnt += 1;
                    }
                }
            };

            // === ASYNC CHUNK PROGRESS REWRITE ===
            let total_size = response.content_length().unwrap_or(0);
            let mut downloaded: u64 = 0;
            let mut collected_bytes = bytes::BytesMut::with_capacity(if total_size > 0 {
                total_size as usize
            } else {
                1024
            });

            let mut last_reported_progress: f64 = -1.0;
            let mut download_success = true;

            // FIX: Uses Reqwest's built-in async stream loops directly, removing type inference bugs
            while let Ok(Some(chunk)) = response.chunk().await {
                let bytes_read = chunk.len();
                collected_bytes.extend_from_slice(&chunk);
                downloaded += bytes_read as u64;

                let current_progress = if total_size > 0 {
                    (downloaded as f64 / total_size as f64) * 100.0
                } else {
                    (downloaded as f64) / (1024.0 * 1024.0)
                };

                if (current_progress - last_reported_progress).abs() >= 0.1 {
                    if let Some(mut file_storage) = file_storage.clone() {
                        let now = tokio::time::Instant::now();
                        if now.duration_since(last_ui_update) >= throttle_duration
                            || current_progress >= 100.0
                        {
                            file_storage.status = FilesStatus::Downloading(current_progress);
                            ctx.update_file(workerid, jobid, &file_storage);
                            last_ui_update = now;
                        }
                    }
                    /*  {
                        let mut list_guard = file_ui_list.write();
                        if let Some(target_file) = list_guard
                            .iter_mut()
                            .find(|f| f.internal_id == file_ui.internal_id)
                        {
                            target_file.status = FilesStatus::Downloading(display_progress);
                        }
                    }*/

                    last_reported_progress = current_progress;
                }
            }

            if !download_success {
                continue;
            }

            bytes = Some(collected_bytes.freeze());
            hasher.update(bytes.as_ref().unwrap().as_ref());

            // Final Hash
            hash = format!("{:X}", hasher.finalize());
            match &file.hash {
                sharedtypes::HashesSupported::None => {
                    boolloop = false;
                }
                _ => {
                    let status = hash_bytes(bytes.as_ref().unwrap(), &file.hash);

                    if !status.1 {
                        error!(
                            "Worker: {workerid} JobID: {jobid} -- Parser file: {:?} FAILED HASHCHECK: {} {}",
                            &file.hash, status.0, status.1
                        );
                        cnt += 1;
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

    if let Some(ref mut file_storage) = file_storage.clone() {
        file_storage.status = FilesStatus::Processing(0.0);

        ctx.update_file(workerid, jobid, &file_storage)
    }

    /*  {
        let mut list_guard = file_ui_list.write();
        if let Some(target_file) = list_guard
            .iter_mut()
            .find(|f| f.internal_id == file_ui.internal_id)
        {
            target_file.status = FilesStatus::Processing(0.0);
        }
    }*/

    if let Some(ref bytes) = bytes {
        let file_ext = FileFormat::from_bytes(bytes).extension().to_string();

        let file_id = process_bytes(bytes, &hash, &file_ext, file, Some(source_url), ctx.clone());

        if let Some(mut file_storage) = file_storage.clone() {
            file_storage.status = FilesStatus::Done;
            ctx.update_file(workerid, jobid, &file_storage);
        }
        /* {
            let mut list_guard = file_ui_list.write();
            if let Some(target_file) = list_guard
                .iter_mut()
                .find(|f| f.internal_id == file_ui.internal_id)
            {
                target_file.status = FilesStatus::Done;
            }
        }*/

        return FileReturnStatus::File((hash, file_ext, file_id.unwrap()));
    }

    FileReturnStatus::TryLater
}

///
/// Runs external bytes processing and starts enclave work
///
pub fn process_bytes(
    bytes: &Bytes,
    hash: &String,
    file_ext: &String,
    file: &mut sharedtypes::FileObjectMain,
    source_url: Option<&String>,
    ctx: Arc<LocalStorage>,
) -> Option<u64> {
    /* {
        let mut list_guard = file_ui_list.write();
        if let Some(target) = list_guard
            .iter_mut()
            .find(|f| f.internal_id == file_ui.internal_id)
        {
            target.status = FilesStatus::Processing(0.0);
        }
    }*/

    let mut out = None;
    // NOTE run the download / file actions first then run the plugin_on_download second.
    // That way if theirs any data that needs to get processed then we can do it while theirs a
    // valid file hash inside of the db
    {
        if let Some(file_id) = ctx
            .db
            .enclave_determine_processing(file, bytes, hash, source_url)
        {
            out = Some(file_id);
        }
    }

    //logging::info_log("Finished enclave_determine_processing".to_string());
    // Flushes to disk before we run the plugins on_download hook.

    // If the plugin manager is None then don't do anything plugin wise. Useful for if
    // doing something that we CANNOT allow plugins to run.
    {
        ctx.globalload
            .plugin_on_download(ctx.db.clone(), bytes, hash, file_ext);
    }

    ctx.db.add_tags_to_fileid(out, &file.tag_list);
    /* {
        let mut list_guard = file_ui_list.write();
        if let Some(target) = list_guard
            .iter_mut()
            .find(|f| f.internal_id == file_ui.internal_id)
        {
            target.status = FilesStatus::Processing(100.0);
        }
    }*/
    out
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
    //file: &sharedtypes::FileObjectMain,
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
///
/// Creates a relelimiter object
pub fn create_ratelimiter(
    input: (u64, Duration),
    worker_id: &u64,
    job_id: &u64,
) -> Arc<Ratelimiter> {
    Arc::new(ratelimiter_create(worker_id, job_id, input.0, input.1))
}

/// Parses weather we should skip downloading the file
/// Returns a Some(u64) if the fileid exists
pub fn parse_skipif(
    skip_condition: &sharedtypes::SkipIf,
    file_url_source: &String,
    database: Main,
    worker_id: &u64,
    job_id: &u64,
    ctx: &Arc<LocalStorage>,
) -> Option<u64> {
    match skip_condition {
        sharedtypes::SkipIf::NoFilesDownloaded => {}
        sharedtypes::SkipIf::FileHash(sha512hash) => {
            return ctx.db.file_get_hash(sha512hash);
        }
        sharedtypes::SkipIf::FileNamespaceNumber((unique_tag, namespace_filter, filter_number)) => {
            let mut cnt = 0;
            let fids;
            if let Some(nidf) = &ctx.db.namespace_get(&namespace_filter.name)
                && let Some(nid) = ctx.db.namespace_get(&unique_tag.namespace.name)
                && let Some(tid) = &ctx.db.tag_get_name(unique_tag.tag.clone(), nid)
            {
                fids = ctx.db.relationship_get_fileid(tid);
                if fids.len() == 1 {
                    let fid = fids.iter().next().unwrap();
                    for tidtofilter in ctx.db.relationship_get_tagid(fid).iter() {
                        if ctx.db.namespace_contains_id(nidf, tidtofilter) {
                            //if ctx.db.namespace_contains_id(nidf, tidtofilter) {
                            cnt += 1;
                        }
                    }
                }
            } else {
                return None;
            }
            if cnt > *filter_number {
                info_log(format!(
                    "Not downloading because unique namespace is greater then limit number. {}",
                    unique_tag.tag
                ));
            } else {
                info_log(
                    "Downloading due to unique namespace not existing or number less then limit number.".to_string(),
                );
                let vec: Vec<u64> = fids.iter().cloned().collect();
                return Some(vec[0]);
            }
        }
        sharedtypes::SkipIf::FileTagRelationship(tag) => {
            if let Some(nsid) = ctx.db.namespace_get(&tag.namespace.name)
                && ctx.db.tag_get_name(tag.tag.to_string(), nsid).is_some()
            {
                info_log(format!(
                    "Worker: {worker_id} JobId: {job_id} -- Skipping file: {} Due to skip tag {} already existing in Tags Table.",
                    file_url_source, tag.tag
                ));
                if let Some(tid) = ctx.db.tag_get_name(tag.tag.to_string(), nsid) {
                    return ctx.db.relationship_get_one_fileid(&tid);
                }
            }
        }
    }
    None
}

/// Main file checking loop manages the downloads
pub async fn main_file_loop(
    file: &mut sharedtypes::FileObjectMain,
    client: Arc<Client>,
    scraper: &sharedtypes::GlobalPluginScraper,
    worker_id: &u64,
    job_id: &u64,
    ctx: Arc<LocalStorage>,
    ratelimiter_obj: &Arc<Ratelimiter>,
    file_storage: Option<FileStorage>,
) {
    let mut fileid = None;
    //let task_id = file_storage.internal_id; // Unique identifier for tracking logs

    let source_url_id = ctx.db.create_default_source_url_ns_id();

    match file.source.clone() {
        Some(source) => match source {
            sharedtypes::FileSource::Url(source_url) => {
                let skipif_start = std::time::Instant::now();
                for file_tag in file.skip_if.iter() {
                    if let Some(file_id) = parse_skipif(
                        file_tag,
                        &source_url,
                        ctx.db.clone(),
                        worker_id,
                        job_id,
                        &ctx.clone(),
                    ) {
                        if let Some(file_storage) = file_storage {
                            let mut file_storage = file_storage.clone();
                            file_storage.status = FilesStatus::Done;
                            ctx.update_file(worker_id, job_id, &file_storage);
                        }
                        ctx.db.add_tags_to_fileid(Some(file_id), &file.tag_list);
                        return;
                    }
                }
                let location = ctx.db.location_get();
                let url_tag = ctx.db.tag_get_name(source_url.clone(), source_url_id);
                //
                // === FIX: Clone our shared list handle for safely escaping the iteration block ===
                //let loop_ui_list_clone = Arc::clone(&file_ui_list);

                fileid = match url_tag {
                    None => {
                        match download_add_to_db(
                            &source_url,
                            location,
                            client,
                            file,
                            worker_id,
                            job_id,
                            scraper,
                            ctx,
                            ratelimiter_obj,
                            file_storage,
                        )
                        .await
                        {
                            None => {
                                return;
                            }
                            Some(out) => Some(out),
                        }
                    }
                    Some(url_id) => {
                        let file_id = ctx.db.relationship_get_one_fileid(&url_id);

                        match file_id {
                            Some(f_id) => {
                                // Updates UI to show that the file is already finished
                                if let Some(mut file_storage) = file_storage.clone() {
                                    file_storage.status = FilesStatus::Done;
                                    ctx.update_file(worker_id, job_id, &file_storage);
                                }
                                info_log(format!(
                                    "Worker: {worker_id} JobId: {job_id} -- Skipping file: {} Due to already existing in Tags Table.",
                                    &source_url
                                ));
                                Some(f_id)
                            }
                            None => {
                                match download_add_to_db(
                                    &source_url,
                                    location,
                                    client,
                                    file,
                                    worker_id,
                                    job_id,
                                    scraper,
                                    ctx,
                                    &ratelimiter_obj.clone(),
                                    file_storage,
                                )
                                .await
                                {
                                    None => return,
                                    Some(id) => Some(id),
                                }
                            }
                        }
                    }
                };
            }
            sharedtypes::FileSource::Bytes(bytes) => {
                let bytes = &bytes::Bytes::from(bytes);
                let file_ext = FileFormat::from_bytes(bytes).extension().to_string();
                let sha512 = hash_bytes(bytes, &sharedtypes::HashesSupported::Sha512("".into()));

                process_bytes(bytes, &sha512.0, &file_ext, file, None, ctx.clone());

                fileid = ctx.db.file_get_hash(&sha512.0);
                ctx.db.add_tags_to_fileid(fileid, &file.tag_list);
            }
        },
        None => {
            // Has a file but no source???
            if let Some(mut file_storage) = file_storage.clone() {
                file_storage.status = FilesStatus::Done;
                ctx.update_file(worker_id, job_id, &file_storage);
            }
            return;
        }
    }
}

///
/// Downloads a file into the db if needed
///
async fn download_add_to_db(
    source: &String,
    _location: String,
    client: Arc<Client>,
    file: &mut sharedtypes::FileObjectMain,
    worker_id: &u64,
    job_id: &u64,
    scraper: &sharedtypes::GlobalPluginScraper,
    ctx: Arc<LocalStorage>,
    ratelimiter_obj: &Arc<Ratelimiter>,
    file_storage: Option<FileStorage>,
) -> Option<u64> {
    // Early exit for if the file is a dead url
    {
        if ctx.db.check_dead_url(source) {
            logging::info_log(format!(
                "Worker: {worker_id} JobID: {job_id} -- Skipping {} because it's a dead link.",
                source
            ));
            return None;
        }
    }

    let blopt;
    {
        //let mut_client = &mut client.write();

        // Download file doesn't exist. URL doesn't exist in DB Will download
        blopt = dlfile_new(
            client,
            file,
            &source,
            worker_id,
            job_id,
            Some(scraper),
            ctx.clone(),
            &ratelimiter_obj,
            file_storage,
        )
        .await;
    }

    match blopt {
        FileReturnStatus::File((_hash, _file_ext, file_id)) => {
            return Some(file_id);
        }
        FileReturnStatus::DeadUrl(dead_url) => {
            ctx.db.add_dead_url(&dead_url);
        }
        _ => {}
    }

    None
}
