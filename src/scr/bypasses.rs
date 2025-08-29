use futures::TryStreamExt;
use reqwest::blocking::{Client, Response};
use reqwest_websocket::Message;

use crate::download;
use crate::globalload::download_from;
use crate::logging;
use crate::Mutex;
use crate::RwLock;
use reqwest_websocket::RequestBuilderExt;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn ddos_guard_bypass(
    response_input: &Response,
    cli: &mut Client,
    url: &String,
) -> Option<Response> {
    let mut cookiestring = String::new();
    let cli = download::client_create(vec![], false);
    let mut should_attempt_bypass = false;
    for (header_name, header_value) in response_input.headers() {
        if header_name.as_str().contains("server")
            && header_value.to_str().unwrap().contains("ddos-guard")
        {
            should_attempt_bypass = true;
            break;
        }
    }

    let host = match response_input.url().host_str() {
        None => {
            return None;
        }
        Some(out) => out,
    };

    for cookie in cli.get(url).send().unwrap().cookies() {
        cookiestring += &format!("{}={};", cookie.name(), cookie.value());
    }

    let temp = cli
        .get("https://kemono.cr/.well-known/ddos-guard/js-challenge/index.js")
        .header("Referer", url)
        .header("Cookie", cookiestring.clone())
        .send()
        .unwrap();

    let guard_check_url = format!("https://check.ddos-guard.net/check.js");

    let guard_resp = cli.get(guard_check_url).send().unwrap();

    dbg!(&guard_resp);

    let mut etag = None;

    for cookie in guard_resp.cookies() {
        if cookie.name() == "__ddg2" {
            etag = Some(cookie.value().to_string());
            break;
        }
    }
    if etag.is_none() {
        for cookie in guard_resp.cookies() {
            dbg!(cookie);
        }

        return None;
    }

    let etag = etag.unwrap();
    dbg!(&etag);

    let ddos_id = cli
        .get(format!(
            "https://kemono.cr/.well-known/ddos-guard/id/{}",
            etag.to_string()
        ))
        .header("Referer", url)
        .header("Cookie", cookiestring.clone())
        .header("sec-fetch-dest", "document")
        .header("sec-fetch-mode", "navigate")
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-user", "?1")
        .send()
        .unwrap();
    cli.get(format!(
        "https://check.ddos-guard.net/.well-known/ddos-guard/id/{}",
        etag.to_string()
    ))
    .header("Referer", url)
    .header("Cookie", cookiestring.clone())
    .header("sec-fetch-dest", "document")
    .header("sec-fetch-mode", "navigate")
    .header("sec-fetch-site", "same-origin")
    .header("sec-fetch-user", "?1")
    .send()
    .unwrap();

    cookiestring += &format!("ddg_last_challenge={};", "0");
    cookiestring += &format!("__ddg2_={}", etag);

    dbg!(&cookiestring);

    let time_dur = Duration::from_secs(2);
    thread::sleep(time_dur);
    let temp = cli
        .post("https://kemono.cr/.well-known/ddos-guard/mark/")
        .body(include_str!("./fakeMark.json"))
        .header("referer", url)
        .header("cookie", cookiestring.clone())
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-user", "?1")
        .header("content-type", "text/plain;charset=UTF-8")
        .header("accept", "*/*")
        .header("accept-language", "en-US,en;q=0.5")
        .header("accept-encoding", "gzip, deflate")
        .header("DNT", "1");

    get_ws_ack(&cookiestring);

    //__ddg8_=2EJt4CkHenBsqE1n;__ddg10_=1754095361;__ddg9_=73.217.249.224;__ddgid_=t29ut4iB8IOZDLfs;__ddgmark_=rzW9LE7EzrrLUYAQ;__ddg5_=tKpsFx8eyI8g8LMw;__ddg2_=cE5XvR9HFHGrBgAj;ddg_last_challenge=1754094337599
    //let test = cli.get(url).header("Cookie", "ddg_last_challenge=0; __ddg8_=gVs0yjkYSxwgrxFR; __ddg10_=1754094336; __ddg9_=73.217.249.224; __ddgid_=ssqtJtKWcdvYyO1Y; __ddgmark_=vCk1j3tBwU8n366j; __ddg5_=llSvSbFs51GtF8Fy; __ddg2_=CE3ZYDujrkeAmEgf").send().unwrap();
    let test = cli.get(url).header("Cookie", cookiestring).send().unwrap();

    dbg!(test.status(), &test);

    if test.status().is_success() {
        return Some(test);
    }

    // dbg!(&ddos_id);
    // for cookie in ddos_id.cookies() {
    //     dbg!(cookie);
    // }

    //    let urls = parse_guard_resp(guard_resp);
    //    assert!(urls.len() == 2);
    //
    //    let site_url = format!("https://{}{}", host, urls[0]);

    /*cli.get(site_url)
            .header(reqwest::header::REFERER, "kemono.cr")
            .send();
    */

    return None;
    if response_input.status().is_client_error() && should_attempt_bypass {
        let host = match response_input.url().host_str() {
            None => {
                return None;
            }
            Some(out) => out,
        };

        let guard_check_url = format!("https://check.ddos-guard.net/check.js",);
        let mark_url = format!("https://{}/.well-known/ddos-guard/mark/", host);

        let guard_resp = cli.get(guard_check_url).send().unwrap();

        let urls = parse_guard_resp(guard_resp);
        assert!(urls.len() == 2);

        let site_url = format!("https://{}{}", host, urls[0]);

        dbg!(&site_url, &urls);
        cli.get(site_url)
            .header(reqwest::header::REFERER, "kemono.cr")
            .send();
        let e = cli.get(urls[1].clone()).send().unwrap();
        dbg!(&e, e.status(), e.headers());

        for cookie in e.cookies() {
            dbg!(cookie);
        }

        let resp = cli
            .post(&mark_url)
            .body(include_str!("./fakeMark.json"))
            .header(reqwest::header::REFERER, url)
            .send();

        match resp {
            Err(err) => {
                logging::error_log(&format!(
                    "Cannot run query for url: {} got response: {:?}",
                    mark_url, err
                ));
            }
            Ok(r) => {
                dbg!(r.status(), &r);
                for cookie in r.cookies() {
                    dbg!(cookie);
                }
                if r.status().is_success() {
                    cli.get("wss://kemono.cr/.well-known/ddos-guard/mark/ws")
                        .send();

                    thread::sleep(Duration::from_secs(2));
                    if let Ok(out) = cli.get(url).send() {
                        for cookie in out.cookies() {
                            dbg!(cookie);
                        }
                        dbg!(
                            out.status(),
                            reqwest::header::REFERER.to_string(),
                            url,
                            &out,
                        );
                        if out.status().is_success() {
                            return Some(out);
                        }
                    }
                }
            }
        }
    }

    None
}

fn parse_guard_resp(resp: Response) -> Vec<String> {
    let mut out = Vec::new();
    let stringparse = resp.text().unwrap();

    for i in stringparse.split(';') {
        for j in i.split('=') {
            if j.trim_start().trim_end().starts_with('\'') {
                out.push(j.trim_end().trim_start()[1..j.trim().len() - 2].to_string());
            }
        }
    }
    out
}

async fn wss(cookiestring: &String) {
    let useragent =
        "User-Agent Mozilla/5.0 (X11; Linux x86_64; rv:141.0) Gecko/20100101 Firefox/141.0"
            .to_string();

    let r = reqwest::ClientBuilder::default()
        .user_agent(useragent)
        .build()
        .unwrap()
        .get("wss://kemono.cr/.well-known/ddos-guard/mark/ws")
        .header("cookie", cookiestring.clone())
        .upgrade()
        .send()
        .await
        .unwrap();

    let mut websocket = r.into_websocket().await.unwrap();

    if let Some(msg) = websocket.try_next().await.unwrap() {
        if let Message::Text(msg) = msg {
            println!("{}", msg);
        }
    }
}

fn get_ws_ack(cookiestring: &String) {
    let websocket = async_std::task::block_on(wss(cookiestring));
}
