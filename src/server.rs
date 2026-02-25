#![allow(dead_code)]
#![allow(unused_variables)]
use crate::Main;
use crate::database;
use crate::download;
use crate::globalload::GlobalLoad;
use crate::jobs::Jobs;
use crate::logging;
use crate::sharedtypes;
use crate::threading;
use anyhow::Context;

use crate::RwLock;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::Mutex;
use std::{
    io::{self, BufReader, prelude::*},
    sync::mpsc::Sender,
};

use crate::types;

pub fn main(notify: Sender<()>) -> anyhow::Result<()> {
    // Define a function that checks for errors in incoming connections. We'll use
    // this to filter through connections that fail on initialization for one reason
    // or another.
    fn handle_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
        match conn {
            Ok(c) => Some(c),
            Err(e) => {
                println!("Incoming connection failed: {}", e);
                None
            }
        }
    }

    // Pick a name. There isn't a helper function for this, mostly because it's
    // largely unnecessary: in Rust, `match` is your concise, readable and expressive
    // decision making construct.
    let socketname = types::SOCKET_NAME;
    let name = types::SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .unwrap();

    // Configures the listener
    let opt = ListenerOptions::new().name(name);

    // Bind our listener.
    let listener = match opt.create_sync() {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            // One important problem that is easy to handle improperly (or not at all) is the
            // "corpse sockets" that are left when a program that uses a file-type socket name
            // terminates its socket server without deleting the file. There's no single
            // strategy for handling this kind of address-already-occupied error. Services
            // that are supposed to only exist as a single instance running on a system should
            // check if another instance is actually running, and if not, delete the socket
            // file. In this example, we leave this up to the user, but in a real application,
            // you usually don't want to do that.
            eprintln!(
                "\
Error: could not start server because the socket file is occupied. Please check if {} is in use by \
another process and try again.",
                types::SOCKET_NAME
            );
            return Err(e.into());
        }
        x => x?,
    };
    println!("Server running at {}", types::SOCKET_NAME);

    // Stand-in for the syncronization used, if any, between the client and the server.
    let _ = notify.send(());

    // Preemptively allocate a sizeable buffer for reading at a later moment. This
    // size should be enough and should be easy to find for the allocator. Since we
    // only have one concurrent client, there's no need to reallocate the buffer
    // repeatedly.
    for conn in listener.incoming().filter_map(handle_error) {
        let buffer = &mut [b'0', b'0'];
        let mut bufstr = String::new();
        let coms_struct = types::Coms {
            com_type: types::EComType::BiDirectional,
            control: types::EControlSigs::Send,
        };
        let b_struct = bincode::encode_to_vec(&coms_struct, bincode::config::standard()).unwrap();

        // Wrap the connection into a buffered reader right away so that we could read a
        // single line out of it.
        let mut conn = BufReader::new(conn);
        println!("Incoming connection!");

        // Since our client example writes first, the server should read a line and only
        // then send a response. Otherwise, because reading and writing on a connection
        // cannot be simultaneous without threads or async, we can deadlock the two
        // processes by having both sides wait for the write buffer to be emptied by the
        // other.
        conn.read(buffer).context("Socket receive failed").unwrap();

        // Now that the read has come through and the client is waiting on the server's
        // write, do it. (`.get_mut()` is to get the writer, `BufReader` doesn't implement
        // a pass-through `Write`.) conn.get_mut().write_all(b"Hello from server!\n")?;
        // Print out the result, getting the newline for free!
        let instruct: types::Coms =
            bincode::decode_from_slice(&buffer[..], bincode::config::standard())
                .unwrap()
                .0;

        // std::mem::forget(buffer.clone());
        match instruct.control {
            types::EControlSigs::Send => {
                bufstr.clear();
                conn.read_line(&mut bufstr)
                    .context("Socket receive failed")
                    .unwrap();

                // bufstr.clear();
                conn.get_mut()
                    .write_all(&b_struct)
                    .context("Socket send failed")
                    .unwrap();
                bufstr.clear();
                conn.read_line(&mut bufstr)
                    .context("Socket receive failed")
                    .unwrap();
            }
            types::EControlSigs::Halt => {}
            types::EControlSigs::Break => {}
        }
        // Let's add an exit condition to shut the server down gracefully. if buffer ==
        // "stop\n" { break; } Clear the buffer so that the next iteration will display
        // new data instead of messages stacking on top of one another. buffer.clear();
    }
    Ok(())
}

