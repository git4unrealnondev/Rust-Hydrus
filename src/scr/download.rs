//extern crate urlparse;
use super::scraper;
use super::sharedtypes;
use crate::scr::file;
use ahash::AHashMap;
use futures;
use http::request;
use http::Method;
use reqwest::{Client, Request, Response};
use sha2::Digest;
use sha2::Sha512;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Cursor;
use std::io::Error;
use std::sync::Mutex;
use std::time::Duration;
use tower::limit::RateLimit;
use tower::Service;
use tower::ServiceExt;
use url::Url;
extern crate cloudflare_bypasser;
extern crate reqwest;
use super::database;
use crate::scr::scraper::InternalScraper;
use async_executor::Executor;
use async_std::task;
use itertools::Itertools;
use libloading::Library;
use ratelimit;
use std::marker::Sync;
use std::sync::Arc;
///
/// Makes ratelimiter and example
///
pub fn ratelimiter_create(number: u64, duration: Duration) -> ratelimit::Limiter {
    dbg!("Making ratelimiter with: {} {}", &number, &duration);

    // The wrapper that implements ratelimiting
    //tower::ServiceBuilder::new()
    //    .rate_limit(number, duration)
    //    .service(client);
    ratelimit::Builder::new()
        .capacity(4) //number of tokens the bucket will hold
        .quantum(number.try_into().unwrap()) //add one token per interval
        .interval(duration) //add quantum tokens every 1 second
        .build()
}

///
/// Creates Client that the downloader will use.
///
///
pub fn client_create() -> Client {
    let useragent = "RustHydrus V1".to_string();
    // The client that does the downloading
    let client = reqwest::ClientBuilder::new()
        .user_agent(useragent)
        .build()
        .unwrap();

    client
}

///
/// Downloads text into db as responses. Filters responses by default limit if their's anything wrong with request.
///
pub async fn dltext_new(
    url_string: String,
    ratelimit_object: &mut ratelimit::Limiter,
    client: &mut Client,
) -> Result<String, reqwest::Error> {
    //let mut ret: Vec<AHashMap<String, AHashMap<String, Vec<String>>>> = Vec::new();
    //let ex = Executor::new();

    let url = Url::parse(&url_string).unwrap();
    //let url = Url::parse("http://www.google.com").unwrap();

    //let requestit = Request::new(Method::GET, url);
    //fut.push();
    dbg!("Spawned web reach");
    //let futureresult = futures::executor::block_on(ratelimit_object.ready())
    //    .unwrap()
    ratelimit_object.wait();
    let futureresult = client.get(url).send().await;

    //let test = reqwest::get(url).await.unwrap().text();

    //let futurez = futures::executor::block_on(futureresult);
    dbg!(&futureresult);

    match futureresult {
        Ok(_) => Ok(task::block_on(futureresult.unwrap().text()).unwrap()),
        Err(_) => Err(futureresult.err().unwrap()),
    }
}

///
/// Downloads file to position
///
pub async fn dlfile_new(
    ratelimit_object: &mut ratelimit::Limiter,
    client: &mut Client,
    tags: &sharedtypes::FileObject,
    db: &mut Arc<Mutex<database::Main>>,
) {
    
    // Determine if we need to download file.
    let does_url_exist = false;
    {
        let unwrappydb = &mut db.lock().unwrap();
        let namespace_id = unwrappydb.namespace_get(&"source_url".to_string()); // defaults to 0 due to unknown.
        if !namespace_id.1 { // Namespace doesn't exist. Will create
            unwrappydb.namespace_add(0, "source_url".to_string(), "Source URL for a file.".to_string(), true);
        }
        let url_tag = unwrappydb.tag_get_name(tags.source_url.to_string(), namespace_id.0);
        does_url_exist = url_tag.1;
        
    }
    dbg!(does_url_exist);
    
    
    ratelimit_object.wait();
    //dbg!(tags);
    //dbg!("waited {}", &url);
}
/*
    // Super Ganky Way of doing this. splits the vec into a vec of vec<String>. Minimizes the total number of requests at a time.
    let chunks: Vec<Vec<String>> = url_vec
        .into_iter()
        .chunks(1)
        .into_iter()
        .map(|c| c.collect())
        .collect();

    // Makes the ratelimiter object


    let client = reqwest::ClientBuilder::new()
        //.user_agent(useragent)
        .build()
        .unwrap();

    for chunkvec in chunks {
        // let mut fut = Vec::new();
        for urlstring in chunkvec {
            dbg!(&urlstring);

            let url = Url::parse(&urlstring).unwrap();
            //let url = Url::parse("http://www.google.com").unwrap();

            dbg!(&url);
            let requestit = Request::new(Method::GET, url);
            //fut.push();
            dbg!("Spawned web reach");
            let test = ratelimit_object.ready().await.unwrap().call(requestit);
            //let test = reqwest::get(url).await.unwrap().text();

            test.await
            //dbg!(&temp);
            //dbg!(temp.unwrap().text().await.unwrap());

        }
        dbg!("Done");
        /*for each in fut {
            let test = ex.run(each).await.unwrap();
            dbg!("Waited");
            if test.status() != 200 {panic!("ERROR CODE {} on {}", test.status(), test.url())}

            let st: String = test.text().await.unwrap().to_string();
            dbg!("two");
            // Calls the parser to parse the data.
            let scrap = scraper::parser_call(liba, &st);
            dbg!(&scrap);
            match scrap {
                Ok(_) => {ret.push(scrap.unwrap());},
                Err(_) => {break}
            }


        }*/
        //for each in fut {
        //    dbg!(futures::(ex.run(each)));
        //}
        break;
    }

    ret
}
*/
///
/// time.0 is the requests per time.1cargo run -- job --add e6 "test female male" now false
/// time.1 is number of total seconds per time slot
///

