#![allow(unused_variables)]
#![allow(dead_code)]

use crate::sharedtypes::DbFileStorage;
use crate::sharedtypes::{self};
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::ToNsName;
use interprocess::local_socket::prelude::LocalSocketStream;
use interprocess::local_socket::traits::Stream;
use std::collections::{HashMap, HashSet};
use std::io::BufReader;
use std::time::Duration;

#[path = "types.rs"]
pub mod types;
//use crate::types;
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

///
/// Adds file into db downloads if needed. Does not block execution until done.
/// Technically will block but only if theirs 1000 downloads going at once
///
pub fn add_file_nonblocking(file: sharedtypes::FileObject, ratelimit: (u64, Duration)) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutFileNoBlock((file, ratelimit)),
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
///
/// Gets the dbfilestorage refence for everything
///
pub fn get_file_raw(fileid: usize) -> Option<DbFileStorage> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileRaw(fileid),
    ))
}

///
/// Returns all filesids of type
///
pub fn get_fileid_where_extension(extype: sharedtypes::FileExtensionType) -> HashSet<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileIdsWhereExtensionIs(extype),
    ))
}

pub fn get_file_ext(fileext: usize) -> Option<String> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetFileExt(fileext),
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

/// Logs to the log and prints
pub fn log(log: String) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::Logging(log),
    ))
}

/// Logs but doesn't print
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
) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::SettingsSet(name, pretty, num, param),
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
pub fn namespace_put(name: String, description: Option<String>) -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::CreateNamespace(name, description),
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
pub fn relationship_add(file: usize, tag: usize) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::RelationshipAdd(file, tag),
    ))
}

///
/// Removes a relationship from the db
///
pub fn relationship_remove(file: usize, tag: usize) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::RelationshipRemove(file, tag),
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
pub fn relationship_get_fileid(id: usize) -> HashSet<usize> {
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
pub fn relationship_get_tagid(id: usize) -> HashSet<usize> {
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
pub fn namespace_get_tagids(id: usize) -> HashSet<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespaceTagIDs(id),
    ))
}

/// Returns the parents relationship The Tag returns data about the parent The Rel
/// returns data about the parent to it's relation
///
/// It's basically a 2 way pointer like the Relations table limit_to limits the
/// exposure of
pub fn parents_get(
    parenttype: types::ParentsType,
    id: usize,
) -> HashSet<sharedtypes::DbParentsObj> {
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

pub fn tag_remove(tag_id: usize) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::TagDelete(tag_id),
    ))
}

/// Adds a parent into the db returns the unique of the parent inserted
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
pub fn tag_add(tag: String, namespace_id: usize, id: Option<usize>) -> usize {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutTag(tag, namespace_id, id),
    ))
}
///
/// Migrates a tag from id to another id
///
pub fn tag_migrate(old_tagid: usize, new_tag_id: usize) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::MigrateTag((old_tagid, new_tag_id)),
    ))
}
///
/// Migrates a tag from id to another id
///
pub fn file_relationship_migrate(file_id: usize, old_tagid: usize, new_tag_id: usize) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::MigrateRelationship((file_id, old_tagid, new_tag_id)),
    ))
}

///
/// Reloads regex
///
pub fn reload_regex() -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::ReloadRegex,
    ))
}

///
/// Gets a list of loaded namespace IDs
///
pub fn namespace_get_tagids_all() -> Vec<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetNamespaceIDsAll,
    ))
}

/// Adds job into db
pub fn job_add(job: sharedtypes::DbJobsObj) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutJob(job),
    ))
}
/// Condenses tags in the db
pub fn condense_tags() -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::CondenseTags(),
    ))
}

///
/// Gets all tag ids that have more then count relations with a file
/// Can be slow
///
pub fn relationship_get_tagid_where_namespace_count(
    namespace_id: usize,
    count: usize,
    dir: sharedtypes::GreqLeqOrEq,
) -> Vec<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetRelationshipTagidWhereNamespace((namespace_id, count, dir)),
    ))
}
///
/// Gets all file ids that have more then count relations with a file
/// Can be slow
///
pub fn relationship_get_fileid_where_namespace_count(
    namespace_id: usize,
    count: usize,
    dir: sharedtypes::GreqLeqOrEq,
) -> Vec<usize> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::GetRelationshipFileidWhereNamespace((namespace_id, count, dir)),
    ))
}

/// See the database reference for this function. I'm a lazy turd just check it
/// their
pub fn relationship_file_tag_add(
    fileid: usize,
    tag: String,
    namespace_id: usize,
    id: Option<usize>,
) -> bool {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PutTagRelationship(fileid, tag, namespace_id, id),
    ))
}

/// Calls an external plugin from inside the plugin manager if it exists Should
/// work will test tomorrow
pub fn external_plugin_call(
    func_name: String,
    vers: usize,
    input: sharedtypes::CallbackInfoInput,
) -> HashMap<String, sharedtypes::CallbackCustomDataReturning> {
    init_data_request(&types::SupportedRequests::Database(
        types::SupportedDBRequests::PluginCallback(func_name, vers, input),
    ))
}

/// This shouldn't come back to haunt me. :x Returns a Vec of bytes that represent
/// the data structure sent from server.
fn init_data_request<T: serde::de::DeserializeOwned>(requesttype: &types::SupportedRequests) -> T {
    let name = types::SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .unwrap();
    let conn;
    loop {
        // Wait indefinitely for this to get a connection. shit way of doing it will
        // likely add a wait or something this will likely block the CPU or something.
        let worker_id;
        loop {
            let temp_conn = LocalSocketStream::connect(name.clone());
            if let Ok(con_ok) = temp_conn {
                let mut buf_reader = BufReader::new(con_ok);
                if let Ok(id) = types::recieve::<u64>(&mut buf_reader) {
                    worker_id = id;
                    break;
                }
            }
        }
        if let Ok(conn_out) = LocalSocketStream::connect(
            format!("{}_{}", types::SOCKET_NAME, worker_id)
                .to_ns_name::<GenericNamespaced>()
                .unwrap(),
        ) {
            conn = conn_out;
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