/// Storage for database interaction object for IPC
pub struct PluginIpcInteract {
    db_interface: DbInteract,
}

/// This is going to be the main way to talk to the plugin system and stuffins.
impl PluginIpcInteract {
    pub fn new(main_db: Main, globalload: GlobalLoad, jobs: Arc<RwLock<Jobs>>) -> Self {
        PluginIpcInteract {
            db_interface: DbInteract {
                database: main_db,
                globalload,
                jobmanager: jobs.clone(),
            },
        }
    }

    /// Spawns a listener for events.
    pub fn spawn_listener(&mut self, main_db: Main) -> anyhow::Result<()> {
        // Define a function that checks for errors in incoming connections. We'll use
        // this to filter through connections that fail on initialization for one reason
        // or another.
        fn handle_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
            match conn {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("Incoming connection failed: {}", e);
                    None
                }
            }
        }use warp::Filter;
use warp::Rejection;
use warp::Reply;

async fn handle_rejection(err: Rejection) -> Result<impl Reply, warp::Rejection> {
    if err.is_not_found() {
        // If the route was not found, return a 404 response
        Ok(warp::reply::with_status(
            "404 Not Found",
            warp::http::StatusCode::NOT_FOUND,
        ))
    } else {
        // Handle any other errors (e.g., internal server error)
        Ok(warp::reply::with_status(
            "500 Internal Server Error",
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}


        let num_threads: u64 = 200;
        let thread_list = Arc::new(Mutex::new(Vec::new()));
        let db = main_db.clone();
        let mut handles = Vec::new();

        for worker_id in 0..num_threads {
            let db = db.clone();
            let thread_list = thread_list.clone();
            let globalload = self.db_interface.globalload.clone();
            let jobmanager = self.db_interface.jobmanager.clone();
            let handle = thread::spawn(move || {
                let socketname = format!("{}_{}", types::SOCKET_NAME, worker_id);

                let name = format!("{}_{}", types::SOCKET_NAME, worker_id)
                    .to_ns_name::<GenericNamespaced>()
                    .unwrap();
                // Configures the listener
                let opt = ListenerOptions::new().name(name);

                // Bind our listener.
                let listener = opt.create_sync().unwrap();

                {
                    thread_list.lock().push(worker_id);
                }
                for stream in listener.incoming().flatten() {
                    let worker_id = worker_id as usize;
                    handle_client(
                        stream,
                        &worker_id,
                        db.clone(),
                        globalload.clone(),
                        jobmanager.clone(),
                    );
                    {
                        thread_list.lock().push(worker_id.try_into().unwrap());
                    }
                }
                /*  let worker_id = i.clone();
                loop {
                    match listener.accept() {
                        Ok(stream) => {
                            handle_client(stream, db_interface.clone(), &worker_id);
                        }
                        Err(err) => {
                            dbg!(err);
                        }
                    }
                }*/
                //format!("Worker thread {} started", i);
                //  for stream in listener.incoming().flatten() {
                //      handle_client(stream, db_interface.clone(), &worker_id);
                //  }
            });
            handles.push(handle);
        }
        let socketname = types::SOCKET_NAME;
        let name = types::SOCKET_NAME
            .to_ns_name::<GenericNamespaced>()
            .unwrap();

        // Configures the listener
        let opt = ListenerOptions::new().name(name);

        // Bind our listener.
        let listener = match opt.create_sync() {
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                eprintln!(
                    "\
Error: could not start server because the socket file is occupied. Please check if {} is in use by \
another process and try again.",
                    types::SOCKET_NAME
                );
                return Err(e.into());
            }
            x => x?,
        };

        // Stand-in for the syncronization used, if any, between the client and the server.
        logging::info_log(format!(
            "IPC Server running at {} with {} threads",
            types::SOCKET_NAME,
            num_threads
        ));

        //listener
        //    .set_nonblocking(interprocess::local_socket::traits::ListenerNonblockingMode::Stream)
        //    .unwrap();

        let listener = Arc::new(listener);
let routes = main_db.clone().get_filters();

    let routes_with_fallback = routes.recover(handle_rejection);

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");



handles.push(std::thread::spawn(move || {
            
    // 2. Use block_on to run the async function and wait for its result
    println!("Server running on 127.0.0.1:3030");
    let result = runtime.block_on(warp::serve(routes_with_fallback).run(([127, 0, 0, 1], 3030)));
        }));


        // NOTE due to the nature of this POS if the number of requests coming in exceed the number
        // of cpu threads we could softlock and I can't trace it.
        // But for now this seems to work.
        //for i in 0..2 {
        {
            let thread_list = thread_list.clone();
            handles.push(std::thread::spawn(move || {
                let cnt: u64 = 0;
                for stream in listener.incoming().flatten() {
                    let worker_id: u64 = {
                        loop {
                            let mut thread_list = thread_list.lock();
                            if let Some(out) = thread_list.pop() {
                                break out;
                            } else {
                                logging::error_log(format!("Waiting for more threads to open up"));
                                std::thread::sleep(Duration::from_millis(10));
                            }
                        }
                    };
                    //if cnt == num_threads - 1 {
                    //    cnt = 0;
                    //}
                    //let worker_id = cnt;
                    //cnt += 1;
                    let mut conn = BufReader::new(stream);
                    types::send_preserialize(&data_size_to_b(&worker_id), &mut conn);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        Ok(())
    }
}
fn handle_client(
    stream: LocalSocketStream,
    worker_id: &usize,
    db: Main,
    globalload: GlobalLoad,
    jobmanager: Arc<RwLock<Jobs>>,
) {
    let worker_id = worker_id.clone();
    std::thread::spawn(move || {
        let buffer = [0u8; 1024];
        let mut conn = BufReader::new(stream);
        let plugin_supportedrequests = match types::recieve(&mut conn) {
            Ok(out) => out,
            Err(err) => {
                dbg!(&err);
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
    jobmanager: Arc<RwLock<Jobs>>,
}

impl DbInteract {}

/// Storage object for database interactions with the plugin system
/// Helper function to return data about a passed object into size and bytes array.
fn data_size_to_b<T: serde::Serialize>(data_object: &T) -> Vec<u8> {
    let tmp = data_object;

    // let bytd = types::x_to_bytes(tmp).to_vec();
    let byt: Vec<u8> = bincode::serde::encode_to_vec(tmp, bincode::config::standard()).unwrap();
    byt
}

/// Packages functions from the DB into their self owned versions before packaging
/// them as bytes to get sent accross IPC to the other software. So far things are
/// pretty mint.
pub fn dbactions_to_function(
    dbaction: types::SupportedDBRequests,
    database: Main,
    globalload: GlobalLoad,
    jobmanager: Arc<RwLock<Jobs>>,
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
        )) => data_size_to_b(&database.relationship_get_tagid_where_namespace_count(
            &namespace_id,
            &count,
            &dir,
        )),
        types::SupportedDBRequests::GetRelationshipFileidWhereNamespace((
            namespace_id,
            count,
            dir,
        )) => data_size_to_b(&database.relationship_get_fileid_where_namespace_count(
            &namespace_id,
            &count,
            &dir,
        )),

        types::SupportedDBRequests::CondenseTags() => {
            let unwrappy = database;
            unwrappy.transaction_flush();
            let mut write_conn = unwrappy.get_database_connection();
            let mut tn = write_conn.transaction().unwrap();
            unwrappy.condense_tags(&mut tn);
            tn.commit().unwrap();
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::TagDelete(tag_id) => {
            let unwrappy = database;
            unwrappy.tag_remove(&tag_id);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::MigrateRelationship((file_id, old_tag_id, new_tag_id)) => {
            let unwrappy = database;
            unwrappy.migrate_relationship_file_tag(&file_id, &old_tag_id, &new_tag_id);
            unwrappy.transaction_flush();
            data_size_to_b(&true)
        }

        types::SupportedDBRequests::MigrateTag((old_tag_id, new_tag_id)) => {
            let unwrappy = database;
            let mut write_conn = unwrappy.get_database_connection();
            let mut tn = write_conn.transaction().unwrap();
            unwrappy.migrate_tag(&old_tag_id, &new_tag_id, &mut tn);
            tn.commit().unwrap();
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::PutFileNoBlock((mut file, ratelimit)) => {
            let mut global_pluginscraper = sharedtypes::return_default_globalpluginparser();
            global_pluginscraper.name = "InternalFileAdd".to_string();
            let ratelimiter_obj = threading::create_ratelimiter(ratelimit, &0, &0);
            let manageeplugin = globalload.clone();
            let client = Arc::new(RwLock::new(download::client_create(vec![], false)));
            let jobstorage = jobmanager.clone();
            let database = database.clone();
            let thread = thread::spawn(move || {
                threading::main_file_loop(
                    &mut file,
                    database,
                    ratelimiter_obj,
                    manageeplugin,
                    client,
                    jobstorage,
                    &global_pluginscraper,
                    &0,
                    &0,
                );
            });

            data_size_to_b(&true)
        }

        types::SupportedDBRequests::PutFile((mut file, ratelimit)) => {
            let mut global_pluginscraper = sharedtypes::return_default_globalpluginparser();
            global_pluginscraper.name = "InternalFileAdd".to_string();

            let ratelimiter_obj = threading::create_ratelimiter(ratelimit, &0, &0);
            let manageeplugin = globalload.clone();
            let client = Arc::new(RwLock::new(download::client_create(vec![], false)));
            let jobstorage = jobmanager.clone();
            threading::main_file_loop(
                &mut file,
                database.clone(),
                ratelimiter_obj,
                manageeplugin,
                client,
                jobstorage,
                &global_pluginscraper,
                &0,
                &0,
            );

            data_size_to_b(&true)
        }
        types::SupportedDBRequests::PutJob(job) => {
            let unwrappy = database;
            let _ = unwrappy.jobs_add_new(job);
            unwrappy.transaction_flush();
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
                types::ParentsType::Tag => &unwrappy.parents_tagid_tag_get(&id),
                types::ParentsType::Rel => &unwrappy.parents_relate_tag_get(&id),
                types::ParentsType::LimitTo => &unwrappy.parents_limitto_tag_get(&id),
            };

            data_size_to_b(&out)
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
            let mut out: HashSet<usize> = HashSet::new();
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
            unwrappy.relationship_add(file, tag);
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::RelationshipRemove(file, tag) => {
            let unwrappy = database;
            unwrappy.relationship_remove(&file, &tag);
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
            unwrappy.relationship_add(fid, tmep);
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
            unwrappy.transaction_flush();
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

            unwrappy.transaction_flush();
            data_size_to_b(&out)
        }
        types::SupportedDBRequests::GetNamespace(name) => {
            let unwrappy = database;
            let tmep = unwrappy.namespace_get(&name);
            option_to_bytes(tmep.as_ref())
        }
        types::SupportedDBRequests::TestUsize() => {
            let test: usize = 32;
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
            unwrappy.transaction_flush();
            data_size_to_b(&true)
        }
        types::SupportedDBRequests::TransactionFlush() => {
            let unwrappy = database;
            unwrappy.transaction_flush();

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
            //bincode::serialize(&tmep).unwrap()
        }
        types::SupportedDBRequests::ReloadRegex => {
            globalload.reload_regex();

            data_size_to_b(&true)
        }
    }
}

/// Turns an Option<&T> into a bytes object.
fn option_to_bytes<T: serde::Serialize + Clone>(input: Option<&T>) -> Vec<u8> {
    match input {
        None => data_size_to_b(&input),
        Some(item) => {
            let i: Option<T> = Some(item.clone());
            data_size_to_b(&i)
        }
    }
}
