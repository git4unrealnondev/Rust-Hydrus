use atrium_api::agent::Agent;
use atrium_api::agent::atp_agent::CredentialSession;
use atrium_api::agent::atp_agent::store::MemorySessionStore;
use atrium_api::app::bsky::actor::defs::ProfileViewBasic;
use atrium_api::app::bsky::feed::defs::{PostViewData, PostViewEmbedRefs};
use atrium_api::app::bsky::feed::get_author_feed::{self};
use atrium_api::types::string::{AtIdentifier, Handle};
use atrium_xrpc_client::reqwest::ReqwestClient;
use ipld_core::ipld::Ipld;
use m3u8_rs::Playlist;
use reqwest::blocking;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::sharedtypes::DEFAULT_PRIORITY;
#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[path = "../../../src/client.rs"]
mod client;
#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

#[path = "../../../generated/client_api.rs"]
pub mod web_api;

pub const SITE_NAME: &str = "bsky.app";

/// Parses an authors profile
fn parse_profile_view_basic(
    raw_profile: &ProfileViewBasic,
    object: &mut sharedtypes::ScraperObject,
) -> String {
    let handle = sharedtypes::SubTag {
        tag: raw_profile.data.handle.as_str().to_string(),
        namespace: sharedtypes::GenericNamespaceObj {
            name: "BSKY_Handle".into(),
            description: Some("The unique handle of a bsky author.".into()),
        },
        ..Default::default()
    };

    let mut tags = HashSet::new();

    if let Some(ref display_name) = raw_profile.data.display_name {
        tags.insert(sharedtypes::TagObject {
            tag: display_name.to_string(),
            tag_type: sharedtypes::TagType::Normal,
            namespace: sharedtypes::GenericNamespaceObj {
                name: "BSKY_Display_Name".into(),
                description: Some("The display name for a bsky Author".into()),
            },
            relates_to: Some(handle.clone()),
        });
    }

    if let Some(ref create_time) = raw_profile.data.created_at {
        tags.insert(sharedtypes::TagObject {
            tag: create_time.as_ref().timestamp_millis().to_string(),
            tag_type: sharedtypes::TagType::Normal,
            namespace: sharedtypes::GenericNamespaceObj {
                name: "BSKY_Account_Creation".into(),
                description: Some("When a Author's account was created on bsky".into()),
            },
            relates_to: Some(handle.clone()),
        });
    }

    if let Some(ref avatar_image) = raw_profile.data.avatar {
        let mut file = HashSet::new();
        file.insert(sharedtypes::FileObject {
            source: Some(sharedtypes::FileSource::Url(vec![avatar_image.to_string()])),
            hash: sharedtypes::HashesSupported::None,
            tag_list: vec![sharedtypes::FileTagAction {
                operation: sharedtypes::TagOperation::Add,
                tags: vec![sharedtypes::TagObject {
                    tag: raw_profile.data.handle.as_str().to_string(),
                    namespace: sharedtypes::GenericNamespaceObj {
                        name: "BSKY_Handle".into(),
                        description: Some("The unique handle of a bsky author.".into()),
                    },
                    ..Default::default()
                }],
            }],
            ..Default::default()
        });
        object.files.extend(file);
    }

    object.tags.extend(tags);

    raw_profile.data.handle.as_str().to_string()
}

