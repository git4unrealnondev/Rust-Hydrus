use std::time::{SystemTime, UNIX_EPOCH};

///
/// Returns time as seconds since unix_epoch
///
pub fn time_secs() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap()
}

///
/// Converts hour & day & minute repeatability.
///
pub fn time_conv(inp: &String) -> usize {
    if inp.to_lowercase() == *"now" {
        return 0;
    }

    let year = 31557600;
    let month = 2629800;
    let week = 604800;
    let day = 86400;
    let hour = 3600;
    let minute = 60;
    let second = 1;

    let strings = [
        "y".to_string(),
        "mo".to_string(),
        "w".to_string(),
        "d".to_string(),
        "h".to_string(),
        "m".to_string(),
        "s".to_string(),
    ];

    let nums = [year, month, week, day, hour, minute, second];

    let mut combine: usize = 0;
    let mut st = inp.clone();

    let mut cnt = 0;
    let mut ttl = 0;
    for time in strings.iter() {
        if st.contains(time) {
            let tmp: Vec<&str> = st.split(time).collect();
            if tmp[0].is_empty() {
                break;
            }
            ttl += nums[cnt] * tmp[0].parse::<usize>().unwrap();
            st = tmp[1].to_string();
        }
        cnt += 1;
    }

    /*for each in 0..strings.len() {
        dbg!(&each);
        if !st.contains(&strings[each]) {
            continue;
        }
        let tmp: Vec<&str> = st.split(&strings[each]).collect();
        if tmp[0].is_empty() {
            break;
        }
        combine += nums[each] * tmp[0].parse::<usize>().unwrap();
        st = tmp[1].to_string();
    }
    combine*/
    ttl
}
