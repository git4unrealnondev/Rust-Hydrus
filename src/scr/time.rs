extern crate chrono;
use chrono::NaiveDateTime;
use std::time::{SystemTime, UNIX_EPOCH};

///
/// Returns time as seconds since unix_epoch
///
pub fn time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

///
/// Converts time as seconds to UTC.
///
pub fn time_utc(inp: isize) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(inp.try_into().unwrap(), 0)
}

///
/// Converts hour & day & minute repeatability.
///
pub fn time_conv(inp: String) -> usize {64}
