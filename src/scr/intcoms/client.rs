#![allow(dead_code)]
#![allow(unused_variables)]

use crate::sharedtypes::{self};
use anyhow::Context;
use interprocess::local_socket::prelude::LocalSocketStream;
use interprocess::local_socket::traits::Stream;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::ToNsName;
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::io::BufReader;
use std::time::Duration;

pub mod types;

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn search_db_files(
    search: sharedtypes::SearchObj,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Option<HashSet<usize>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::Search((search, limit, offset)),
    ))
}

///
/// Adds file into db downloads if needed. Blocks execution until done
///
pub fn add_file(file: sharedtypes::FileObject, ratelimit: (u64, Duration)) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutFile((file, ratelimit)),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn load_table(table: sharedtypes::LoadDBTable) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::LoadTable(table),
    ))
}

pub fn get_file(fileid: usize) -> Option<String> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileLocation(fileid),
    ))
}

pub fn get_file_bytes(fileid: usize) -> Option<Vec<u8>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileByte(fileid),
    ))
}

/// Reloads the loaded plugins
pub fn reload_loaded_plugins() -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::ReloadLoadedPlugins(),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn namespace_get_string(id: usize) -> Option<sharedtypes::DbNamespaceObj> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespaceString(id),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn namespace_contains_id(namespaceid: usize, tagid: usize) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::NamespaceContainsId(namespaceid, tagid),
    ))
}

pub fn filter_namespaces_by_id(tags: HashSet<usize>, namespaceid: usize) -> HashSet<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::FilterNamespaceById((tags, namespaceid)),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn log(log: String) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::Logging(log),
    ))
}

pub fn log_no_print(log: String) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::LoggingNoPrint(log),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn setting_add(
    name: String,
    pretty: Option<String>,
    num: Option<usize>,
    param: Option<String>,
    addtodb: bool,
) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::SettingsSet(name, pretty, num, param, addtodb),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn namespace_get(name: String) -> Option<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespace(name),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn namespace_put(name: String, description: Option<String>, addtodb: bool) -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::CreateNamespace(name, description, addtodb),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn testusize() -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::TestUsize(),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn transaction_flush() -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::TransactionFlush(),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn relationship_add(file: usize, tag: usize, addtodb: bool) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::RelationshipAdd(file, tag, addtodb),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn file_get_list_id() -> HashSet<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileListId(),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn relationship_get_fileid(id: usize) -> Option<HashSet<usize>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::RelationshipGetFileid(id),
    ))
}

/// Gets a file based on their ID
pub fn file_get_id(fid: usize) -> Option<sharedtypes::DbFileStorage> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFile(fid),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn relationship_get_tagid(id: usize) -> Option<HashSet<usize>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::RelationshipGetTagid(id),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn tag_get_name(tag: String, namespaceid: usize) -> Option<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetTagName((tag, namespaceid)),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn file_get_list_all() -> HashMap<usize, sharedtypes::DbFileStorage> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileListAll(),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn settings_get_name(name: String) -> Option<sharedtypes::DbSettingObj> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::SettingsGetName(name),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn tag_get_id(id: usize) -> Option<sharedtypes::DbTagNNS> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetTagId(id),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn namespace_get_tagids(id: usize) -> Option<HashSet<usize>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespaceTagIDs(id),
    ))
}

/// Returns the parents relationship The Tag returns data about the parent The Rel
/// returns data about the parent to it's relation
///
/// It's basically a 2 way pointer like the Relations table limit_to limits the
/// exposure of
pub fn parents_get(parenttype: types::ParentsType, id: usize) -> Option<HashSet<usize>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::ParentsGet((parenttype, id)),
    ))
}

/// Deletes a parent If idtwo is set to none then this deletes all relationships
/// that match the key
pub fn parents_delete(parentobj: sharedtypes::DbParentsObj) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::ParentsDelete(parentobj),
    ))
}

/// Adds a parent into the db returns the cantor pair of the parent inserted
pub fn parents_put(parentobj: sharedtypes::DbParentsObj) -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::ParentsPut(parentobj),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn location_get() -> String {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetDBLocation(),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn tag_add(tag: String, namespace_id: usize, addtodb: bool, id: Option<usize>) -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutTag(tag, namespace_id, addtodb, id),
    ))
}

/// Adds job into db
pub fn job_add(
    id: Option<usize>,
    time: usize,
    reptime: usize,
    site: String,
    param: String,
    addtodb: bool,
    committype: sharedtypes::CommitType,
    dbjobtype: sharedtypes::DbJobType,
    system_data: BTreeMap<String, String>,
    user_data: BTreeMap<String, String>,
    jobsmanager: sharedtypes::DbJobsManager,
) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutJob((
            id,
            time,
            reptime,
            site,
            param,
            addtodb,
            committype,
            dbjobtype,
            system_data,
            user_data,
            jobsmanager,
        )),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn relationship_file_tag_add(
    fileid: usize,
    tag: String,
    namespace_id: usize,
    addtodb: bool,
    id: Option<usize>,
) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutTagRelationship(fileid, tag, namespace_id, addtodb, id),
    ))
}

/// Calls an external plugin from inside the plugin manager if it exists Should
/// work will test tomorrow
pub fn external_plugin_call(
    func_name: String,
    vers: usize,
    input: sharedtypes::CallbackInfoInput,
) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PluginCallback(func_name, vers, input),
    ))
}

/// This shouldn't come back to haunt me. :x Returns a Vec of bytes that represent
/// the data structure sent from server.
fn init_data_request<T: serde::de::DeserializeOwned>(requesttype: &types::SupportedRequests) -> T {
    let _coms_struct = types::Coms {
        com_type: types::EComType::BiDirectional,
        control: types::EControlSigs::Send,
    };
    let name = types::SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .unwrap();
    let conn;
    loop {
        // Wait indefinitely for this to get a connection. shit way of doing it will
        // likely add a wait or something this will likely block the CPU or something.
        let temp_conn =
            LocalSocketStream::connect(name.clone()).context("Failed to connect to server");
        if let Ok(con_ok) = temp_conn {
            conn = con_ok;
            break;
        }
    }

    // Wrap it into a buffered reader right away so that we could read a single line
    // out of it.
    let mut conn = BufReader::new(conn);

    // Requesting data from server.
    types::send(requesttype, &mut conn);

    // Recieving size Data from server
    match types::recieve(&mut conn) {
        Ok(out) => out,
        Err(err) => {
            dbg!(err, requesttype);
            panic!();
        }
    }
}
