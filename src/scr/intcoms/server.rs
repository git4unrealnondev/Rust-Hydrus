use crate::{database, sharedtypes::DbFileObj, sharedtypes::DbTagObjCompatability};
use anyhow::Context;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use itertools::Itertools;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::{
    io::{self, prelude::*, BufReader},
    mem,
    sync::mpsc::Sender,
};

use crate::{logging, sharedtypes};

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
            com_type: types::EComType::BiDirectional,
            control: types::EControlSigs::Send,
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
            types::EControlSigs::Send => {
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
            types::EControlSigs::Halt => {}
            types::EControlSigs::Break => {}
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
pub struct PluginIpcInteract {
    db_interface: DbInteract,
}

///
/// This is going to be the main way to talk to the plugin system and stuffins.
///
impl PluginIpcInteract {
    pub fn new(main_db: Arc<Mutex<database::Main>>) -> Self {
        PluginIpcInteract {
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

            let mut plugin_com_type: types::EComType = types::EComType::None; // Default value for no data

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
                types::EControlSigs::Send => {}
                // If we get HALT then stop connection. Natural stop.
                types::EControlSigs::Halt => {
                    break;
                }
                // HALT EVERYTHING WILL STOP ALL PLUGINS FROM COMUNICATING.
                types::EControlSigs::Break => {
                    self.halt_all_coms();
                }
            }

            plugin_com_type = instruct.com_type;

            match &plugin_com_type {
                types::EComType::SendOnly => {}    //TBD
                types::EComType::RecieveOnly => {} //TBD
                types::EComType::BiDirectional => {
                    //Default
                    let plugin_supportedrequests = self.send_data_request(&mut conn);

                    match plugin_supportedrequests {
                        types::SupportedRequests::Database(db_actions) => {
                            let tosend = self.db_interface.dbactions_to_function(db_actions);
                            if let Some((size, data)) = tosend {
                                self.send_processed_data(size, data, &mut conn);
                            }
                        }
                        types::SupportedRequests::PluginCross(_plugindata) => {}
                    }
                }
                types::EComType::None => {} // Do nothing.
            }
        }
        Ok(())
    }
    ///
    /// Converts a vec of T into an array.
    /// Stolen from: https://stackoverflow.com/questions/29570607/is-there-a-good-way-to-convert-a-vect-to-an-array
    ///
    fn demo<T, const N: usize>(v: Vec<T>) -> [T; N] {
        v.try_into().unwrap_or_else(|v: Vec<T>| {
            panic!("Expected a Vec of length {} but it was {}", N, v.len())
        })
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

        // Having to implement this because of the local socket getting overloaded.
        //let b_arbdata = types::x_to_bytes(&data);
        //dbg!(&b_arbdata);

        conn.get_mut()
            .write_all(b_size)
            .context("Socket send failed")
            .unwrap();
        //let arraytest: &mut [u8; 72] = &mut types::demo(data.to_vec());
        //let mut objtag = types::con_dbtagobj(arraytest);
        //dbg!(objtag)
        let size_buffer: &mut [u8; 8] = &mut [b'0'; 8];

        conn.read(size_buffer)
            .context("plugin failed 3nd step init")
            .unwrap();

        let binding = &mut data.clone();
        //println!("server data: {:?}", &binding);
        conn.get_mut()
            .write_all(binding)
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
        let buffer: &mut [u8; 40] = &mut [b'0'; 40];
        let b_control = types::x_to_bytes(&types::EControlSigs::Send);
        let beans = conn
            .get_mut()
            .write(b_control)
            .context("Socket send failed")
            .unwrap();
        conn.read_exact(buffer)
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
    ///
    /// Packages functions from the DB into their self owned versions
    /// before packaging them as bytes to get sent accross IPC to the other
    /// software. So far things are pretty mint.
    ///
    pub fn dbactions_to_function(
        &mut self,
        dbaction: types::SupportedDBRequests,
    ) -> Option<(usize, Vec<u8>)> {
        match dbaction {
            types::SupportedDBRequests::GetTagId(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.tag_id_get(&id);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::RelationshipGetTagid(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.relationship_get_tagid(&id);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::GetFile(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.file_get_id(&id);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::RelationshipGetFileid(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.relationship_get_fileid(&id);
                Some(Self::option_to_bytes(tmep))
            }

            types::SupportedDBRequests::SettingsGetName(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.settings_get_name(&id);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::GetTagName((name, namespace)) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.tag_get_name(name, namespace);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::GetFileHash(name) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.file_get_hash(&name);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::GetNamespace(name) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.namespace_get(&name);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::GetNamespaceString(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.namespace_get_string(&id);
                Some(Self::option_to_bytes(tmep))
            }
            types::SupportedDBRequests::LoadTable(table) => {
                let mut unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.load_table(&table);
                Some(Self::data_size_to_b(&true))
            }
            types::SupportedDBRequests::GetNamespaceTagIDs(id) => {
                let unwrappy = self._database.lock().unwrap();
                let tmep = unwrappy.namespage_get_tagids(&id);
                Some(Self::data_size_to_b(tmep))
            }
        }
    }
    ///
    /// Turns an Option<&T> into a bytes object.
    ///
    fn option_to_bytes<T: serde::Serialize + Clone>(input: Option<&T>) -> (usize, Vec<u8>) {
        match input {
            None => Self::data_size_to_b(&input),
            Some(item) => {
                let i: Option<T> = Some(item.clone());

                Self::data_size_to_b(&i)
            }
        }
    }
}