pub async fn dltext(
    url_vec: Vec<String>,
    parser: &mut scraper::ScraperManager,
    uintref: &InternalScraper,
) -> AHashMap<String, AHashMap<String, AHashMap<String, Vec<String>>>> {
    let respvec: Vec<Response> = Vec::new();
    let retvec: Vec<String> = Vec::new();
    let mut test: AHashMap<String, AHashMap<String, AHashMap<String, Vec<String>>>> =
        AHashMap::new();

    // The wrapper that implements ratelimiting

    let client = reqwest::ClientBuilder::new()
        .user_agent("RUST-HYDRUS V0.1")
        .build()
        .unwrap();
    let mut example = tower::ServiceBuilder::new()
        .rate_limit(1, Duration::from_secs(2))
        .concurrency_limit(1)
        .service(client);

    println!("Starting scraping urls.");
    for (cnt, each) in url_vec.into_iter().enumerate() {
        let url = Url::parse(&each).unwrap();
        //let url = Url::parse("http://www.google.com").unwrap();
        let requestit = Request::new(Method::GET, url);

        dbg!("B");
        dbg!(&each);
        //dbg!(&example);
        let resp = example
            .ready()
            .await
            .unwrap()
            .call(requestit)
            .await
            .unwrap();
        dbg!("a");
        //let resp = client.call(requestit).await.unwrap();
        //let resp = reqwest::blocking::Request(requestit).user_agent("RustHydrus V0.1");
        //thread::sleep(Duration::from_millis(750));
        println!("Downloaded total urls to parse: {}", &cnt);
        //dbg!(resp.text().await.unwrap());
        //let resp = example.ready().await.unwrap().call(requestit).await.unwrap();

        let st: String = resp.text().await.unwrap().to_string();
        //let st: String = "[posts]".to_string();
        //test.insert(st, "".to_string());
        //retvec.push(st);
        //respvec.push(resp);
        println!("Getting DATA FROM URL len: {}.", &respvec.len());

        /*match parser.parser_call(uintref, &st) {
            Ok(_) => (),
            Err(_) => break,
        }

        test.insert(cnt.to_string(), parser.parser_call(uintref, &st).unwrap());*/
    }
    test
}

///
/// Download file
///
pub async fn file_download(
    url_vec: &String,
    location: &String,
) -> (HashMap<String, String>, String) {
    let mut fut: HashMap<String, String> = HashMap::new();
    let mut ext_vec: String = String::new();
    if url_vec.is_empty() {
        return (fut, ext_vec);
    }
    let client = reqwest::ClientBuilder::new()
        .user_agent("RUST-HYDRUS V0.1")
        .build()
        .unwrap();
    // The wrapper that implements ratelimiting
    let mut exampleone = tower::ServiceBuilder::new()
        .rate_limit(2, Duration::from_secs(1))
        .service(client);

    let mut url = Url::parse(url_vec).unwrap();
    let mut requestit = Request::new(Method::GET, url);
    let mut a = exampleone.ready().await.unwrap().call(requestit).await;
    // Handles failed downloads or weidness from system.
    match a {
        Ok(_) => (),
        Err(_) => {
            url = Url::parse(url_vec).unwrap();
            requestit = Request::new(Method::GET, url);
            a = exampleone.ready().await.unwrap().call(requestit).await;
        }
    }
    let headers = format!(
        "{:?}",
        &a.as_ref().unwrap().headers().get("content-type").unwrap()
    );
    //dbg!(example.ready());

    let mut hasher = Sha512::new();
    let bytes = a.unwrap().bytes().await;
    hasher.update(&bytes.as_ref().unwrap());
    //let bystring= &bytes.unwrap();
    //let mut temp: &mut [u8] = u8::new();
    //bystring.clone_into(temp);
    //std::io::copy(&mut temp, &mut hasher);
    let hash = format!("{:X}", hasher.finalize());

    let final_loc = format!(
        "{}/{}{}/{}{}/{}{}",
        &location,
        hash.chars().next().unwrap(),
        hash.chars().nth(1).unwrap(),
        hash.chars().nth(2).unwrap(),
        hash.chars().nth(3).unwrap(),
        hash.chars().nth(4).unwrap(),
        hash.chars().nth(5).unwrap()
    );

    file::folder_make(&final_loc);
    let mut content = Cursor::new(bytes.unwrap());

    let orig_path = format!("{}/{}", &final_loc, &hash);
    let mut file_path = std::fs::File::create(&orig_path).unwrap();
    fut.insert(url_vec.to_string(), hash);
    std::io::copy(&mut content, &mut file_path).unwrap();

    let metadata = fs::metadata(orig_path).unwrap();

    let split = headers.split('/');
    let header_split_vec: Vec<&str> = split.collect();
    let header_split_vec1: Vec<&str> = header_split_vec[1].split('"').collect();
    ext_vec = header_split_vec1[0].to_string();

    (fut, ext_vec)
}

pub fn hash_file(filename: String) -> String {
    let mut hasher = Sha512::new();
    let mut file = fs::File::open(filename).unwrap();

    let bytes_written = io::copy(&mut file, &mut hasher).unwrap();
    let hash_bytes = hasher.finalize();

    format!("{:X}", hash_bytes)
}
