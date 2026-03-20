#![allow(dead_code)]
#![allow(unused_variables)]

use crate::sharedtypes;
use interprocess::local_socket::prelude::LocalSocketStream;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;

pub const SOCKET_NAME: &str = "RustHydrus.sock";

#[derive(Debug, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub enum EComType {
    SendOnly,
    RecieveOnly,
    BiDirectional,
    None,
}

#[derive(Debug, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub enum EControlSigs {
    // Sending data to and fro
    Send,
    // Come to a stop naturally
    Halt,
    // STOP NOW PANIC
    Break,
}

/// Main communication block structure.
#[derive(Debug, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct Coms {
    pub com_type: EComType,
    pub control: EControlSigs,
}

// pub fn x_to_bytes<T: Sized>(input_generic: &T) -> &[u8] { unsafe {
// any_as_u8_slice(input_generic) } }
//
// /// /// Turns a generic into bytes /// unsafe fn any_as_u8_slice<T: Sized>(p:
// &T) -> &[u8] { ::core::slice::from_raw_parts((p as *const T) as *const u8,
// ::core::mem::size_of::`<T>`()) }
//
// /// /// Turns bytes into a coms structure. /// pub fn con_coms(input: &mut [u8;
// 2]) -> Coms { unsafe { std::mem::transmute(*input) } }
//
// /// /// Turns bytes into a controlsig structure. /// pub fn
// con_econtrolsigs(input: &mut [u8; 1]) -> EControlSigs { unsafe {
// std::mem::transmute(*input) } }
//
// /// /// Turns bytes into a uszie structure. /// pub fn con_u64(input: &mut
// [u8; 8]) -> u64 { unsafe { std::mem::transmute(*input) } }
//
// /// /// Turns bytes into a SupportedRequests structure. /// //pub fn
// con_supportedrequests(input: &mut [u8; 56]) -> SupportedRequests { //    unsafe
// { std::mem::transmute(*input) } //}
/// Supported Database operations.
#[derive(Debug, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub enum SupportedDBRequests {
    GetTagId(u64),
    PutTag(String, u64, Option<u64>),
    PutTagRelationship(u64, String, u64, Option<u64>),
    GetTagName((String, u64)),
    RelationshipAdd(u64, u64),
    RelationshipRemove(u64, u64),
    RelationshipGetTagid(u64),
    RelationshipGetFileid(u64),
    GetFile(u64),
    GetFileExt(u64),
    GetFileHash(String),
    GetNamespace(String),
    CreateNamespace(String, Option<String>),
    GetNamespaceTagIDs(u64),
    GetNamespaceString(u64),
    SettingsGetName(String),
    SettingsSet(String, Option<String>, Option<u64>, Option<String>),
    LoadTable(sharedtypes::LoadDBTable),
    Testu64(),
    GetFileListId(),
    GetFileListAll(),
    TransactionFlush(),
    GetDBLocation(),
    Logging(String),
    LoggingNoPrint(String),
    Search((sharedtypes::SearchObj, Option<u64>, Option<u64>)),
    GetFileByte(u64),
    GetFileLocation(u64),
    NamespaceContainsId(u64, u64),
    FilterNamespaceById((HashSet<u64>, u64)),
    PluginCallback(String, u64, sharedtypes::CallbackInfoInput),
    ReloadLoadedPlugins(),
    ParentsGet((ParentsType, u64)),
    ParentsDelete(sharedtypes::DbParentsObj),
    ParentsPut(sharedtypes::DbParentsObj),
    PutJob(sharedtypes::DbJobsObj),
    GetJob(u64),
    TagDelete(u64),
    PutFile((sharedtypes::FileObject, (u64, std::time::Duration))),
    PutFileNoBlock((sharedtypes::FileObject, (u64, std::time::Duration))),
    ReloadRegex,
    GetNamespaceIDsAll,
    MigrateTag((u64, u64)),
    MigrateRelationship((u64, u64, u64)),
    CondenseTags(),
    GetFileRaw(u64),
    GetRelationshipFileidWhereNamespace((u64, u64, sharedtypes::GreqLeqOrEq)),
    GetRelationshipTagidWhereNamespace((u64, u64, sharedtypes::GreqLeqOrEq)),
    GetFileIdsWhereExtensionIs(sharedtypes::FileExtensionType),
}

/// A descriptor for the parents and the type of data that we're sending
#[derive(Debug, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub enum ParentsType {
    Tag,
    Rel,
    LimitTo,
}

/// Actions for Database
///
/// Returns all data, general structure.
#[derive(Debug)]
pub enum AllReturns {
    DB(DBReturns),
    Plugin(SupportedPluginRequests),
    // Placeholder don't actually use. I'm using it lazizly because I'm a shitter.
    // Keep it
    Nune,
    // here for handling edge cases or nothing needs to get sent. lol
}

/// Returns the db data
#[derive(Debug)]
pub enum DBReturns {
    GetTagId(Option<sharedtypes::DbTagNNS>),
    GetTagName(Option<u64>),
    RelationshipGetTagid(Option<HashSet<u64>>),
    RelationshipGetFileid(Option<HashSet<u64>>),
    GetFile(Option<sharedtypes::DbFileObj>),
    GetFileHash(Option<u64>),
    GetNamespaceTagIDs(HashSet<u64>),
    GetNamespace(Option<u64>),
    GetNamespaceString(Option<sharedtypes::DbNamespaceObj>),
    SettingsGetName(Option<sharedtypes::DbSettingObj>),
    LoadTable(bool),
}

pub enum EfficientDataReturn {
    Data(Vec<u8>),
    Nothing,
}

/// Supported cross talks between plugins.
#[derive(Debug, Deserialize, Serialize, bitcode::Encode, bitcode::Decode)]
pub enum SupportedPluginRequests {}

/// Supported enum requests for the transaction. Will get sent over to sever /
/// client to determine what data will be sent back and forth.
#[derive(Debug, Deserialize, Serialize, bitcode::Encode, bitcode::Decode)]
pub enum SupportedRequests {
    Database(SupportedDBRequests),
    PluginCross(SupportedPluginRequests),
}

/// Writes all data into buffer.
pub fn send<T: Sized + bitcode::Encode>(inp: &T, conn: &mut BufReader<LocalSocketStream>) {
    let byte_buf = bitcode::encode(inp);
    let size = &byte_buf.len();

    conn.get_mut().write_all(&size.to_ne_bytes()).unwrap();
    conn.get_mut().write_all(&byte_buf).unwrap();
}

/// Writes all data into buffer. Assumes data is preserialzied from data generic
/// function. Can be hella dangerous. Types going in and recieved have to match
/// EXACTLY.
pub fn send_preserialize(inp: &Vec<u8>, conn: &mut BufReader<LocalSocketStream>) {
    let mut temp = inp.len().to_ne_bytes().to_vec();
    temp.extend(inp);
    let _ = conn.get_mut().write_all(&temp);
}

/// Returns a vec of bytes that represent an object
pub fn recieve<T: for<'de> bitcode::Decode<'de>>(
    conn: &mut BufReader<LocalSocketStream>,
) -> Result<T, bitcode::Error> {
    let mut u64_b = [0u8; 8];
    conn.read_exact(&mut u64_b).unwrap();
    let size_of_data = u64::from_ne_bytes(u64_b);

    let mut data_b = vec![0; size_of_data as usize];
    conn.read_exact(&mut data_b).unwrap();

    bitcode::decode(&data_b)
}
