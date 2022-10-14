//extern crate urlparse;
use crate::scr::file;
use crate::scr::scraper;
use http::Method;
use reqwest::{Client, Request, Response};
use sha2::Digest;
use sha2::Sha512;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Cursor;
use std::time::Duration;
use tower::limit::RateLimit;
use tower::Service;
use tower::ServiceExt;
use url::Url;
extern crate cloudflare_bypasser;
extern crate reqwest;
///
/// Makes ratelimiter and example
///

pub async fn ratelimiter_create(time: (u64, Duration)) -> RateLimit<Client> {
    // The client that does the downloading
    let client = reqwest::ClientBuilder::new()
        .user_agent("RUST-HYDRUS V0.1")
        .build()
        .unwrap();
    // The wrapper that implements ratelimiting
    tower::ServiceBuilder::new()
        .rate_limit(time.0, time.1)
        .service(client)
}

///
/// time.0 is the requests per time.1cargo run -- job --add e6 "test female male" now false
/// time.1 is number of total seconds per time slot
///
#[tokio::main]
pub async fn dltext(
    url_vec: Vec<String>,
    parser: &mut scraper::ScraperManager,
    uintref: usize,
) -> HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> {
    let respvec: Vec<Response> = Vec::new();
    let retvec: Vec<String> = Vec::new();
    let mut test: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> = HashMap::new();

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

        match parser.parser_call(uintref, &st) {
            Ok(_) => (),
            Err(_) => break,
        }

        test.insert(cnt.to_string(), parser.parser_call(uintref, &st).unwrap());
    }
    test
}

///
/// Download file
///
#[tokio::main]
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

    let url = Url::parse(url_vec).unwrap();
    let requestit = Request::new(Method::GET, url);
    let a = exampleone
        .ready()
        .await
        .unwrap()
        .call(requestit)
        .await
        .unwrap(); //.unwrap().call(requestit).await
    let headers = format!("{:?}", &a.headers().get("content-type").unwrap());
    //dbg!(example.ready());

    let mut hasher = Sha512::new();
    let bytes = a.bytes().await;
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
