//extern crate urlparse;
use crate::scr::file;
use ahash::AHashMap;
use bytes;
use futures::future::join_all;
use http::Method;
use reqwest::{get, Client, Error, Request, Response};
use sha2::Digest;
use sha2::Sha512;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use tower::layer::util::{Identity, Stack};
use tower::limit::RateLimit;
use tower::ServiceExt;
use tower::{BoxError, Service};
use url::Url;
use urlparse::urlparse;

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
    let mut example = tower::ServiceBuilder::new()
        .rate_limit(time.0, time.1)
        .service(client);
    return example;
}

///
/// time.0 is the requests per time.1
/// time.1 is number of total seconds per time slot
///

pub async fn dltext(example: &mut RateLimit<Client>, url_vec: Vec<String>) -> Vec<String> {
    let mut respvec: Vec<Response> = Vec::new();
    let mut retvec: Vec<String> = Vec::new();
    for each in url_vec {
        let url = Url::parse(&each).unwrap();
        let requestit = Request::new(Method::GET, url);
        let resp = example.ready().await.unwrap().call(requestit).await;
        respvec.push(resp.unwrap());
        println!("Getting DATA FROM URL len: {}.", &respvec.len());
        dbg!(&each);
        break;
    }
    // Return the response as an immediate future
    let fut = async { example };
    //dbg!(Box::pin(fut).await);
    for each in respvec {
        //dbg!(Box::pin(fut).await);
        let st: String = each.text().await.unwrap().to_string();
        retvec.push(st);
        println!("Getting DATA FROM resp len: {}.", &retvec.len());
    }
    return retvec;
}

///
/// Download file
///
pub async fn file_download(
    example: &mut RateLimit<Client>,
    url_vec: &Vec<String>,
    location: &String,
) -> (HashMap<String, String>, Vec<String>) {
    let mut fut: HashMap<String, String> = HashMap::new();
    let mut ext_vec: Vec<String> = Vec::new();
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
    let mut cnt = 0;
    for each in url_vec {
        let url = Url::parse(&each).unwrap();
        let requestit = Request::new(Method::GET, url);
        let a = example
            .ready()
            .await
            .unwrap()
            .call(requestit)
            .await
            .unwrap(); //.unwrap().call(requestit).await
        let headers = format!("{:?}", &a.headers().get("content-type").unwrap());
        dbg!(&each);
        //dbg!(example.ready());

        let mut hasher = Sha512::new();
        let bytes = a.bytes().await;
        hasher.update(&bytes.as_ref().unwrap());
        //let bystring= &bytes.unwrap();
        //let mut temp: &mut [u8] = u8::new();
        //bystring.clone_into(temp);
        //std::io::copy(&mut temp, &mut hasher);
        let hash = format!("{:X}", hasher.finalize());
        dbg!(&hash);

        let final_loc = format!(
            "./{}/{}{}/{}{}/{}{}",
            &location,
            hash.chars().nth(0).unwrap(),
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
        fut.insert(each.to_string(), hash.to_string());
        dbg!(cnt, hash);
        std::io::copy(&mut content, &mut file_path).unwrap();

        let metadata = fs::metadata(orig_path).unwrap();
        dbg!(metadata.file_type());
        let mut split = headers.split("/");
        let header_split_vec: Vec<&str> = split.collect();
        let header_split_vec1: Vec<&str> = header_split_vec[1].split('"').collect();
        ext_vec.push(header_split_vec1[0].to_string());
        //dbg!(hash_file(cnt.to_string()));
        cnt += 1;
    }
    //let futone = async { example };
    //joined_fut.await;
    //for each in fut {
    //    let b = each.bytes().await.unwrap();
    //    dbg!("downloaded");
    //    //dbg!(b);
    //}
    return (fut, ext_vec);
}

pub fn hash_file(filename: String) -> String {
    let mut hasher = Sha512::new();
    let mut file = fs::File::open(filename).unwrap();

    let bytes_written = io::copy(&mut file, &mut hasher).unwrap();
    let hash_bytes = hasher.finalize();

    return format!("{:X}", hash_bytes);
}
