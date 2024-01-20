use crate::sharedtypes::{self, AllFields};
use anyhow::Context;
use bincode;
use interprocess::local_socket::{LocalSocketStream, NameTypeSupport};
use std::collections::{HashMap, HashSet};
use std::io::{prelude::*, BufReader};

use self::types::{AllReturns, EfficientDataReturn};

pub mod types;

///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn load_table(table: sharedtypes::LoadDBTable) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::LoadTable(table),
    ))
}
///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn namespace_get_string(id: usize) -> Option<sharedtypes::DbNamespaceObj> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespaceString(id),
    ))
}

///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn namespace_get(name: String) -> Option<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespace(name),
    ))
}
///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn namespace_put(name: String, description: Option<String>, addtodb: bool) -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::CreateNamespace(name, description, addtodb),
    ))
}
///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn testusize() -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::TestUsize(),
    ))
}

///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn settings_get_name(name: String) -> Option<sharedtypes::DbSettingObj> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::SettingsGetName(name),
    ))
}

///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn tag_get_id(id: usize) -> Option<sharedtypes::DbTagNNS> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetTagId(id),
    ))
}
///
/// See the database reference for this function.
/// I'm a lazy turd just check it their
///
pub fn namespace_get_tagids(id: usize) -> HashSet<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespaceTagIDs(id),
    ))
}

///
/// Going to make this public.
/// This shouldn't come back to haunt me. :x
/// Returns a Vec of bytes that represent the data structure sent from server.
///
fn init_data_request<T: serde::de::DeserializeOwned>(requesttype: &types::SupportedRequests) -> T {
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

    // Requesting data from server.
    types::send(requesttype, &mut conn);

    //Recieving size Data from server
    //
    types::recieve(&mut conn)

    //println!("client data: {:?} size {}", data_buffer, size);
    // println!("slice: {:?}", &datavec[size..size * 2]);
    //println!("{}", std::mem::size_of_val(b_struct));
    // Handle data from server.
    //handle_supportedrequesttypes(data_buffer, requesttype)
    //println!("{:?}", vare);
}

/*///
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
            types::SupportedDBRequests::GetTagName(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetTagName(out))
            }
            types::SupportedDBRequests::GetFileHash(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetFileHash(out))
            }
            types::SupportedDBRequests::GetNamespace(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetNamespace(out))
            }
            types::SupportedDBRequests::GetNamespaceString(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetNamespaceString(out))
            }
            types::SupportedDBRequests::LoadTable(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::LoadTable(out))
            }
            types::SupportedDBRequests::GetNamespaceTagIDs(_key) => {
                let out = bincode::deserialize(data_buffer).unwrap();
                AllReturns::DB(types::DBReturns::GetNamespaceTagIDs(out))
            }
        },
        types::SupportedRequests::PluginCross(_plugindata) => AllReturns::Nune,
    }
}*/
