use crate::sharedtypes::DEFAULT_PRIORITY;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use chrono::format::ParseError;
use chrono::{DateTime, NaiveDateTime, Utc};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::io::Write;
use std::{collections::HashSet, time::Duration};
use ureq::{Agent, ResponseExt};
#[path = "../../../src/client.rs"]
mod client;
#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

pub const LOCAL_NAME: &str = "Bunkr";
pub const MAIN_SITE: &str = "https://bunkr.cr";

///
/// Returns the info needed to scrape an fxembed system
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut defaultscraper = sharedtypes::return_default_globalpluginparser();

    defaultscraper.name = LOCAL_NAME.into();
    defaultscraper.storage_type = Some(sharedtypes::ScraperOrPlugin::Scraper(
        sharedtypes::ScraperInfo {
            ratelimit: (1, Duration::from_secs(1)),
            sites: vec![LOCAL_NAME.into(), LOCAL_NAME.to_lowercase()],
            priority: DEFAULT_PRIORITY,
            num_threads: None,
            modifiers: vec![],
        },
    ));

    let _callbackvec = [sharedtypes::GlobalCallbacks::Start(
        sharedtypes::StartupThreadType::SpawnInline,
    )];

    vec![defaultscraper]
}

///
/// Takes the raw user input and outputs a url to download
///
#[unsafe(no_mangle)]
pub fn url_dump(
    _params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = vec![];

    let mut should_search = false;

    if scraperdata.job.param.len() == 1 {
        let mut search_string = "https://bunkr-albums.io/?search=".to_string();
        for param in scraperdata.job.param.iter() {
            if let sharedtypes::ScraperParam::Normal(temp) = param {
                if temp.contains("/a/") || temp.contains("/f/") {
                    out.push((temp.to_string(), scraperdata.clone()));
                    continue;
                }
                search_string += &format!("{} ", temp);
                should_search = true;
            }
        }
        let temp = search_string.trim();
        if should_search {
            out.push((temp.to_string(), scraperdata.clone()));
        }
    }

    out
}

