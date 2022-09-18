//extern crate urlparse;
use http::Method;
use reqwest::{Error, Request, Response};
use std::time::Duration;
use tower::layer::util::{Identity, Stack};
use tower::ServiceExt;
use tower::{BoxError, Service};
use tower::limit::RateLimit;
use ahash::AHashMap;
use std::collections::HashMap;
use url::Url;
use reqwest::Client;
use urlparse::urlparse;

///
/// Makes ratelimiter and example
///
#[tokio::main]
pub async fn ratelimiter_create(time: (u64, Duration)) -> RateLimit<Client> {
    let client = reqwest::ClientBuilder::new()
        .user_agent("RUST-HYDRUS V0.1")
        .build()
        .unwrap();
    let mut example = tower::ServiceBuilder::new()
        //.buffer(100)
        .rate_limit(time.0, time.1)
        .service(client);
    return example
}

///
/// time.0 is the requests per time.1
/// time.1 is number of total seconds per time slot
///
#[tokio::main]
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
#[tokio::main]
pub async fn file_download(example: &mut RateLimit<Client>, url_vec: &HashMap<String, HashMap<String, Vec<String>>>) {
    if url_vec.is_empty() {return}
    for each in url_vec.keys()
    {
        dbg!(each);
    }
}
