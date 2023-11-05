#[derive(Debug)]
pub enum eComType {
    SendOnly,
    RecieveOnly,
    BiDirectional,
    None,
}

#[derive(Debug)]
pub enum eControlSigs {
    SEND,  // Sending data to and fro
    HALT,  // Come to a stop naturally
    BREAK, // STOP NOW PANIC
}

pub struct coms {
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
pub fn con_coms(input: &mut [u8; 2]) -> coms {
    unsafe { std::mem::transmute(*input) }
}

///
/// Turns bytes into a controlsig structure.
///
pub fn con_econtrolsigs(input: &mut [u8; 1]) -> eControlSigs {
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
    relationship_get_tagid(usize),
    file_get_id(usize),
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
