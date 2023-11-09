use crate::{database, sharedtypes::DbTagObj};
use anyhow::Context;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::{
    io::{self, prelude::*, BufReader},
    mem,
    sync::mpsc::Sender,
};

use crate::logging;
use crate::sharedtypes;

mod types;

pub fn main(notify: Sender<()>) -> anyhow::Result<()> {
    // Define a function that checks for errors in incoming connections. We'll use this to filter
    // through connections that fail on initialization for one reason or another.
    fn handle_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
        match conn {
            Ok(c) => Some(c),
            Err(e) => {
                println!("Incoming connection failed: {}", e);
                None
            }
        }
    }

    // Pick a name. There isn't a helper function for this, mostly because it's largely unnecessary:
    // in Rust, `match` is your concise, readable and expressive decision making construct.
    let name = {
        // This scoping trick allows us to nicely contain the import inside the `match`, so that if
        // any imports of variants named `Both` happen down the line, they won't collide with the
        // enum we're working with here. Maybe someone should make a macro for this.
        use NameTypeSupport::*;
        match NameTypeSupport::query() {
            OnlyPaths => "/tmp/RustHydrus.sock",
            OnlyNamespaced | Both => "@RustHydrus.sock",
        }
    };

    // Bind our listener.
    let listener = match LocalSocketListener::bind(name) {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            // One important problem that is easy to handle improperly (or not at all) is the
            // "corpse sockets" that are left when a program that uses a file-type socket name
            // terminates its socket server without deleting the file. There's no single strategy
            // for handling this kind of address-already-occupied error. Services that are supposed
            // to only exist as a single instance running on a system should check if another
            // instance is actually running, and if not, delete the socket file. In this example,
            // we leave this up to the user, but in a real application, you usually don't want to do
            // that.
            eprintln!(
                "\
Error: could not start server because the socket file is occupied. Please check if {} is in use by \
another process and try again.",
                name,
            );
            return Err(e.into());
        }
        x => x?,
    };

    println!("Server running at {}", name);
    // Stand-in for the syncronization used, if any, between the client and the server.
    let _ = notify.send(());

    // Preemptively allocate a sizeable buffer for reading at a later moment. This size should be
    // enough and should be easy to find for the allocator. Since we only have one concurrent
    // client, there's no need to reallocate the buffer repeatedly.

    for conn in listener.incoming().filter_map(handle_error) {
        let buffer = &mut [b'0', b'0'];
        let mut bufstr = String::new();
        let coms_struct = types::Coms {
            com_type: types::eComType::BiDirectional,
            control: types::eControlSigs::Send,
        };
        let b_struct = types::x_to_bytes(&coms_struct);
        // Wrap the connection into a buffered reader right away
        // so that we could read a single line out of it.
        let mut conn = BufReader::new(conn);
        println!("Incoming connection!");

        // Since our client example writes first, the server should read a line and only then send a
        // response. Otherwise, because reading and writing on a connection cannot be simultaneous
        // without threads or async, we can deadlock the two processes by having both sides wait for
        // the write buffer to be emptied by the other.
        conn.read(buffer).context("Socket receive failed")?;

        // Now that the read has come through and the client is waiting on the server's write, do
        // it. (`.get_mut()` is to get the writer, `BufReader` doesn't implement a pass-through
        // `Write`.)
        //conn.get_mut().write_all(b"Hello from server!\n")?;

        // Print out the result, getting the newline for free!

        let instruct: types::Coms = types::con_coms(buffer);
        //std::mem::forget(buffer.clone());
        dbg!(&buffer);

        println!(
            "Client answered: {:?} {:?}",
            instruct.com_type, instruct.control
        );

        match instruct.control {
            types::eControlSigs::Send => {
                bufstr.clear();
                conn.read_line(&mut bufstr)
                    .context("Socket receive failed")?;

                dbg!(&bufstr);
                //bufstr.clear();

                conn.get_mut()
                    .write_all(b_struct)
                    .context("Socket send failed")?;
                bufstr.clear();
                conn.read_line(&mut bufstr)
                    .context("Socket receive failed")?;
                dbg!(&bufstr);
            }
            types::eControlSigs::Halt => {}
            types::eControlSigs::Break => {}
        }

        // Let's add an exit condition to shut the server down gracefully.
        //if buffer == "stop\n" {
        //    break;
        //}

        // Clear the buffer so that the next iteration will display new data instead of messages
        // stacking on top of one another.
        //buffer.clear();
    }
    Ok(())
}

