#![allow(dead_code)]
#![allow(unused_variables)]

use crate::sharedtypes;
use anyhow::Context;
use interprocess::local_socket::prelude::LocalSocketStream;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;

pub const SOCKET_NAME: &str = "RustHydrus.sock";

#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum EComType {
    SendOnly,
    RecieveOnly,
    BiDirectional,
    None,
}

#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum EControlSigs {
    // Sending data to and fro
    Send,
    // Come to a stop naturally
    Halt,
    // STOP NOW PANIC
    Break,
}

/// Main communication block structure.
#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
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
// /// /// Turns bytes into a uszie structure. /// pub fn con_usize(input: &mut
// [u8; 8]) -> usize { unsafe { std::mem::transmute(*input) } }
//
// /// /// Turns bytes into a SupportedRequests structure. /// //pub fn
// con_supportedrequests(input: &mut [u8; 56]) -> SupportedRequests { //    unsafe
// { std::mem::transmute(*input) } //}
/// Supported Database operations.
#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum SupportedDBRequests {
    GetTagId(usize),
    PutTag(String, usize, bool, Option<usize>),
    PutTagRelationship(usize, String, usize, bool, Option<usize>),
    GetTagName((String, usize)),
    RelationshipAdd(usize, usize, bool),
    RelationshipGetTagid(usize),
    RelationshipGetFileid(usize),
    GetFile(usize),
    GetFileExt(usize),
    GetFileHash(String),
    GetNamespace(String),
    CreateNamespace(String, Option<String>, bool),
    GetNamespaceTagIDs(usize),
    GetNamespaceString(usize),
    SettingsGetName(String),
    SettingsSet(String, Option<String>, Option<usize>, Option<String>, bool),
    LoadTable(sharedtypes::LoadDBTable),
    TestUsize(),
    GetFileListId(),
    GetFileListAll(),
    TransactionFlush(),
    GetDBLocation(),
    Logging(String),
    LoggingNoPrint(String),
    Search((sharedtypes::SearchObj, Option<usize>, Option<usize>)),
    GetFileByte(usize),
    GetFileLocation(usize),
    NamespaceContainsId(usize, usize),
    FilterNamespaceById((HashSet<usize>, usize)),
    PluginCallback(String, usize, sharedtypes::CallbackInfoInput),
    ReloadLoadedPlugins(),
    ParentsGet((ParentsType, usize)),
    ParentsDelete(sharedtypes::DbParentsObj),
    ParentsPut(sharedtypes::DbParentsObj),
    PutJob(sharedtypes::DbJobsObj),
    GetJob(usize),
    PutFile((sharedtypes::FileObject, (u64, std::time::Duration))),
    PutFileNoBlock((sharedtypes::FileObject, (u64, std::time::Duration))),
    ReloadRegex,
    GetNamespaceIDsAll,
}

/// A descriptor for the parents and the type of data that we're sending
#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum ParentsType {
    Tag,
    Rel,
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
    GetTagName(Option<usize>),
    RelationshipGetTagid(Option<HashSet<usize>>),
    RelationshipGetFileid(Option<HashSet<usize>>),
    GetFile(Option<sharedtypes::DbFileObj>),
    GetFileHash(Option<usize>),
    GetNamespaceTagIDs(HashSet<usize>),
    GetNamespace(Option<usize>),
    GetNamespaceString(Option<sharedtypes::DbNamespaceObj>),
    SettingsGetName(Option<sharedtypes::DbSettingObj>),
    LoadTable(bool),
}

pub enum EfficientDataReturn {
    Data(Vec<u8>),
    Nothing,
}

/// Supported cross talks between plugins.
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum SupportedPluginRequests {}

/// Supported enum requests for the transaction. Will get sent over to sever /
/// client to determine what data will be sent back and forth.
#[derive(Debug, Deserialize, Serialize, bincode::Encode, bincode::Decode)]
pub enum SupportedRequests {
    Database(SupportedDBRequests),
    PluginCross(SupportedPluginRequests),
}

/// Writes all data into buffer.
pub fn send<T: Sized + Serialize + bincode::Encode>(
    inp: T,
    conn: &mut BufReader<LocalSocketStream>,
) {
    let byte_buf = bincode::serde::encode_to_vec(&inp, bincode::config::standard()).unwrap();
    let size = &byte_buf.len();

    conn.get_mut()
        .write_all(&size.to_ne_bytes())
        .context("Socket send failed")
        .unwrap();
    conn.get_mut()
        .write_all(&byte_buf)
        .context("Socket send failed")
        .unwrap();
}

/// Writes all data into buffer. Assumes data is preserialzied from data generic
/// function. Can be hella dangerous. Types going in and recieved have to match
/// EXACTLY.
pub fn send_preserialize(inp: &Vec<u8>, conn: &mut BufReader<LocalSocketStream>) {
    let mut temp = inp.len().to_ne_bytes().to_vec();
    temp.extend(inp);
    let _ = conn
        .get_mut()
        .write_all(&temp)
        .context("Socket send failed");
}

/// Returns a vec of bytes that represent an object
pub fn recieve<T: serde::de::DeserializeOwned>(
    conn: &mut BufReader<LocalSocketStream>,
) -> Result<T, anyhow::Error> {
    let mut usize_b: [u8; std::mem::size_of::<usize>()] = [0; std::mem::size_of::<usize>()];
    let _ = conn
        .get_mut()
        .read_exact(&mut usize_b[..])
        .context("Socket send failed");
    let size_of_data: usize = usize::from_ne_bytes(usize_b);
    let mut data_b = vec![0; size_of_data];
    let _ = conn
        .get_mut()
        .read_exact(&mut data_b[..])
        .context("Socket send failed");

    let out = bincode::serde::decode_from_slice(&data_b, bincode::config::standard())?;
    Ok(out.0)
}