#[unsafe(no_mangle)]
pub fn parser(
    html_input: &str,
    source_url: &str,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut out = vec![];

    let mut flag = Vec::new();
    let mut file_out = HashSet::new();
    let mut tag_out = HashSet::new();
    let fragment = Html::parse_fragment(html_input);
    if source_url.contains("bunkr-albums") {
        let selector = Selector::parse(r#"a[aria-label="watch"][href]"#).unwrap();

        for a in fragment.select(&selector) {
            if let Some(href) = a.value().attr("href") {
                let mut tag = HashSet::new();

                let mut scraperdata = scraperdata.clone();
                scraperdata.job = sharedtypes::JobScraper {
                    site: LOCAL_NAME.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(href.to_string())],
                    job_type: sharedtypes::DbJobType::Scraper,
                };

                let temp_tag = sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "".to_string(),
                        description: None,
                    },
                    tag: href.to_string(),
                    tag_type: sharedtypes::TagType::ParseUrl((scraperdata, None)),
                    relates_to: None,
                };
                tag.insert(temp_tag);
                out.push(sharedtypes::ScraperReturn::Data(
                    sharedtypes::ScraperObject {
                        file: HashSet::new(),
                        tag,
                        flag: vec![],
                    },
                ));
            }
        }
        // Gets other pages from the bunkr albums site
        let selector = Selector::parse(r#"a[class="btn btn-sm btn-seco max-sm:hidden"]"#).unwrap();
        for a in fragment.select(&selector) {
            if let Some(href) = a.value().attr("href") {
                let mut tag = HashSet::new();
                let mut scraperdata = scraperdata.clone();
                scraperdata.job = sharedtypes::JobScraper {
                    site: LOCAL_NAME.to_string(),
                    param: vec![sharedtypes::ScraperParam::Url(format!(
                        "{}{}",
                        "https://bunkr-albums.io", href
                    ))],
                    job_type: sharedtypes::DbJobType::Scraper,
                };

                let temp_tag = sharedtypes::TagObject {
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "".to_string(),
                        description: None,
                    },
                    tag: href.to_string(),
                    tag_type: sharedtypes::TagType::ParseUrl((scraperdata, None)),
                    relates_to: None,
                };
                tag.insert(temp_tag);
                out.push(sharedtypes::ScraperReturn::Data(
                    sharedtypes::ScraperObject {
                        file: HashSet::new(),
                        tag,
                        flag: vec![],
                    },
                ));
            }
        }
    } else {
        // This section handles the files page
        // Gets the download info for it
        let mut album_name = None;
        let selector = Selector::parse(r#"h1[class="truncate"]"#).unwrap();
        for span in fragment.select(&selector) {
            let album_name_temp = span.text().collect::<String>().to_string();
            let album_name_temp = album_name_temp.trim();
            album_name = Some(album_name_temp.to_string());
        }

        // Overrides the scraping info if we already have this
        if let Some(albumname) = scraperdata.user_data.get("bunkr-album-name") {
            album_name = Some(albumname.to_string());
        }

        let mut is_file = false;

        let title_selector = Selector::parse("h1.truncate").unwrap();
        let mut filename = None;
        if let Some(h1) = fragment.select(&title_selector).next() {
            filename = Some(h1.text().collect::<String>().trim().to_string());
        }
        let selector = Selector::parse(r#"div[id="fileTracker"]"#).unwrap();

        'fragloop: for a in fragment.select(&selector) {
            is_file = true;
            if let Some(file_id) = a.value().attr("data-file-id") {
                let api_url = "https://apidl.bunkr.ru/api/_001_v2";
                let data_json = format!("{{\"id\":\"{}\"}}", file_id);

                let config = Agent::config_builder()
                    .timeout_global(Some(Duration::from_secs(5)))
                    .build();

                let agent: Agent = config.into();
                let response = agent
                    .post(api_url)
                    .header("Accept", "application/json, text/plain, */*")
                    .header("Content-Type", "application/json")
                    .header("Origin", "https://bunkr.su")
                    .send(data_json);
                if response.is_err() {
                    continue 'fragloop;
                }
                let response: ApiFileDownloadResponse =
                    response.unwrap().body_mut().read_json().unwrap();
                let url = decrypt_url(response);
                if let Ok(url) = url {
                    if let Some(ref tag) = filename {
                        let fileid = source_url.rsplit('/').next().unwrap();
                        let file_tag = make_file_tags(fileid, tag, scraperdata);

                        if let Some(ref album_name) = album_name {
                            for tag in make_general_tags(album_name, tag, scraperdata) {
                                tag_out.insert(tag);
                            }
                        }

                        if !should_download_file(fileid) {
                            client::log(format!(
                                "scraper-bunkr: Cannot download because a bunkr id already exists {}",
                                fileid
                            ));
                            for tag in file_tag {
                                tag_out.insert(tag);
                            }
                            out.push(sharedtypes::ScraperReturn::Data(
                                sharedtypes::ScraperObject {
                                    file: file_out,
                                    tag: tag_out,
                                    flag,
                                },
                            ));
                            out.push(sharedtypes::ScraperReturn::Nothing);
                            return out;
                        }
                        match download_file(&url, source_url) {
                            Ok(filevec) => {
                                file_out.insert(sharedtypes::FileObject {
                                    source: Some(sharedtypes::FileSource::Bytes(filevec)),
                                    hash: sharedtypes::HashesSupported::None,
                                    tag_list: file_tag,
                                    skip_if: vec![],
                                });
                            }
                            Err(err) => {
                                for tag in file_tag {
                                    tag_out.insert(tag);
                                }
                                match err.to_string().as_str() {
                                    "Server is in maintence mode" | "Too many retries" => {
                                        out.push(sharedtypes::ScraperReturn::RetryLater(3600)); // wait one hour
                                        return out;
                                        // flag.push(sharedtypes::Flags::Redo);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else {
                    client::log_no_print(format!(
                        "scraper-bunkr: Couldn't get the source url for {}",
                        source_url
                    ));
                }
            }
        }

        let selector = Selector::parse(r#"span[class="ic-clock ic-before text-xs before:text-sm before:opacity-60 inline-flex items-center gap-1 py-1 px-2 rounded-full border border-soft theDate"]"#).unwrap();
        let mut times = Vec::new();

        if !is_file {
            let time = match scraperdata.user_data.get("bunkr-album-time") {
                Some(time) => Some(time.clone()),
                None => {
                    for span in fragment.select(&selector) {
                        // Define the format of the input string
                        let format = "%H:%M:%S %d/%m/%Y";

                        let internal_time = &span.text().collect::<String>();

                        // Parse without timezone
                        let naive =
                            NaiveDateTime::parse_from_str(internal_time.trim(), format).unwrap();

                        // Interpret the naive datetime as UTC
                        let utc_dt: DateTime<Utc> = DateTime::<Utc>::from_utc(naive, Utc);

                        // Unix timestamp (seconds since epoch)
                        let unix_timestamp = utc_dt.timestamp();

                        times.push(unix_timestamp);
                    }

                    times.sort();
                    times.reverse();
                    times.first().map(|a| a.to_string())
                }
            };

            let albumid = match scraperdata.user_data.get("bunkr-album-id") {
                Some(out) => out,
                None => source_url.rsplit('/').next().unwrap(),
            };

            {
                let mut pagenum_largest = 1;
                let mut pagenum_storage = Vec::new();

                let selector = Selector::parse(r#"nav.pagination a"#).unwrap();
                for a in fragment.select(&selector) {
                    if let Some(href) = a.value().attr("href") {
                        let pagenum_ref = match scraperdata.user_data.get("bunkr-page-current") {
                            Some(out) => out.parse::<u32>().unwrap(),
                            None => 1,
                        };

                        if let Some(pagenum) = extract_page(href) {
                            if pagenum > pagenum_ref {
                                pagenum_largest = pagenum;
                                pagenum_storage.push(pagenum);
                            }
                        }
                    }
                }
                for pagenum in pagenum_storage {
                    let url = format!("{}?page={}", source_url, &pagenum);
                    let mut tag = HashSet::new();
                    let mut scraperdata = scraperdata.clone();
                    scraperdata
                        .user_data
                        .insert("bunkr-album-id".to_string(), albumid.to_string());
                    if let Some(ref name) = album_name {
                        scraperdata
                            .user_data
                            .insert("bunkr-album-name".to_string(), name.to_string());
                    }

                    scraperdata.user_data.insert(
                        "bunkr-page-current".to_string(),
                        pagenum_largest.to_string(),
                    );

                    if let Some(ref time) = time {
                        scraperdata
                            .user_data
                            .insert("bunkr-album-time".to_string(), time.to_string());
                    }
                    scraperdata.job = sharedtypes::JobScraper {
                        site: LOCAL_NAME.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(url)],
                        job_type: sharedtypes::DbJobType::Scraper,
                    };
                    let temp_tag = sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "".to_string(),
                            description: None,
                        },
                        tag: "".to_string(),
                        tag_type: sharedtypes::TagType::ParseUrl((scraperdata, None)),
                        relates_to: None,
                    };
                    tag.insert(temp_tag);
                    out.push(sharedtypes::ScraperReturn::Data(
                        sharedtypes::ScraperObject {
                            file: HashSet::new(),
                            tag,
                            flag: vec![],
                        },
                    ));
                }
            }
            // Scrapes the files from the albums page
            let selector = Selector::parse(r#"a[aria-label="download"][href]"#).unwrap();

            for a in fragment.select(&selector) {
                if let Some(href) = a.value().attr("href") {
                    // Skips if we already have a fileid that relates to the file
                    if let Some(fileid) = href.rsplit('/').next() {
                        if !should_download_file(fileid)
                            && scraperdata.system_data.get("recursion").is_none()
                        {
                            client::log_no_print(format!(
                                "Scraper - Bunkr - Skipping file download because bunkr file id exists: {}",
                                fileid
                            ));
                            continue;
                        }
                    }
                    let url = format!("{}{}", MAIN_SITE, &href);
                    let mut tag = HashSet::new();
                    let mut scraperdata = scraperdata.clone();
                    //let albumid = source_url.rsplit('/').next().unwrap();
                    scraperdata
                        .user_data
                        .insert("bunkr-album-id".to_string(), albumid.to_string());
                    if let Some(ref name) = album_name {
                        scraperdata
                            .user_data
                            .insert("bunkr-album-name".to_string(), name.to_string());
                    }

                    if let Some(ref time) = time {
                        scraperdata
                            .user_data
                            .insert("bunkr-album-time".to_string(), time.to_string());
                    }
                    scraperdata.job = sharedtypes::JobScraper {
                        site: LOCAL_NAME.to_string(),
                        param: vec![sharedtypes::ScraperParam::Url(url)],
                        job_type: sharedtypes::DbJobType::Scraper,
                    };
                    let temp_tag = sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "".to_string(),
                            description: None,
                        },
                        tag: href.to_string(),
                        tag_type: sharedtypes::TagType::ParseUrl((scraperdata, None)),
                        relates_to: None,
                    };
                    tag.insert(temp_tag);
                    out.push(sharedtypes::ScraperReturn::Data(
                        sharedtypes::ScraperObject {
                            file: HashSet::new(),
                            tag,
                            flag: vec![],
                        },
                    ));
                }
            }
        }

        out.push(sharedtypes::ScraperReturn::Data(
            sharedtypes::ScraperObject {
                file: file_out,
                tag: tag_out,
                flag,
            },
        ));
    }

    out
}

fn make_general_tags(
    album_name: &str,
    tag: &String,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<sharedtypes::TagObject> {
    let limit_to = scraperdata
        .user_data
        .get("bunkr-album-time")
        .map(|time| sharedtypes::Tag {
            tag: time.to_string(),
            namespace: sharedtypes::GenericNamespaceObj {
                name: "bunkr-album-time".to_string(),
                description: Some("Last time the bunkr album was modified".to_string()),
            },
        });
    let relates_to = scraperdata.user_data.get("bunkr-album-id").map(|tag| sharedtypes::SubTag {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "bunkr-album-id".to_string(),
                description: Some(
                    "A unique album id for bunkr. Sometimes its called a slug internally. Often used as a root for the album".to_string(),),},
            tag: tag.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            limit_to,
        });

    vec![sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "bunkr-album-name".to_string(),
            description: Some("A name for a bunkr album".to_string()),
        },
        tag: album_name.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to,
    }]
}

fn make_file_tags(
    fileid: &str,
    tag: &String,
    scraperdata: &sharedtypes::ScraperData,
) -> Vec<sharedtypes::TagObject> {
    let limit_to = scraperdata
        .user_data
        .get("bunkr-album-time")
        .map(|time| sharedtypes::Tag {
            tag: time.to_string(),
            namespace: sharedtypes::GenericNamespaceObj {
                name: "bunkr-album-time".to_string(),
                description: Some("Last time the bunkr album was modified".to_string()),
            },
        });

    let relates_to = scraperdata.user_data.get("bunkr-album-id").map(|tag| sharedtypes::SubTag {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "bunkr-album-id".to_string(),
                description: Some(
                    "A unique album id for bunkr. Sometimes its called a slug internally. Often used as a root for the album".to_string(),),},
            tag: tag.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            limit_to,
        });

    let tag_out = sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "bunkr-filename".to_string(),
            description: Some("A filename from a bunkr file".to_string()),
        },
        tag: tag.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: Some(sharedtypes::SubTag {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "bunkr-fileid".to_string(),
                description: Some(
                    "A unique file id for bunkr. Sometimes its called a slug internally"
                        .to_string(),
                ),
            },
            tag: fileid.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            limit_to: None,
        }),
    };

    let file_id = sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "bunkr-fileid".to_string(),
            description: Some(
                "A unique file id for bunkr. Sometimes its called a slug internally".to_string(),
            ),
        },
        tag: fileid.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to,
    };
    vec![tag_out, file_id]
}

