#![allow(dead_code)]
#![allow(unused_variables)]
use crate::Main;
use crate::globalload::GlobalLoad;
use crate::jobs::Jobs;
use crate::logging;
use anyhow::Context;
use rayon::ThreadPool;

use crate::Mutex;
use crate::RwLock;
use crate::types;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};
use sharedtypes;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::{
    io::{self, BufReader, prelude::*},
    sync::mpsc::Sender,
};

pub struct PluginIpcInteract {
    db_interface: DbInteract,
    heavy_processing_pool: Arc<ThreadPool>,
}

impl PluginIpcInteract {
    pub fn new(
        main_db: Main,
        globalload: GlobalLoad,
        jobs: Arc<Jobs>,
        heavy_processing_pool: Arc<ThreadPool>,
    ) -> Self {
        PluginIpcInteract {
            db_interface: DbInteract {
                database: main_db,
                globalload,
                jobmanager: jobs,
            },
            heavy_processing_pool,
        }
    }

    /// Spawns a clean listener context directly natively inside your existing Tokio runtime.
    pub async fn spawn_listener(&mut self, main_db: Main) -> anyhow::Result<()> {
        use warp::Filter;

        let db = main_db.clone();

        let name = types::SOCKET_NAME
            .to_ns_name::<GenericNamespaced>()
            .unwrap();

        let opt = ListenerOptions::new().name(name);

        // Bind our listener.
        let listener = opt.create_sync().unwrap();

        logging::info_log(format!("IPC Server running at {}", types::SOCKET_NAME));

        // Setup warp routes
        let routes_with_fallback =
            main_db
                .clone()
                .get_filters()
                .recover(|err: warp::Rejection| async move {
                    if err.is_not_found() {
                        Ok::<_, warp::Rejection>(warp::reply::with_status(
                            String::from("404 Not Found"), // Use an owned String here
                            warp::http::StatusCode::NOT_FOUND,
                        ))
                    } else {
                        Ok::<_, warp::Rejection>(warp::reply::with_status(
                            String::from("500 Internal Server Error"), // Use an owned String here
                            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                        ))
                    }
                });

        // Spawn API Server directly on your existing background runtime context
        let mut api_url = db.get_api_url().url;
        let tokio_listener = loop {
            match tokio::net::TcpListener::bind(api_url).await {
                Ok(l) => break l,
                Err(e) => {
                    logging::error_log(&format!("Failed to bind server: {}", e));
                    api_url.set_port(api_url.port() + 1);
                }
            }
        };

        // Native Warp serving on standard tokio listener
        tokio::spawn(async move {
            warp::serve(routes_with_fallback)
                .incoming(tokio_listener)
                .run()
                .await;
        });

        logging::info_log(&format!("Starting API server on {}", api_url));

        // Stream routing loop mapping automatically to tasks
        let globalload = self.db_interface.globalload.clone();
        let jobmanager = self.db_interface.jobmanager.clone();
        self.heavy_processing_pool.spawn(move || {
            for stream in listener.incoming().flatten() {
                let db_clone = main_db.clone();
                let global_clone = globalload.clone();
                let job_clone = jobmanager.clone();
                handle_client(stream, db_clone, global_clone, job_clone);
            }
        });
        Ok(())
    }
}

fn handle_client(
    stream: LocalSocketStream,
    db: Main,
    globalload: GlobalLoad,
    jobmanager: Arc<Jobs>,
) {
    std::thread::spawn(move || {
        let buffer = [0u8; 1024];
        let mut conn = BufReader::new(stream);
        let plugin_supportedrequests = match types::recieve(&mut conn) {
            Ok(out) => out,
            Err(err) => {
                logging::error_log(err.to_string());
                return;
            }
        };

        /* logging::log(format!(
            "Threadded - Worker {} got a connection {:?}",
            worker_id, &plugin_supportedrequests
        ));*/

        // Default
        match plugin_supportedrequests {
            types::SupportedRequests::Database(db_actions) => {
                let data = dbactions_to_function(db_actions, db, globalload, jobmanager);
                types::send_preserialize(&data, &mut conn);
            }
            types::SupportedRequests::PluginCross(_plugindata) => {}
        }
    });
}

