#![allow(dead_code)]
#![allow(unused_variables)]

use crate::database;
use crate::download;
use crate::globalload::GlobalLoad;
use crate::jobs::Jobs;
use crate::logging;
use crate::logging::error_log;
use crate::sharedtypes;
use crate::threading;
use anyhow::Context;

use crate::RwLock;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

use std::{
    io::{self, BufReader, prelude::*},
    sync::mpsc::Sender,
};

mod types;

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
    pub fn new(
        main_db: Arc<RwLock<database::Main>>,
        globalload: Arc<RwLock<GlobalLoad>>,
        jobs: Arc<RwLock<Jobs>>,
    ) -> Self {
        PluginIpcInteract {
            db_interface: DbInteract {
                _database: main_db.clone(),
                globalload,
                jobmanager: jobs.clone(),
            },
        }
    }

    /// Spawns a listener for events.
    pub fn spawn_listener(&mut self) -> anyhow::Result<()> {
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
        }

        // Pick a name. There isn't a helper function for this, mostly because it's
        // largely unnecessary: in Rust, `match` is your concise, readable and expressive
        // decision making construct. Pick a name. There isn't a helper function for this,
        // mostly because it's largely unnecessary: in Rust, `match` is your concise,
        // readable and expressive decision making construct.
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

        let num_threads = match thread::available_parallelism() {
            Ok(thread_num) => thread_num,
            Err(err) => {
                error_log(
                    "IPC Server could not spawn because it couldn't find the number of CPU threads to use",
                );
                return Err(err.into());
            }
        };

        // Stand-in for the syncronization used, if any, between the client and the server.
        logging::info_log(format!(
            "IPC Server running at {} with {} threads",
            types::SOCKET_NAME,
            num_threads
        ));

        let listener = Arc::new(listener);
        let db_interface = self.db_interface.clone();

        let mut handles = Vec::new();

        // NOTE due to the nature of this POS if the number of requests coming in exceed the number
        // of cpu threads we could softlock and I can't trace it.
        // But for now this seems to work.
        for i in 0..num_threads.into() {
            let listener = Arc::clone(&listener);
            let db_interface = db_interface.clone();
            let handle = thread::spawn(move || {
                //format!("Worker thread {} started", i);
                for stream in listener.incoming().flatten() {
                    // info_ format!("Worker {} got a connection", i);
                    handle_client(stream, db_interface.clone());
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
        Ok(())
    }
}
fn handle_client(mut stream: LocalSocketStream, db_interface: DbInteract) {
    let mut buffer = [0u8; 1024];
    let mut conn = BufReader::new(stream);
    let plugin_supportedrequests = match types::recieve(&mut conn) {
        Ok(out) => out,
        Err(err) => {
            dbg!(&err);
            logging::error_log(err.to_string());
            return;
        }
    };

    // Default
    match plugin_supportedrequests {
        types::SupportedRequests::Database(db_actions) => {
            let data = db_interface.dbactions_to_function(db_actions);
            types::send_preserialize(&data, &mut conn);
        }
        types::SupportedRequests::PluginCross(_plugindata) => {}
    }
}

#[derive(Clone)]
struct DbInteract {
    _database: Arc<RwLock<database::Main>>,
    globalload: Arc<RwLock<GlobalLoad>>,
    jobmanager: Arc<RwLock<Jobs>>,
}

/// Storage object for database interactions with the plugin system
impl DbInteract {
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
    pub fn dbactions_to_function(&self, dbaction: types::SupportedDBRequests) -> Vec<u8> {
        match dbaction {
            types::SupportedDBRequests::TagDelete(tag_id) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.tag_remove(&tag_id);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::MigrateRelationship((file_id, old_tag_id, new_tag_id)) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.migrate_relationship_file_tag(&file_id, &old_tag_id, &new_tag_id);
                Self::data_size_to_b(&true)
            }

            types::SupportedDBRequests::MigrateTag((old_tag_id, new_tag_id)) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.migrate_tag(&old_tag_id, &new_tag_id);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::PutFileNoBlock((mut file, ratelimit)) => {
                let mut global_pluginscraper = sharedtypes::return_default_globalpluginparser();
                global_pluginscraper.name = "InternalFileAdd".to_string();
                let ratelimiter_obj = threading::create_ratelimiter(ratelimit, &0, &0);
                let manageeplugin = self.globalload.clone();
                let client = Arc::new(RwLock::new(download::client_create(vec![], false)));
                let jobstorage = self.jobmanager.clone();
                let database = self._database.clone();
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

                Self::data_size_to_b(&true)
            }

            types::SupportedDBRequests::PutFile((mut file, ratelimit)) => {
                let mut global_pluginscraper = sharedtypes::return_default_globalpluginparser();
                global_pluginscraper.name = "InternalFileAdd".to_string();

                let ratelimiter_obj = threading::create_ratelimiter(ratelimit, &0, &0);
                let manageeplugin = self.globalload.clone();
                let client = Arc::new(RwLock::new(download::client_create(vec![], false)));
                let jobstorage = self.jobmanager.clone();
                threading::main_file_loop(
                    &mut file,
                    self._database.clone(),
                    ratelimiter_obj,
                    manageeplugin,
                    client,
                    jobstorage,
                    &global_pluginscraper,
                    &0,
                    &0,
                );

                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::PutJob(job) => {
                let mut unwrappy = self._database.write().unwrap();
                let _ = &unwrappy.jobs_add_new(job);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::GetNamespaceIDsAll => {
                let unwrappy = self._database.read().unwrap();
                Self::data_size_to_b(&unwrappy.namespace_keys())
            }

            types::SupportedDBRequests::GetFileExt(ext_id) => {
                let unwrappy = self._database.read().unwrap();
                Self::option_to_bytes(unwrappy.extension_get_string(&ext_id).as_ref())
            }
            types::SupportedDBRequests::GetJob(id) => {
                let unwrappy = self._database.read().unwrap();
                Self::option_to_bytes(unwrappy.jobs_get(&id).as_ref())
            }
            types::SupportedDBRequests::ParentsPut(parent) => {
                let mut unwrappy = self._database.write().unwrap();
                Self::data_size_to_b(&unwrappy.parents_add(parent))
            }
            types::SupportedDBRequests::ParentsGet((parentswitch, id)) => {
                let unwrappy = self._database.read().unwrap();
                match parentswitch {
                    types::ParentsType::Tag => Self::data_size_to_b(&unwrappy.parents_rel_get(&id)),
                    types::ParentsType::Rel => Self::data_size_to_b(&unwrappy.parents_tag_get(&id)),
                }
            }
            types::SupportedDBRequests::ParentsDelete(parentobj) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.parents_selective_remove(&parentobj);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::GetFileLocation(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.get_file(&id);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::FilterNamespaceById((ids, namespace_id)) => {
                let mut out: HashSet<usize> = HashSet::new();
                let unwrappy = self._database.read().unwrap();
                for each in ids.iter() {
                    if unwrappy.namespace_contains_id(&namespace_id, each) {
                        out.insert(*each);
                    }
                }
                Self::data_size_to_b(&out)
            }
            types::SupportedDBRequests::ReloadLoadedPlugins() => {
                //let mut plugin = self.globalload.write().unwrap();
                //plugin.reload_loaded_plugins();
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::NamespaceContainsId(namespaceid, tagid) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.namespace_contains_id(&namespaceid, &tagid);
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::PluginCallback(func_name, version, input_data) => {
                let mut plugin = self.globalload.read().unwrap();
                let out = plugin.external_plugin_call(&func_name, &version, &input_data);
                Self::data_size_to_b(&out)
            }
            types::SupportedDBRequests::GetFileByte(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.get_file_bytes(&id);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::Search((search, limit, offset)) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.search_db_files(search, limit);
                //let tmep = unwrappy.search_db_files(search, limit, offset);
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::GetTagId(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.tag_id_get(&id);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::Logging(log) => {
                logging::info_log(&log);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::LoggingNoPrint(log) => {
                logging::log(&log);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::RelationshipAdd(file, tag) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.relationship_add(file, tag, true);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::RelationshipRemove(file, tag) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.relationship_remove(&file, &tag);
                Self::data_size_to_b(&true)
            }

            types::SupportedDBRequests::PutTag(tags, namespace_id, addtodb, id) => {
                let mut unwrappy = self._database.write().unwrap();
                let tmep = unwrappy.tag_add(&tags, namespace_id, addtodb, id);
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::PutTagRelationship(
                fid,
                tags,
                namespace_id,
                addtodb,
                id,
            ) => {
                let mut unwrappy = self._database.write().unwrap();
                let tmep = unwrappy.tag_add(&tags, namespace_id, addtodb, id);
                unwrappy.relationship_add(fid, tmep, addtodb);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::GetDBLocation() => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.location_get();
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::SettingsSet(name, pretty, num, param, addtodb) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.setting_add(name, pretty, num, param, addtodb);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::RelationshipGetTagid(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.relationship_get_tagid(&id);
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::GetFile(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.file_get_id(&id);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::RelationshipGetFileid(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.relationship_get_fileid(&id);
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::SettingsGetName(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.settings_get_name(&id);
                Self::option_to_bytes(tmep)
            }
            types::SupportedDBRequests::GetTagName((name, namespace)) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.tag_get_name(name, namespace);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::GetFileHash(name) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.file_get_hash(&name);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::CreateNamespace(name, description) => {
                let mut unwrappy = self._database.write().unwrap();
                let out = unwrappy.namespace_add(&name, &description);
                Self::data_size_to_b(&out)
            }
            types::SupportedDBRequests::GetNamespace(name) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.namespace_get(&name);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::TestUsize() => {
                let test: usize = 32;
                Self::data_size_to_b(&test)
            }
            types::SupportedDBRequests::GetNamespaceString(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.namespace_get_string(&id);
                Self::option_to_bytes(tmep.as_ref())
            }
            types::SupportedDBRequests::LoadTable(table) => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.load_table(&table);
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::TransactionFlush() => {
                let mut unwrappy = self._database.write().unwrap();
                unwrappy.transaction_flush();
                Self::data_size_to_b(&true)
            }
            types::SupportedDBRequests::GetNamespaceTagIDs(id) => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.namespace_get_tagids(&id);
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::GetFileListId() => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.file_get_list_id();
                Self::data_size_to_b(&tmep)
            }
            types::SupportedDBRequests::GetFileListAll() => {
                let unwrappy = self._database.read().unwrap();
                let tmep = unwrappy.file_get_list_all();
                Self::data_size_to_b(&tmep)
                //bincode::serialize(&tmep).unwrap()
            }
            types::SupportedDBRequests::ReloadRegex => {
                let globalload;
                {
                    let unwrappy = self._database.write().unwrap();
                    if let Some(globalload_arc) = unwrappy.globalload.clone() {
                        globalload = globalload_arc.clone();
                    } else {
                        return Self::data_size_to_b(&true);
                    }
                }

                globalload.write().unwrap().reload_regex();

                Self::data_size_to_b(&true)
            }
        }
    }

    /// Turns an Option<&T> into a bytes object.
    fn option_to_bytes<T: serde::Serialize + Clone>(input: Option<&T>) -> Vec<u8> {
        match input {
            None => Self::data_size_to_b(&input),
            Some(item) => {
                let i: Option<T> = Some(item.clone());
                Self::data_size_to_b(&i)
            }
        }
    }
}
