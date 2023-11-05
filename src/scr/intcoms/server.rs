use anyhow::Context;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use std::{
    io::{self, prelude::*, BufReader},
    sync::mpsc::Sender,
};
use std::sync::{Mutex, Arc, mpsc};
use crate::{database, sharedtypes::DbTagObj};

mod types;

pub fn main(notify: Sender<()>) -> anyhow::Result<()> {
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
        let coms_struct = types::coms {
            com_type: types::eComType::BiDirectional,
            control: types::eControlSigs::SEND,
        };
        let b_struct = types::coms_to_bytes(&coms_struct);
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
        conn.get_mut().write_all(b"Hello from server!\n")?;

        // Print out the result, getting the newline for free!

        let instruct: types::coms = types::con_coms(buffer);
        //std::mem::forget(buffer.clone());
        dbg!(&buffer);

        print!(
            "Client answered: {:?} {:?}\n",
            instruct.com_type, instruct.control
        );

        match instruct.control {
            types::eControlSigs::SEND => {
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
            types::eControlSigs::HALT => {}
            types::eControlSigs::BREAK => {}
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

struct plugin_ipc_interact{
    db_interface: db_interact, 
}

///
/// This is going to be the main way to talk to the plugin system and stuffins.
///
impl plugin_ipc_interact{
    pub fn new(main_db: Arc<Mutex<database::Main>>) -> Self{
        plugin_ipc_interact{db_interface: db_interact{_database: main_db}}
    }
}




struct db_interact{
    _database: Arc<Mutex<database::Main>>,
}

///
/// Storage object for database interactions with the plugin system
///
impl db_interact {
    
    ///
    /// Stores database inside of self for DB interactions with plugin system
    ///
    pub fn new(main_db: Arc<Mutex<database::Main>>) -> Self {
        let reftoself = db_interact {_database: main_db};
        
        reftoself
    }
    
    pub fn db_tag_id_get(&mut self, uid: &usize) -> Option<DbTagObj> {
            self._database.lock().unwrap().tag_id_get(uid)
    }
    pub fn db_relationship_get_tagid(&mut self, tag: &usize) -> Vec<usize> {
        self._database.lock().unwrap().relationship_get_tagid(tag)
    }
    pub fn db_get_file(&mut self, fileid: &usize) -> Option<(String, String, String)> {
        self._database.lock().unwrap().file_get_id(fileid)
    }
}

enum db_transaction {
    db_tag_id_get,
    db_relationship_get_tagid,
    db_get_file
}