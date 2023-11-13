use super::plugins::PluginManager;
//extern crate urlparse;
use super::database;
use super::sharedtypes;
use crate::file;
use bytes::Bytes;
use file_format::FileFormat;
use log::{error, info};
use md5;
use reqwest::Client;
use sha1;
use sha2::Digest as sha2Digest;
use sha2::Sha512;
use std::fs;
use std::io;
use std::io::Cursor;
use std::time::Duration;
use url::Url;
extern crate reqwest;
use async_std::task;
use ratelimit::Ratelimiter;
use std::sync::{Arc, Mutex};
use std::thread;

///
/// Makes ratelimiter and example
///
pub fn ratelimiter_create(number: u64, duration: Duration) -> Ratelimiter {
    println!(
        "Making ratelimiter with: {} Request Per: {:?}",
        &number, &duration
    );

    // The wrapper that implements ratelimiting
    Ratelimiter::builder(number, duration).build().unwrap()
}

pub fn ratelimiter_wait(ratelimit_object: &mut Ratelimiter) {
    let limit = ratelimit_object.try_wait();

    match limit {
        Ok(_) => {}
        Err(sleep) => {
            std::thread::sleep(sleep);
        }
    }
}

///
/// Creates Client that the downloader will use.
///
///
pub fn client_create() -> Client {
    let useragent = "RustHydrusV1".to_string();
    // The client that does the downloading
    reqwest::ClientBuilder::new()
        .user_agent(useragent)
        .cookie_store(true)
        //.brotli(true)
        //.deflate(true)
        .gzip(true)
        .build()
        .unwrap()
}

///
/// Downloads text into db as responses. Filters responses by default limit if their's anything wrong with request.
///
pub async fn dltext_new(
    url_string: String,
    ratelimit_object: &mut Ratelimiter,
    client: &mut Client,
    _pluginmanager: Arc<Mutex<PluginManager>>,
) -> Result<String, reqwest::Error> {
    //let mut ret: Vec<AHashMap<String, AHashMap<String, Vec<String>>>> = Vec::new();
    //let ex = Executor::new();

    let url = Url::parse(&url_string).unwrap();
    //let url = Url::parse("http://www.google.com").unwrap();

    //let requestit = Request::new(Method::GET, url);
    //fut.push();
    println!("Spawned web reach to: {}", &url_string);
    //let futureresult = futures::executor::block_on(ratelimit_object.ready())
    //    .unwrap()
    ratelimiter_wait(ratelimit_object);
    let futureresult = client.get(url).send().await;

    //let test = reqwest::get(url).await.unwrap().text();

    //let futurez = futures::executor::block_on(futureresult);
    //dbg!(&futureresult);

    match futureresult {
        Ok(_) => Ok(task::block_on(futureresult.unwrap().text()).unwrap()),
        Err(_) => Err(futureresult.err().unwrap()),
    }
}

pub async fn test(url: String) -> String {
    dbg!(url);
    "hi".to_string()
}

///
/// Hashes the bytes and compares it to what the scraper should of recieved.
///
pub fn hash_bytes(bytes: &Bytes, hash: sharedtypes::HashesSupported) -> (String, bool) {
    match hash {
        sharedtypes::HashesSupported::Md5(hash) => {
            let digest = md5::compute(bytes);
            //let sharedtypes::HashesSupported(hashe, _) => hash;
            (format!("{:x}", digest), format!("{:x}", digest) == hash)
        }
        sharedtypes::HashesSupported::Sha1(hash) => {
            let mut hasher = sha1::Sha1::new();
            hasher.update(bytes);
            let hastring = format!("{:X}", hasher.finalize());
            let dune = &hastring == &hash;
            (hastring, dune)
        }
        sharedtypes::HashesSupported::Sha256(hash) => {
            let mut hasher = Sha512::new();
            hasher.update(bytes);
            let hastring = format!("{:X}", hasher.finalize());
            let dune = hastring == hash;
            (hastring, dune)
        }
        sharedtypes::HashesSupported::None => ("".to_string(), false),
    }
}

