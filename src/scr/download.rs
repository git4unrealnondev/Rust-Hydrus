use super::plugins::PluginManager;
use crate::database::Main;

// extern crate urlparse;
use super::sharedtypes;
use crate::logging;
use bytes::Bytes;
use file_format::FileFormat;
use log::{error, info};
use reqwest::blocking::Client;
use reqwest::cookie;
use sha2::Digest as sha2Digest;
use sha2::Sha256;
use sha2::Sha512;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::time::Duration;
use url::Url;

extern crate reqwest;

use crate::helpers;
use ratelimit::Ratelimiter;
use std::fs::File;

// use std::sync::{Arc, Mutex};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

/// Makes ratelimiter and example
pub fn ratelimiter_create(number: u64, duration: Duration) -> Ratelimiter {
    println!(
        "Making ratelimiter with: {} Request Per: {:?}",
        &number, &duration
    );

    // The wrapper that implements ratelimiting
    Ratelimiter::builder(number, duration)
        .max_tokens(number)
        .initial_available(number)
        .build()
        .unwrap()
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
        .build()
        .unwrap()
}

/// Downloads text into db as responses. Filters responses by default limit if
/// their's anything wrong with request.
pub async fn dltext_new(
    url_string: String,
    client: &Client,
    ratelimiter_obj: &Arc<Mutex<Ratelimiter>>,
) -> Result<String, reqwest::Error> {
    // let mut ret: Vec<AHashMap<String, AHashMap<String, Vec`<String>`>>> =
    // Vec::new(); let ex = Executor::new(); let url =
    // Url::parse("http://www.google.com").unwrap();
    let mut cnt = 0;
    loop {
        let url = Url::parse(&url_string).unwrap();

        // let requestit = Request::new(Method::GET, url); fut.push();
        logging::info_log(&format!("Spawned web reach to: {}", &url_string));

        // let futureresult = futures::executor::block_on(ratelimit_object.ready())
        // .unwrap()
        ratelimiter_wait(ratelimiter_obj);
        let futureresult = client.get(url).send();

        // let test = reqwest::get(url).await.unwrap().text(); let futurez =
        // futures::executor::block_on(futureresult); dbg!(&futureresult);
        if cnt == 3 {
            return Err(futureresult.err().unwrap());
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
                return Err(futureresult.err().unwrap());
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

/// Downloads file to position
pub fn dlfile_new(
    client: &Client,
    db: Arc<Mutex<Main>>,
    file: &sharedtypes::FileObject,
    location: &String,
    pluginmanager: Option<Arc<Mutex<PluginManager>>>,
    ratelimiter_obj: &Arc<Mutex<Ratelimiter>>,
) -> Option<(String, String)> {
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
            ratelimiter_wait(&ratelimiter_obj);
            logging::info_log(&format!("Downloading: {} to: {}", &fileurlmatch, &location));
            let mut futureresult = client.get(url.as_ref()).send();
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
                        futureresult = client.get(url.as_ref()).send();
                    }
                }
            }

            // Downloads file into byte memory buffer
            let byte = futureresult.unwrap().bytes();
            // Error handling for dling a file. Waits 10 secs to retry
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
            if cnt >= 3 {
                return None;
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
                        "Parser file: {} FAILED HASHCHECK: {} {}",
                        &file.hash, status.0, status.1
                    );
                    cnt += 1;
                } else {
                    // dbg!("Parser returned: {} Got: {}", &file.hash, status.0);
                }
                if cnt >= 3 {
                    return None;
                }
                boolloop = !status.1;
            }
        };
    }
    let final_loc = helpers::getfinpath(location, &hash);

    // Gives file extension
    let file_ext = FileFormat::from_bytes(&bytes).extension().to_string();

    // Gets final path of file.
    let orig_path = format!("{}/{}", &final_loc, &hash);
    let file_path_res = std::fs::File::create(&orig_path);

    while file_path_res.is_err() {
        logging::info_log(&format!(
            "Cannot create file at path: {} Err: {:?}",
            &orig_path, file_path_res
        ));
        thread::sleep(Duration::from_secs(1));
    }

    // If the plugin manager is None then don't do anything plugin wise. Useful for if
    // doing something that we CANNOT allow plugins to run.
    {
        if let Some(pluginmanager) = pluginmanager {
            crate::plugins::plugin_on_download(pluginmanager, db, bytes.as_ref(), &hash, &file_ext);
        }
    }
    let mut content = Cursor::new(bytes);

    if let Ok(mut file_path) = file_path_res {
        // Copies file from memory to disk
        while let Err(err) = std::io::copy(&mut content, &mut file_path) {
            logging::info_log(&format!(
                "Cannot copy file at path: {} Err: {:?}",
                &orig_path, err
            ));

            thread::sleep(Duration::from_secs(1));
        }
        logging::info_log(&format!("Downloaded hash: {}", &hash));
    }
    Some((hash, file_ext))
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