///
/// Storage for database interaction object for IPC
///
pub struct plugin_ipc_interact {
    db_interface: DbInteract,
}

///
/// This is going to be the main way to talk to the plugin system and stuffins.
///
impl plugin_ipc_interact {
    pub fn new(main_db: Arc<Mutex<database::Main>>) -> Self {
        plugin_ipc_interact {
            db_interface: DbInteract { _database: main_db },
        }
    }

    ///
    /// Spawns a listener for events.
    ///
    pub fn spawn_listener(&mut self, notify: Sender<()>) -> anyhow::Result<()> {
        // Define a function that checks for errors in incoming connections. We'll use this to filter
        // through connections that fail on initialization for one reason or another.
        fn handle_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
            match conn {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("Incoming connection failed: {}", e);
                    None
                }
            }
        }

        // Pick a name. There isn't a helper function for this, mostly because it's largely unnecessary:
        // in Rust, `match` is your concise, readable and expressive decision making construct.
        let name = {
            // This scoping trick allows us to nicely contain the import inside the `match`, so that if
            // any imports of variants named `Both` happen down the line, they won't collide with the
            // enum we're working with here. Maybe someone should make a macro for this.
            use NameTypeSupport::*;
            match NameTypeSupport::query() {
                OnlyPaths => "/tmp/RustHydrus.sock",
                OnlyNamespaced | Both => "@RustHydrus.sock",
            }
        };
        // Bind our listener.
        let listener = match LocalSocketListener::bind(name) {
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                // One important problem that is easy to handle improperly (or not at all) is the
                // "corpse sockets" that are left when a program that uses a file-type socket name
                // terminates its socket server without deleting the file. There's no single strategy
                // for handling this kind of address-already-occupied error. Services that are supposed
                // to only exist as a single instance running on a system should check if another
                // instance is actually running, and if not, delete the socket file. In this example,
                // we leave this up to the user, but in a real application, you usually don't want to do
                // that.
                logging::error_log(&format!(
                    "Error: could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.",name,)
                );
                return Err(e.into());
            }
            x => x?,
        };

        println!("Server running at {}", name);
        // Stand-in for the syncronization used, if any, between the client and the server.
        let _ = notify.send(());

        // Main Plugin interaction loop
        for conn in listener.incoming().filter_map(handle_error) {
            let buffer = &mut [b'0', b'0'];

            let mut plugin_com_type: types::eComType = types::eComType::None; // Default value for no data

            let mut conn = BufReader::new(conn);
            //logging::info_log(&"Incoming connection from Plugin.".to_string());

            // Since our client example writes first, the server should read a line and only then send a
            // response. Otherwise, because reading and writing on a connection cannot be simultaneous
            // without threads or async, we can deadlock the two processes by having both sides wait for
            // the write buffer to be emptied by the other.
            conn.read(buffer).context("Socket receive failed")?;

            let instruct = types::con_coms(buffer);

            //Control flow for sending / receiving data from a plugin.
            match instruct.control {
                // If we get SEND then everything is good.
                types::eControlSigs::Send => {}
                // If we get HALT then stop connection. Natural stop.
                types::eControlSigs::Halt => {
                    break;
                }
                // HALT EVERYTHING WILL STOP ALL PLUGINS FROM COMUNICATING.
                types::eControlSigs::Break => {
                    self.halt_all_coms();
                }
            }

            plugin_com_type = instruct.com_type;

            match &plugin_com_type {
                types::eComType::SendOnly => {}    //TBD
                types::eComType::RecieveOnly => {} //TBD
                types::eComType::BiDirectional => {
                    //Default
                    let plugin_supportedrequests = self.send_data_request(&mut conn);

                    match plugin_supportedrequests {
                        types::SupportedRequests::Database(db_actions) => {
                            let (size, data) = self.db_interface.dbactions_to_function(db_actions);
                            self.send_processed_data(size, data, &mut conn);
                        }
                        types::SupportedRequests::PluginCross(_plugindata) => {}
                    }
                }
                types::eComType::None => {} // Do nothing.
            }
        }
        Ok(())
    }

    ///
    /// Sends data over the IPC channel.
    /// Sends size first then data.
    ///
    fn send_processed_data(
        &mut self,
        size: usize,
        data: Vec<u8>,
        conn: &mut BufReader<LocalSocketStream>,
    ) {
        //let arbdata = types::ArbitraryData{buffer_size: size, buffer_data:data};
        let b_size = types::x_to_bytes(&size);
        //let b_arbdata = types::x_to_bytes(&data);
        //dbg!(&b_arbdata);
        conn.get_mut()
            .write_all(b_size)
            .context("Socket send failed")
            .unwrap();

        //let arraytest: &mut [u8; 72] = &mut types::demo(data.to_vec());
        //let mut objtag = types::con_dbtagobj(arraytest);
        //dbg!(objtag);

        conn.get_mut()
            .write_all(&data)
            .context("Socket send failed")
            .unwrap();
    }

    ///
    /// Sends request for data to the calling plugin.
    ///
    fn send_data_request(
        &self,
        conn: &mut BufReader<LocalSocketStream>,
    ) -> types::SupportedRequests {
        let buffer: &mut [u8; 16] = &mut [b'0'; 16];
        let b_control = types::x_to_bytes(&types::eControlSigs::Send);
        conn.get_mut()
            .write_all(b_control)
            .context("Socket send failed")
            .unwrap();
        conn.read(buffer)
            .context("plugin failed 2nd step auth")
            .unwrap();
        types::con_supportedrequests(buffer)
    }

    ///
    /// Stops all coms from talking.
    ///
    fn halt_all_coms(&mut self) {}
}

