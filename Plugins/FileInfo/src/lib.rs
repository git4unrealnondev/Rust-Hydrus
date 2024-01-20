use std::time::Duration;
use std::time::Instant;
#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[path = "../../../src/scr/intcoms/client.rs"]
mod client;
static PLUGIN_NAME: &str = "File Info";
static PLUGIN_DESCRIPTION: &str = "Gets information from a file.";

#[no_mangle]
pub fn return_info() -> sharedtypes::PluginInfo {
    let callbackvec = vec![sharedtypes::PluginCallback::OnStart];
    sharedtypes::PluginInfo {
        name: PLUGIN_NAME.to_string(),
        description: PLUGIN_DESCRIPTION.to_string(),
        version: 1.00,
        api_version: 1.00,
        callbacks: callbackvec,
        communication: Some(sharedtypes::PluginSharedData {
            thread: sharedtypes::PluginThreadType::Inline,
            com_channel: Some(sharedtypes::PluginCommunicationChannel::pipe(
                "beans".to_string(),
            )),
        }),
    }
}
#[no_mangle]
pub fn on_start() {
    println!("From Fileinfo plugin");

    fast_log::init(fast_log::Config::new().file("./log.txt")).unwrap();
    log::info!("FileInfo - Commencing yak shaving{}", 0);
    println!("Fileinfo waiting");
    check_existing_db();
    log::info!("FileInfo - Commencing yak shaving{}", 1);
}

fn check_existing_db() {
    //std::thread::sleep(Duration::from_secs(1));
    let table = sharedtypes::LoadDBTable::Namespace;
    client::load_table(table);
    let table = sharedtypes::LoadDBTable::Tags;
    client::load_table(table);
    let now = Instant::now();
    for i in 0..30000 {
        let test = client::tag_get_id(i);
        //println!("{:?}", test);
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
