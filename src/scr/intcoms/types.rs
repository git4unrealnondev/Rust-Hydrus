#[derive(Debug)]
pub enum eComType {
    SendOnly,
    RecieveOnly,
    BiDirectional,
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
/// Turns a coms structure into bytes.
///
pub fn coms_to_bytes(sComs: &coms) -> &[u8] {
    unsafe { any_as_u8_slice(sComs) }
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
