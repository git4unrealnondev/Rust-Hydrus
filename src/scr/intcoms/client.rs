use anyhow::Context;
use interprocess::local_socket::{LocalSocketStream, NameTypeSupport};
use std::collections::HashSet;
use std::io::{prelude::*, BufReader};
use bincode;

use crate::sharedtypes;
use crate::database;

mod types;

pub fn main() -> anyhow::Result<()> {
    // Pick a name. There isn't a helper function for this, mostly because it's largely unnecessary:
    // in Rust, `match` is your concise, readable and expressive decision making construct.

    call_conn(1000, "beans".to_string())
}

fn call_conn(size: usize, _message: String) -> anyhow::Result<()> {
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


    //let _buffers = &mut [b'0', b'0'];

    // Preemptively allocate a sizeable buffer for reading.
    // This size should be enough and should be easy to find for the allocator.
    //let _buffer = String::with_capacity(size);





    let typerequets =
        types::SupportedRequests::Database(types::SupportedDBRequests::db_tag_id_get(13));

    init_data_request(&name, &typerequets);
    
        let typerequets =
        types::SupportedRequests::Database(types::SupportedDBRequests::db_relationship_get_tagid(0));

    init_data_request(&name, &typerequets);
    
        let typerequets =
        types::SupportedRequests::Database(types::SupportedDBRequests::db_get_file(1));

    init_data_request(&name, &typerequets);

    /*// We now employ the buffer we allocated prior and read until EOF, which the server will
    // similarly invoke with `.shutdown()`, verifying validity of UTF-8 on the fly.
    conn.read_line(&mut buffer)
        .context("Socket receive failed")?;
    dbg!(&buffer);
    conn.get_mut()
        .write_all(b"beans\n")
        .context("Socket send failed")?;

    conn.read(buffers).context("Socket receive failed")?;
    dbg!(&buffer);
    buffer.clear();
    conn.get_mut()
        .write_all(b"beans1\n")
        .context("Socket send failed")?;
    // Print out the result, getting the newline for free!
    print!("Server answered: {}", buffer);*/
    Ok(())
}

pub fn init_data_request(
    name: &str,
    requesttype: &types::SupportedRequests,
) {
        let coms_struct = types::Coms {
        com_type: types::eComType::BiDirectional,
        control: types::eControlSigs::SEND,
    };
    
    // Create our connection. This will block until the server accepts our connection, but will fail
    // immediately if the server hasn't even started yet; somewhat similar to how happens with TCP,
    // where connecting to a port that's not bound to any server will send a "connection refused"
    // response, but that will take twice the ping, the roundtrip time, to reach the client.
    let conn = LocalSocketStream::connect(name).context("Failed to connect to server").unwrap();
    // Wrap it into a buffered reader right away so that we could read a single line out of it.
    let mut conn = BufReader::new(conn);
    
    
    let b_struct = types::x_to_bytes(&coms_struct);
        // Sends the plugin com type.
    conn.get_mut()
        .write_all(b_struct)
        .context("Socket send failed").unwrap();
    
    let buffer: &mut [u8; 1] = &mut [b'0'];
    conn.read(buffer)
        .context("plugin failed 2nd step init")
        .unwrap();

    let econtrolsig = types::con_econtrolsigs(buffer);

    match econtrolsig {
        types::eControlSigs::HALT => return,
        types::eControlSigs::SEND => {}
        types::eControlSigs::BREAK => {
            panic!("This plugin was called to break. Will break NOW.");
        }
    }

    // Requesting data from server.
    let b_requesttype = types::x_to_bytes(requesttype);
    conn.get_mut().write_all(b_requesttype).unwrap();
    
    //Recieving size Data from server
    let size_buffer: &mut [u8; 8] = &mut [b'0'; 8];
    conn.read(size_buffer)
        .context("plugin failed 3nd step init")
        .unwrap();

    // Receiving actual data from server
    let size: usize = types::con_usize(size_buffer);
    let data_buffer = &mut vec![b'0'; size];
    conn.read(data_buffer)
        .context("plugin failed 3nd step init")
        .unwrap();

    // Handle data from server.
   handle_supportedrequesttypes(data_buffer, requesttype);
    
}

///
/// Converts vec into a supported data type.
///
fn handle_supportedrequesttypes(data_buffer: &mut Vec<u8>,requesttype: &types::SupportedRequests) {
    
    match requesttype {
        types::SupportedRequests::Database(db_actions) => match db_actions {
            types::SupportedDBRequests::db_tag_id_get(id) => {
                let mut opjtag: Option<sharedtypes::DbTagObj> = bincode::deserialize(&data_buffer[..]).unwrap();
                dbg!(opjtag);
            },
            types::SupportedDBRequests::db_relationship_get_tagid(id) => {
                let mut opjtag: Vec<usize> = bincode::deserialize(&data_buffer[..]).unwrap();
                dbg!(opjtag);
                
            },
            types::SupportedDBRequests::db_relationship_get_fileid(id) => {
                let mut opjtag: HashSet<usize> = bincode::deserialize(&data_buffer[..]).unwrap();
                dbg!(opjtag);
                
            },
            types::SupportedDBRequests::db_get_file(id) => {
                let mut opjtag: Option<(String, String, String)> = bincode::deserialize(&data_buffer[..]).unwrap();
                dbg!(opjtag);
            },
        },
        types::SupportedRequests::PluginCross(plugindata) => {}
    }
}