#[derive(Clone)]
struct DbInteract {
    database: Main,
    globalload: GlobalLoad,
    jobmanager: Arc<Jobs>,
}

impl DbInteract {}

/// Storage object for database interactions with the plugin system
/// Helper function to return data about a passed object into size and bytes array.
fn data_size_to_b<T: bitcode::Encode + ?Sized>(data_object: &T) -> Vec<u8> {
    // let bytd = types::x_to_bytes(tmp).to_vec();
    bitcode::encode(data_object)
}

/// Packages functions from the DB into their self owned versions before packaging
/// them as bytes to get sent accross IPC to the other software. So far things are
/// pretty mint.
pub fn dbactions_to_function(
    dbaction: types::SupportedDBRequests,
    database: Main,
    mut globalload: GlobalLoad,
    jobmanager: Arc<Jobs>,
) -> Vec<u8> {
    match dbaction {
        types::SupportedDBRequests::GetFileIdsWhereExtensionIs(file_extension_type) => {
            let file_ids = match file_extension_type {
                sharedtypes::FileExtensionType::Image => database.extensions_images_get_fileid(),
                sharedtypes::FileExtensionType::Video => database.extensions_videos_get_fileid(),
            };
            data_size_to_b(&file_ids)
        }
        types::SupportedDBRequests::GetRelationshipTagidWhereNamespace((
            namespace_id,
            count,
            dir,
        )) => data_size_to_b(&database.relationship_get_tagid_where_namespace_count_sql(
            &namespace_id,
            &count,
            &dir,
        )),
        types::SupportedDBRequests::GetRelationshipFileidWhereNamespace((
            namespace_id,
            count,
            dir,
        )) => data_size_to_b(&database.relationship_get_fileid_where_namespace_count_sql(
            &namespace_id,
            &count,
            &dir,
        )),

        types::SupportedDBRequests::CondenseTags() => {
            let unwrappy = database;
            unwrappy.condense_tags();
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::TagDelete(tag_id) => {
            let unwrappy = database;
            unwrappy.delete_tag(&tag_id);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::MigrateRelationship((file_id, old_tag_id, new_tag_id)) => {
            let unwrappy = database;
            unwrappy.migrate_relationship_file_tag(&file_id, &old_tag_id, &new_tag_id);
            data_size_to_b(&true)
        }

        types::SupportedDBRequests::MigrateTag((old_tag_id, new_tag_id)) => {
            let unwrappy = database;
            unwrappy.migrate_tag(&old_tag_id, &new_tag_id);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::PutFileNoBlock((mut file, ratelimit)) => {
            /*  let mut global_pluginscraper = sharedtypes::return_default_globalpluginparser();
            global_pluginscraper.name = "InternalFileAdd".to_string();
            let ratelimiter_obj = download::create_ratelimiter(ratelimit, &0, &0);
            let manageeplugin = globalload.clone();
            let client = Arc::new(RwLock::new(download::client_create(vec![], false)));
            let jobstorage = jobmanager.clone();
            let database = database.clone();
            let thread = thread::spawn(move || {
                download::main_file_loop(
                    &mut file,
                    database,
                    ratelimiter_obj,
                    manageeplugin,
                    client,
                    jobstorage,
                    &global_pluginscraper,
                    &0,
                    &0,
                    uisender,
                    &crate::ui::ui::FileStorage {
                        internal_id: 0,
                        status: crate::ui::ui::FilesStatus::Waiting,
                        hash: sharedtypes::HashesSupported::None,
                    },
                    Arc::new(RwLock::new(vec![])),
                );
            });*/

            data_size_to_b(&true)
        }

        types::SupportedDBRequests::PutFile((mut file, ratelimit)) => {
            /*  let mut global_pluginscraper = sharedtypes::return_default_globalpluginparser();
            global_pluginscraper.name = "InternalFileAdd".to_string();

            let ratelimiter_obj = download::create_ratelimiter(ratelimit, &0, &0);
            let manageeplugin = globalload.clone();
            let client = Arc::new(RwLock::new(download::client_create(vec![], false)));
            let jobstorage = jobmanager.clone();
            download::main_file_loop(
                &mut file,
                database.clone(),
                ratelimiter_obj,
                manageeplugin,
                client,
                jobstorage,
                &global_pluginscraper,
                &0,
                &0,
                uisender,
                &crate::ui::ui::FileStorage {
                    internal_id: 0,
                    status: crate::ui::ui::FilesStatus::Waiting,
                    hash: sharedtypes::HashesSupported::None,
                },
                Arc::new(RwLock::new(vec![])),
            );*/

            data_size_to_b(&true)
        }
        types::SupportedDBRequests::PutJob(job) => {
            let unwrappy = database;
            let _ = unwrappy.jobs_add_new(job);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::GetNamespaceIDsAll => {
            let unwrappy = database;
            data_size_to_b(&unwrappy.namespace_keys())
        }

        types::SupportedDBRequests::GetFileExt(ext_id) => {
            let unwrappy = database;
            option_to_bytes(unwrappy.extension_get_string(&ext_id).as_ref())
        }
        types::SupportedDBRequests::GetFileRaw(ext_id) => {
            let unwrappy = database;
            option_to_bytes(unwrappy.file_get_id(&ext_id).as_ref())
        }

        types::SupportedDBRequests::GetJob(id) => {
            let unwrappy = database;
            option_to_bytes(unwrappy.jobs_get(&id).as_ref())
        }
        types::SupportedDBRequests::ParentsPut(parent) => {
            let unwrappy = database;
            let out = &unwrappy.parents_add(parent);
            data_size_to_b(out)
        }
        types::SupportedDBRequests::ParentsGet((parentswitch, id)) => {
            let unwrappy = database;
            let out = match parentswitch {
                sharedtypes::ParentsType::Tag => &unwrappy.parents_tagid_tag_get(&id),
                sharedtypes::ParentsType::Rel => &unwrappy.parents_relate_tag_get(&id),
                sharedtypes::ParentsType::LimitTo => &unwrappy.parents_limitto_tag_get(&id),
            };

            data_size_to_b(out)
        }
        types::SupportedDBRequests::ParentsDelete(parentobj) => {
            let unwrappy = database;
            unwrappy.parents_selective_remove(&parentobj);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::GetFileLocation(id) => {
            let unwrappy = database;
            let tmep = unwrappy.get_file(&id);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::FilterNamespaceById((ids, namespace_id)) => {
            let mut out: HashSet<u64> = HashSet::new();
            let unwrappy = database;
            for each in ids.iter() {
                if unwrappy.namespace_contains_id(&namespace_id, each) {
                    out.insert(*each);
                }
            }
            data_size_to_b(&out)
        }
        types::SupportedDBRequests::ReloadLoadedPlugins() => {
            //let mut plugin = globalload.write();
            //plugin.reload_loaded_plugins();
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::NamespaceContainsId(namespaceid, tagid) => {
            let unwrappy = database;
            let tmep = unwrappy.namespace_contains_id(&namespaceid, &tagid);
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::PluginCallback(func_name, version, input_data) => {
            let plugin = globalload;
            let out = plugin.external_plugin_call(&func_name, &version, &input_data);
            data_size_to_b(&out)
        }
        types::SupportedDBRequests::GetFileByte(id) => {
            let unwrappy = database;
            let tmep = unwrappy.get_file_bytes(&id);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::Search((search, limit, offset)) => {
            let unwrappy = database;
            let tmep = unwrappy.search_db_files(search, limit);
            //let tmep = unwrappy.search_db_files(search, limit, offset);
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::GetTagId(id) => {
            let unwrappy = database;
            let tmep = unwrappy.tag_id_get(&id);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::Logging(log) => {
            logging::info_log(&log);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::LoggingNoPrint(log) => {
            logging::log(&log);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::RelationshipAdd(file, tag) => {
            let unwrappy = database;
            unwrappy.add_relationship(&file, &tag);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::RelationshipRemove(file, tag) => {
            let unwrappy = database;

            unwrappy.delete_relationship(&file, &tag);
            data_size_to_b(&true)
        }

        types::SupportedDBRequests::PutTag(tags, namespace_id, id) => {
            let unwrappy = database;
            let tmep = unwrappy.tag_add(&tags, namespace_id, id);
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::PutTagRelationship(fid, tags, namespace_id, id) => {
            let unwrappy = database;
            let tmep = unwrappy.tag_add(&tags, namespace_id, id);
            unwrappy.add_relationship(&fid, &tmep);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::GetDBLocation() => {
            let unwrappy = database;
            let tmep = unwrappy.location_get();
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::SettingsSet(name, pretty, num, param) => {
            let unwrappy = database;
            unwrappy.setting_add(name, pretty, num, param);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::RelationshipGetTagid(id) => {
            let unwrappy = database;
            let tmep = unwrappy.relationship_get_tagid(&id);
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::GetFile(id) => {
            let unwrappy = database;
            let tmep = unwrappy.file_get_id(&id);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::RelationshipGetFileid(id) => {
            let unwrappy = database;
            let tmep = unwrappy.relationship_get_fileid(&id);
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::SettingsGetName(id) => {
            let unwrappy = database;
            let tmep = unwrappy.settings_get_name(&id);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::GetTagName((name, namespace)) => {
            let unwrappy = database;
            let tmep = unwrappy.tag_get_name(name, namespace);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::GetFileHash(name) => {
            let unwrappy = database;
            let tmep = unwrappy.file_get_hash(&name);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::CreateNamespace(name, description) => {
            let unwrappy = database;
            let out = unwrappy.namespace_add(&name, &description);

            data_size_to_b(&out)
        }
        types::SupportedDBRequests::GetNamespace(name) => {
            let unwrappy = database;
            let tmep = unwrappy.namespace_get(&name);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::Testu64() => {
            let test: u64 = 32;
            data_size_to_b(&test)
        }
        types::SupportedDBRequests::GetNamespaceString(id) => {
            let unwrappy = database;
            let tmep = unwrappy.namespace_get_string(&id);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::LoadTable(table) => {
            let unwrappy = database;
            unwrappy.load_table(&table);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::TransactionFlush() => {
            let unwrappy = database;

            data_size_to_b(&true)
        }
        types::SupportedDBRequests::GetNamespaceTagIDs(id) => {
            let unwrappy = database;
            let tmep = unwrappy.namespace_get_tagids(&id);
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::GetFileListId() => {
            let unwrappy = database;
            let tmep = unwrappy.file_get_list_id();
            data_size_to_b(&tmep)
        }
        types::SupportedDBRequests::GetFileListAll() => {
            let unwrappy = database;
            let tmep = unwrappy.file_get_list_all();
            data_size_to_b(&tmep)
            //bitcode::serialize(&tmep).unwrap()
        }
        types::SupportedDBRequests::ReloadRegex => {
            globalload.reload_regex();

            data_size_to_b(&true)
        }
    }
}

/// Turns an Option<&T> into a bytes object.
fn option_to_bytes<T: bitcode::Encode + Clone>(input: Option<&T>) -> Vec<u8> {
    match input {
        None => data_size_to_b(&input.cloned()),
        Some(item) => {
            let i: Option<T> = Some(item.clone());
            data_size_to_b(&i)
        }
    }
}