struct DbInteract {
    _database: Arc<Mutex<database::Main>>,
}

///
/// Storage object for database interactions with the plugin system
///
impl DbInteract {
    ///
    /// Stores database inside of self for DB interactions with plugin system
    ///
    pub fn new(main_db: Arc<Mutex<database::Main>>) -> Self {

        DbInteract { _database: main_db }
    }
    ///
    /// Helper function to return data about a passed object into size and bytes array.
    ///
    fn data_size_to_b<T: serde::Serialize>(data_object: &T) -> (usize, Vec<u8>) {
        let tmp = data_object;
        //let bytd = types::x_to_bytes(tmp).to_vec();
        let byt: Vec<u8> = bincode::serialize(&tmp).unwrap();
        let sze = byt.len();
        (sze, byt)
    }

    pub fn dbactions_to_function(
        &mut self,
        dbaction: types::SupportedDBRequests,
    ) -> (usize, Vec<u8>) {
        match dbaction {
            types::SupportedDBRequests::db_tag_id_get(id) => {
                let data = self.db_tag_id_get(id);
                Self::data_size_to_b(&data)
            }
            types::SupportedDBRequests::db_relationship_get_tagid(id) => {
                let data = self.db_relationship_get_tagid(&id);
                Self::data_size_to_b(&data)
            }
            types::SupportedDBRequests::db_get_file(id) => {
                let data = self.db_get_file(&id);
                Self::data_size_to_b(&data)
            }
            types::SupportedDBRequests::db_relationship_get_fileid(id) => {
                let data = self.db_relationship_get_fileid(&id);
                Self::data_size_to_b(&data)
            }
        }
    }

    ///
    /// Wrapper for DB function. Takes in tag id returns tag object
    ///
    pub fn db_tag_id_get(&mut self, uid: usize) -> Option<DbTagObj> {
        self._database.lock().unwrap().tag_id_get(uid)
    }

    ///
    /// Wrapper for DB function. Returns a list of tag's associated with fileid
    ///
    pub fn db_relationship_get_tagid(&mut self, tag: &usize) -> Vec<usize> {
        self._database.lock().unwrap().relationship_get_tagid(tag)
    }

    ///
    /// Wrapper for DB function. Returns a files info based on id
    ///
    pub fn db_get_file(&mut self, fileid: &usize) -> Option<(String, String, String)> {
        self._database.lock().unwrap().file_get_id(fileid)
    }

    ///
    /// Wrapper for DB function. Returns a list of fileid's associated with tagid
    ///
    pub fn db_relationship_get_fileid(&mut self, tagid: &usize) -> HashSet<usize> {
        self._database
            .lock()
            .unwrap()
            .relationship_get_fileid(tagid)
    }
}
