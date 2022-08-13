use std::time::{SystemTime, UNIX_EPOCH};

///
/// Returns time as seconds since unix_epoch
///
pub fn time_secs() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs().into()
}



///
/// Converts hour & day & minute repeatability.
///
pub fn time_conv(inp: String) -> u128 {
    if inp == "now".to_string() {
        return 0;
    }

    let year = 31557600;
    let month = 2629800;
    let week = 604800;
    let day = 86400;
    let hour = 3600;
    let minute = 60;
    let second = 1;

    let strings = vec![
        "y".to_string(),
        "m".to_string(),
        "w".to_string(),
        "d".to_string(),
        "h".to_string(),
        "m".to_string(),
        "s".to_string(),
    ];

    let nums = vec![year, month, week, day, hour, minute, second];

    let mut combine: u128 = 0;
    let mut st = inp;

    for each in 0..strings.len() {
        if st.contains(&strings[each]) == false {
            continue;
        }
        let tmp: Vec<&str> = st.split(&strings[each]).collect();
        if tmp[0].to_string() == "" {
            break;
        }
        combine += nums[each] * tmp[0].parse::<u128>().unwrap();
        st = tmp[1].to_string();
    }
    return combine;
}
