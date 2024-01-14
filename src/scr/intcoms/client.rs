use anyhow::Context;
use bincode;
use interprocess::local_socket::{LocalSocketStream, NameTypeSupport};
use std::io::{prelude::*, BufReader};

use crate::sharedtypes;

use self::types::AllReturns;

pub mod types;

pub fn main() -> anyhow::Result<()> {
    call_conn()
}

fn call_conn() -> anyhow::Result<()> {
    //let _buffers = &mut [b'0', b'0'];

    // Preemptively allocate a sizeable buffer for reading.
    // This size should be enough and should be easy to find for the allocator.
    //let _buffer = String::with_capacity(size);

    let typerequets = types::SupportedRequests::Database(types::SupportedDBRequests::GetTagId(13));

    init_data_request(&typerequets);

    let typerequets =
        types::SupportedRequests::Database(types::SupportedDBRequests::RelationshipGetTagid(0));

    init_data_request(&typerequets);

    let typerequets = types::SupportedRequests::Database(types::SupportedDBRequests::GetFile(1));

    init_data_request(&typerequets);

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

pub fn init_data_request(requesttype: &types::SupportedRequests) {
    let coms_struct = types::Coms {
        com_type: types::EComType::BiDirectional,
        control: types::EControlSigs::Send,
    };

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

    // Create our connection. This will block until the server accepts our connection, but will fail
    // immediately if the server hasn't even started yet; somewhat similar to how happens with TCP,
    // where connecting to a port that's not bound to any server will send a "connection refused"
    // response, but that will take twice the ping, the roundtrip time, to reach the client.
    let conn = LocalSocketStream::connect(name)
        .context("Failed to connect to server")
        .unwrap();
    // Wrap it into a buffered reader right away so that we could read a single line out of it.
    let mut conn = BufReader::new(conn);

    let b_struct = types::x_to_bytes(&coms_struct);
    // Sends the plugin com type.
    conn.get_mut()
        .write_all(b_struct)
        .context("Socket send failed")
        .unwrap();

    let buffer: &mut [u8; 1] = &mut [b'0'];
    conn.read(buffer)
        .context("plugin failed 2nd step init")
        .unwrap();

    let econtrolsig = types::con_econtrolsigs(buffer);

    match econtrolsig {
        types::EControlSigs::Halt => return,
        types::EControlSigs::Send => {}
        types::EControlSigs::Break => {
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
    let vare = handle_supportedrequesttypes(data_buffer, requesttype);
    println!("{:?}", vare);
}

///
/// Converts vec into a supported data type.
///
fn handle_supportedrequesttypes(
    data_buffer: &mut [u8],
    requesttype: &types::SupportedRequests,
) -> AllReturns {
    match requesttype {
        types::SupportedRequests::Database(db_actions) => match db_actions {
            types::SupportedDBRequests::GetTagId(_id) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetTagId(out))
            }
            types::SupportedDBRequests::RelationshipGetTagid(_id) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::RelationshipGetTagid(out))
            }
            types::SupportedDBRequests::RelationshipGetFileid(_id) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::RelationshipGetFileid(out))
            }
            types::SupportedDBRequests::GetFile(_id) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetFile(out))
            }
            types::SupportedDBRequests::SettingsGetName(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::SettingsGetName(out))
            }
        },
        types::SupportedRequests::PluginCross(_plugindata) => AllReturns::Nune,
    }
}
