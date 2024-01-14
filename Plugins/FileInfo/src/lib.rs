use std::time::Duration;
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
    println!("From FIleinfo plugin");

    fast_log::init(fast_log::Config::new().file("./log.txt")).unwrap();
    log::info!("FileInfo - Commencing yak shaving{}", 0);
    println!("Fileinfo waiting");
    check_existing_db();
    let mills_fifty = Duration::from_secs(5);
    std::thread::sleep(mills_fifty);
    log::info!("FileInfo - Commencing yak shaving{}", 1);
}

fn check_existing_db() {
    use std::time::Instant;
    let table = sharedtypes::LoadDBTable::Namespace;
    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::LoadTable(table),
    );
    client::init_data_request(&typerequets);

    let now = Instant::now();
    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::RelationshipGetTagid(1),
    );
    client::init_data_request(&typerequets);
    let typerequets =
        client::types::SupportedRequests::Database(client::types::SupportedDBRequests::GetFile(1));
    client::init_data_request(&typerequets);

    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::RelationshipGetTagid(1),
    );
    client::init_data_request(&typerequets);
    let typerequets =
        client::types::SupportedRequests::Database(client::types::SupportedDBRequests::GetFile(1));
    client::init_data_request(&typerequets);

    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::RelationshipGetTagid(1),
    );
    client::init_data_request(&typerequets);
    let typerequets =
        client::types::SupportedRequests::Database(client::types::SupportedDBRequests::GetFile(1));
    client::init_data_request(&typerequets);

    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::RelationshipGetTagid(1),
    );
    client::init_data_request(&typerequets);
    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::GetNamespaceString(1),
    );
    let ex = client::init_data_request(&typerequets);
    dbg!(ex);
    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::GetNamespaceString(1),
    );
    let ex = client::init_data_request(&typerequets);
    dbg!(ex);

    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::GetNamespaceString(1),
    );
    let ex = client::init_data_request(&typerequets);
    dbg!(ex);
    let typerequets = client::types::SupportedRequests::Database(
        client::types::SupportedDBRequests::GetNamespaceString(1),
    );
    let ex = client::init_data_request(&typerequets);

    dbg!(ex);
    let typerequets =
        client::types::SupportedRequests::Database(client::types::SupportedDBRequests::GetFile(1));
    let ex = client::init_data_request(&typerequets);
    dbg!(ex);
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