/// Determines if we should download a file
fn should_download_file(file_id: &str) -> bool {
    if let Some(file_nsid) = client::namespace_get("bunkr-fileid".to_string())
        && let Some(tag_id) = client::tag_get_name(file_id.to_string(), file_nsid)
    {
        return client::relationship_get_fileid(tag_id).is_empty();
    }
    true
}

#[derive(Deserialize, Debug)]
struct ApiFileDownloadResponse {
    #[allow(dead_code)]
    encrypted: bool,
    timestamp: u64,
    url: String,
}

/// Handles downloading a file and returns a Vec if we're good
fn download_file(url: &String, pretty_url: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();

    let mut response;
    let mut cnt = 0;
    loop {
        //let sleep = rand::thread_rng().gen_range(2..10);
        //client::log(format!("scraper-bunkr: Waiting {} seconds for ratelimit random", sleep));
        //std::thread::sleep(Duration::from_secs(sleep));
        client::log(format!("scraper-bunkr: Downloading {}", pretty_url));

        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(50)))
            .build()
            .into();

        // 2. Use the agent to perform the GET request
        let response_temp = agent
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Referer", "https://get.bunkrr.su/")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:146.0) Gecko/20100101 Firefox/146.0",
            )
            .call();

        if let Ok(response_out) = response_temp {
            response = response_out;
            break;
        }
        if cnt == 3 {
            return Err(anyhow!("Too many retries"));
        }
        cnt += 1;
    }

    let final_url = response.get_uri().to_string();

    let body = response.body();
    let content_length = body.content_length().unwrap_or(0);
    let should_copy = content_length != 0 && !final_url.contains("maint.mp4");

    if should_copy {
        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 8192]; // 8 KB chunks
        let mut reader = response.body_mut().as_reader();

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break; // EOF
            }
            out.write_all(&buffer[..n])?;
            downloaded += n as u64;

            // Print progress
            if content_length != 0 {
                let progress = downloaded as f64 / content_length as f64 * 100.0;
                print!("\rDownloading: {:.2}%", progress);
            } else {
                print!("\rDownloaded {} bytes", downloaded);
            }
            std::io::stdout().flush().unwrap();
        }

        println!("\nDownload complete: {}", pretty_url);
        return Ok(out);
    } else if final_url.contains("maint.mp4") {
        client::log_no_print(
            "scraper-bunkr: Could not download file because the servers are in maintenance mode"
                .to_string(),
        );

        return Err(anyhow!("Server is in maintence mode"));
    } else {
        client::log_no_print(
            "scraper-bunkr: Could not download file as the file is empty".to_string(),
        );
    }

    Err(anyhow!("File Download Err"))
}

///
/// Decrypts the url from the api
/// from: https://github.com/sn0w12/bunkr-client
///
fn decrypt_url(input: ApiFileDownloadResponse) -> Result<String> {
    let divisor = 3600.0;
    let suffix = ((input.timestamp as f64) / divisor).floor() as i64;

    let key = format!("SECRET_KEY_{}", suffix);

    // Base64 decode
    let bytes = general_purpose::STANDARD.decode(input.url)?;

    // XOR decrypt with key
    let key_bytes = key.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        output.push(b ^ key_bytes[i % key_bytes.len()]);
    }

    // Decode as UTF-8
    let decoded = String::from_utf8(output)?;
    Ok(decoded)
}

/// Extracts the page number from the string
fn extract_page(href: &str) -> Option<u32> {
    href.split("page=").nth(1)?.parse().ok()
}