///
/// Downloads file to position
///
pub async fn dlfile_new(
    client: &Client,
    file: &sharedtypes::FileObject,
    location: &String,
    pluginmanager: Arc<Mutex<PluginManager>>,
) -> (String, String) {
    let mut boolloop = true;
    let mut hash = String::new();
    let mut bytes: bytes::Bytes = Bytes::from(&b""[..]);
    while boolloop {
        let mut hasher = Sha512::new();

        let errloop = true;

        while errloop {
            let url = Url::parse(&file.source_url).unwrap();

            let mut futureresult = client.get(url.as_ref()).send().await;
            loop {
                match futureresult {
                    Ok(_) => {
                        break;
                    }
                    Err(_) => {
                        error!("Repeating: {}", &url);
                        dbg!("Repeating: {}", &url);
                        let time_dur = Duration::from_secs(10);
                        thread::sleep(time_dur);
                        futureresult = client.get(url.as_ref()).send().await;
                    }
                }
            }

            // Downloads file into byte memory buffer
            let byte = futureresult.unwrap().bytes().await;

            // Error handling for dling a file.
            // Waits 10 secs to retry
            match byte {
                Ok(_) => {
                    bytes = byte.unwrap();
                    break;
                }
                Err(_) => {
                    error!("Repeating: {} , Due to: {:?}", &url, &byte.as_ref().err());
                    dbg!("Repeating: {} , Due to: {:?}", &url, &byte.as_ref().err());
                    let time_dur = Duration::from_secs(10);
                    thread::sleep(time_dur);
                }
            }
        }

        hasher.update(&bytes.as_ref());

        // Final Hash
        hash = format!("{:X}", hasher.finalize());

        // Check and compare  to what the scraper wants
        let status = hash_bytes(&bytes, file.hash.clone());

        // Logging
        if !status.1 {
            error!(
                "Parser file: {} FAILED HASHCHECK: {} {}",
                file.hash, status.0, status.1
            )
        } else {
            info!("Parser returned: {} Got: {}", &file.hash, status.0);
            //dbg!("Parser returned: {} Got: {}", &file.hash, status.0);
        }
        boolloop = !status.1;
    }

    let final_loc = getfinpath(&location, &hash);

    // Gives file extension
    let file_ext = FileFormat::from_bytes(&bytes).extension().to_string();

    let mut content = Cursor::new(bytes.clone());

    // Gets final path of file.
    let orig_path = format!("{}/{}", &final_loc, &hash);
    let mut file_path = std::fs::File::create(&orig_path).unwrap();

    // Copies file from memory to disk
    std::io::copy(&mut content, &mut file_path).unwrap();

    // Wouldove rather passed teh Cursor or Bytes Obj but it wouldn't work for some reason with the ABI
    pluginmanager
        .lock()
        .unwrap()
        .plugin_on_download(bytes.as_ref(), &hash, &file_ext);

    println!("Downloaded hash: {}", &hash);

    (hash, file_ext)
}

pub fn hash_file(filename: &String) -> String {
    let mut hasher = Sha512::new();
    let mut file = fs::File::open(filename).unwrap();

    let _bytes_written = io::copy(&mut file, &mut hasher).unwrap();
    let hash_bytes = hasher.finalize();

    format!("{:X}", hash_bytes)
}

pub fn getfinpath(location: &String, hash: &String) -> String {
    // Gets and makes folderpath.
    let final_loc = format!(
        "{}/{}{}/{}{}/{}{}",
        location,
        hash.chars().next().unwrap(),
        hash.chars().nth(1).unwrap(),
        hash.chars().nth(2).unwrap(),
        hash.chars().nth(3).unwrap(),
        hash.chars().nth(4).unwrap(),
        hash.chars().nth(5).unwrap()
    );
    file::folder_make(&final_loc);
    return final_loc;
}
