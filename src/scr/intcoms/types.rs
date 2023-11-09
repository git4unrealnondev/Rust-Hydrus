use crate::sharedtypes::DbTagObj;

#[derive(Debug)]
pub enum eComType {
    SendOnly,
    RecieveOnly,
    BiDirectional,
    None,
}

#[derive(Debug)]
pub enum eControlSigs {
    Send,  // Sending data to and fro
    Halt,  // Come to a stop naturally
    Break, // STOP NOW PANIC
}

///
/// Main communication block structure.
///
pub struct Coms {
    pub com_type: eComType,
    pub control: eControlSigs,
}

///
/// Turns a ""x"" structure into bytes.
/// Anything from X into bytes
///
pub fn x_to_bytes<T>(input_generic: &T) -> &[u8] {
    unsafe { any_as_u8_slice(input_generic) }
}

///
/// Turns a generic into bytes
///
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

///
/// Turns bytes into a coms structure.
///
pub fn con_coms(input: &mut [u8; 2]) -> Coms {
    unsafe { std::mem::transmute(*input) }
}

///
/// Turns bytes into a controlsig structure.
///
pub fn con_econtrolsigs(input: &mut [u8; 1]) -> eControlSigs {
    unsafe { std::mem::transmute(*input) }
}

///
/// Turns bytes into a uszie structure.
///
pub fn con_usize(input: &mut [u8; 8]) -> usize {
    unsafe { std::mem::transmute(*input) }
}

///
/// Turns bytes into a SupportedRequests structure.
///
pub fn con_supportedrequests(input: &mut [u8; 16]) -> SupportedRequests {
    unsafe { std::mem::transmute(*input) }
}

///
/// Supported Database operations.
///
#[derive(Debug)]
pub enum SupportedDBRequests {
    db_tag_id_get(usize),
    db_relationship_get_tagid(usize),
    db_get_file(usize),
    db_relationship_get_fileid(usize),
}

///
/// Supported cross talks between plugins.
///
#[derive(Debug)]
pub enum SupportedPluginRequests {}

///
/// Supported enum requests for the transaction.
/// Will get sent over to sever / client to determine what data will be sent back and forth.
///
#[derive(Debug)]
pub enum SupportedRequests {
    Database(SupportedDBRequests),
    PluginCross(SupportedPluginRequests),
}

///
/// Will send over arbitrary data
///
pub struct ArbitraryData {
    pub buffer_size: usize,
    pub buffer_data: Vec<u8>,
}

pub fn demo<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}