fn parse_feed_view_post_data(
    raw_post: &PostViewData,
    object: &mut sharedtypes::ScraperObject,
) -> Result<(), Box<dyn std::error::Error>> {
    let author_handle = parse_profile_view_basic(&raw_post.author, object);

    let unique_post_id = raw_post
        .uri
        .split('/')
        .next_back()
        .ok_or("CANNOT PARSE UNIQUE ID")?;

    let relates_to = Some(sharedtypes::SubTag {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "BSKY_Post_Id".into(),
            description: Some("A unique post id thats inside of bsky.".into()),
        },
        tag: unique_post_id.to_string(),
        limit_to: Some(sharedtypes::Tag {
            tag: author_handle.to_string(),
            namespace: sharedtypes::GenericNamespaceObj {
                name: "BSKY_Handle".into(),
                description: Some("The unique handle of a bsky author.".into()),
            },
        }),
        tag_type: sharedtypes::TagType::Normal,
    });

    let mut tags = HashSet::new();
    if let atrium_api::types::Unknown::Object(ref map) = raw_post.record
        && let Some(text) = map.get("text")
    {
        let tag = match &**text {
            Ipld::String(s) => s.to_string(),
            Ipld::Integer(s) => s.to_string(),
            _ => panic!("not a string"),
        };

        tags.insert(sharedtypes::TagObject {
            namespace: sharedtypes::GenericNamespaceObj {
                name: "BSKY_Post_Text".into(),
                description: Some("The text from a post in bsky".into()),
            },
            tag,
            relates_to: relates_to.clone(),
            ..Default::default()
        });
    }
    tags.insert(sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "BSKY_Post_Timestamp".into(),
            description: Some("The timestamp a post was created in bsky".into()),
        },
        tag: raw_post.indexed_at.as_ref().timestamp_millis().to_string(),
        relates_to: relates_to.clone(),
        ..Default::default()
    });

    object.tags.extend(tags);

    let mut should_check_embed = true;
    if let Some(nsid) = client::namespace_get("BSKY_Post_Id".to_string())
        && client::tag_get_name(unique_post_id.into(), nsid).is_some()
    {
        should_check_embed = false;
    }

    let mut file = HashSet::new();
    if let Some(ref embed) = raw_post.embed
        && let atrium_api::types::Union::Refs(refs) = embed
    {
        match refs {
            atrium_api::app::bsky::feed::defs::PostViewEmbedRefs::AppBskyEmbedVideoView(
                viewdata,
            ) => {
                if should_check_embed {
                    let response = blocking::get(viewdata.playlist.clone())?;
                    let bytes: &[u8] = &response.bytes()?;
                    let parsed = m3u8_rs::parse_playlist_res(bytes).unwrap();

                    if let Playlist::MasterPlaylist(master_playlist) = parsed {
                        for variant in master_playlist.variants.into_iter() {
                            let url = url::Url::parse(&viewdata.playlist)?;

                            // Gets the playlist info
                            let url = url.join(&variant.uri.to_string())?;
                            let response = blocking::get(url.to_string())?;
                            let bytes: &[u8] = &response.bytes()?;

                            let mut storage = Vec::new();

                            let mut last_url = String::new();
                            let parsed = m3u8_rs::parse_playlist_res(bytes).unwrap();
                            if let Playlist::MediaPlaylist(media_playlist) = parsed {
                                for variant in media_playlist.segments {
                                    let url = url.join(&variant.uri)?;

                                    last_url = url.to_string();

                                    let response = blocking::get(url)?;

                                    storage.extend(response.bytes()?);
                                }
                            }
                            let resolution = last_url
                                .split('/')
                                .filter(|s| !s.is_empty())
                                .nth_back(1)
                                .ok_or("CANNOT FIND VIDEO RESOLUTION")?;
                            file.insert(sharedtypes::FileObject {
                                source: Some(sharedtypes::FileSource::Bytes(storage)),
                                hash: sharedtypes::HashesSupported::None,
                                tag_list: vec![sharedtypes::FileTagAction {
                                    operation: sharedtypes::TagOperation::Add,
                                    tags: vec![
                                        sharedtypes::TagObject {
                                            namespace: sharedtypes::GenericNamespaceObj {
                                                name: "BSKY_Post_Id".into(),
                                                description: Some(
                                                    "A unique post id thats inside of bsky.".into(),
                                                ),
                                            },
                                            tag: unique_post_id.to_string(),
                                            relates_to: None,
                                            tag_type: sharedtypes::TagType::Normal,
                                        },
                                        sharedtypes::TagObject {
                                            namespace: sharedtypes::GenericNamespaceObj {
                                                name: "BSKY_Video_Resolution".into(),
                                                description: Some(
                                                    "A video's resolution from bsky..".into(),
                                                ),
                                            },

                                            tag: resolution.to_string(),

                                            tag_type: sharedtypes::TagType::Normal,
                                            relates_to: relates_to.clone(),
                                        },
                                    ],
                                }],
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            PostViewEmbedRefs::AppBskyEmbedImagesView(viewdata) => {
                for (cnt, image) in viewdata.data.images.clone().into_iter().enumerate() {
                    file.insert(sharedtypes::FileObject {
                        source: Some(sharedtypes::FileSource::Url(vec![image.data.fullsize])),
                        hash: sharedtypes::HashesSupported::None,
                        tag_list: vec![sharedtypes::FileTagAction {
                            operation: sharedtypes::TagOperation::Add,
                            tags: vec![
                                sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: "BSKY_Post_Id".into(),
                                        description: Some(
                                            "A unique post id thats inside of bsky.".into(),
                                        ),
                                    },
                                    tag: unique_post_id.to_string(),
                                    relates_to: None,
                                    tag_type: sharedtypes::TagType::Normal,
                                },
                                sharedtypes::TagObject {
                                    namespace: sharedtypes::GenericNamespaceObj {
                                        name: "BSKY_Image_Position".into(),
                                        description: Some(
                                            "A image files position in the post".into(),
                                        ),
                                    },

                                    tag: cnt.to_string(),

                                    tag_type: sharedtypes::TagType::Normal,
                                    relates_to: relates_to.clone(),
                                },
                            ],
                        }],
                        ..Default::default()
                    });
                }
            }
            _ => {}
        }
    }
    object.files.extend(file);
    dbg!(object.files.len());
    Ok(())
}

///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let scraper = sharedtypes::GlobalPluginScraper {
        name: "bsky".into(),
        storage_type: Some(sharedtypes::ScraperOrPlugin::Scraper(
            sharedtypes::ScraperInfo {
                ratelimit: (10, Duration::from_secs(1)),
                sites: vec_of_strings!("bsky", "bsky.app"),
                priority: DEFAULT_PRIORITY,
                num_threads: Some(1),
                modifiers: vec![],
            },
        )),

        login_type: vec![(
            "bsky".to_string(),
            sharedtypes::LoginType::Login("Bsky_User_Login".to_string(), None),
            sharedtypes::LoginNeed::Required,
            Some("Username and Password for bsky goes in here.".to_string()),
            false,
        )],

        should_handle_text_scraping: true,
        ..Default::default()
    };

    vec![scraper]
}

///
/// Dumps a list of urls to scrape
///
#[unsafe(no_mangle)]
pub fn url_dump(
    params: &[sharedtypes::ScraperParam],
    _scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperDataReturn> {
    let mut user_data = BTreeMap::new();

    // Gets the post handle and user id from url just some parsing
    for param in params {
        if let sharedtypes::ScraperParam::Url(url) = param
            && url.contains("bsky.app")
            && let Ok(url) = url::Url::parse(url)
        {
            let segments: Vec<_> = url.path_segments().unwrap().collect();

            if segments.len() >= 3 {
                let handle = segments[1];
                let post_id = segments[3];

                if !handle.is_empty() {
                    user_data.insert("post_handle".into(), handle.to_string());
                }
                if !post_id.is_empty() {
                    user_data.insert("post_id".into(), post_id.to_string());
                }
            } else {
                let post_handle = segments[1];

                if !post_handle.is_empty() {
                    user_data.insert("post_handle".into(), post_handle.to_string());
                }
            }
        }
    }

    let mut params = params.to_vec();
    params.push(sharedtypes::ScraperParam::Url("".to_string()));
    vec![sharedtypes::ScraperDataReturn {
        job: sharedtypes::DbJobsObj {
            site: SITE_NAME.into(),
            param: params.to_vec(),
            jobmanager: sharedtypes::DbJobsManager {
                jobtype: sharedtypes::DbJobType::Scraper,
                ..Default::default()
            },
            user_data,

            ..Default::default()
        },
        ..Default::default()
    }]
}

fn get_login_info(
    params: &[sharedtypes::ScraperParam],
) -> Option<sharedtypes::LoginUsernameOrPassword> {
    for param in params {
        if let sharedtypes::ScraperParam::Login(sharedtypes::LoginType::Login(_, Some(login))) =
            param
        {
            return Some(login.clone());
        }
    }
    None
}

fn run_logic(
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperDataReturn,
    object: &mut sharedtypes::ScraperObject,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = CredentialSession::new(
        ReqwestClient::new("https://bsky.social"),
        MemorySessionStore::default(),
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Some(login) = get_login_info(params) {
        rt.block_on(session.login(
            &login.username.expose_secret(),
            &login.password.expose_secret(),
        ))?;
    }
    let agent = Arc::new(Agent::new(session));

    let mut post_info_parameters: Vec<PostViewData> = Vec::new();
    if !scraperdata.job.user_data.contains_key("post_id")
        && let Some(handle_text) = scraperdata.job.user_data.get("post_handle")
    {
        let handle = Handle::new(handle_text.into())?;

        let mut cursor = None;
        loop {
            let response = rt.block_on(agent.api.app.bsky.feed.get_author_feed(
                atrium_api::app::bsky::feed::get_author_feed::Parameters {
                    data: get_author_feed::ParametersData {
                        actor: AtIdentifier::Handle(handle.clone()),
                        cursor,
                        filter: None,
                        limit: Some(atrium_api::types::LimitedNonZeroU8::try_from(50)?),
                        include_pins: Some(true),
                    },
                    extra_data: ipld_core::ipld::Ipld::Null,
                },
            ))?;

            cursor = response.cursor.clone();
            post_info_parameters
                .extend(response.data.feed.iter().map(|f| f.data.post.data.clone()));

            if response.cursor.is_none() {
                break;
            }

            dbg!(post_info_parameters.len());
        }
    }

    if let Some(post_text) = scraperdata.job.user_data.get("post_id")
        && let Some(handle_text) = scraperdata.job.user_data.get("post_handle")
    {
        let profile = rt.block_on(agent.api.app.bsky.actor.get_profile(
            atrium_api::app::bsky::actor::get_profile::Parameters {
                data: atrium_api::app::bsky::actor::get_profile::ParametersData {
                    actor: AtIdentifier::Handle(Handle::new(handle_text.to_string())?),
                },
                extra_data: ipld_core::ipld::Ipld::Null,
            },
        ))?;

        let user_did = profile.data.did.trim().to_string();

        let response = rt.block_on(agent.api.app.bsky.feed.get_posts(
            atrium_api::app::bsky::feed::get_posts::Parameters {
                data: atrium_api::app::bsky::feed::get_posts::ParametersData {
                    uris: vec![format!("at://{user_did}/app.bsky.feed.post/{post_text}")],
                },
                extra_data: ipld_core::ipld::Ipld::Null,
            },
        ))?;

        post_info_parameters.extend(response.data.posts.iter().map(|f| f.data.clone()));
    }

    for post_view in post_info_parameters.iter() {
        parse_feed_view_post_data(post_view, object).unwrap();
    }

    Ok(())
}

#[unsafe(no_mangle)]
pub fn text_scraping(
    _url_input: &str,
    params: &[sharedtypes::ScraperParam],
    scraperdata: &sharedtypes::ScraperDataReturn,
) -> Vec<sharedtypes::ScraperReturn> {
    let mut object = sharedtypes::ScraperObject {
        ..Default::default()
    };

    run_logic(params, scraperdata, &mut object).unwrap();

    vec![sharedtypes::ScraperReturn::Data(object)]
}
