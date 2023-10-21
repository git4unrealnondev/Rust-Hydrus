#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

use std::io::Write;
use std::io::Read;

static PLUGIN_NAME:&str = "WebAPI";
static PLUGIN_DESCRIPTION:&str = "Adds support for WebUI & WebAPI..";

#[no_mangle]
pub fn return_info() -> sharedtypes::PluginInfo {
    let callbackvec = vec![sharedtypes::PluginCallback::OnStart];
    sharedtypes::PluginInfo {
        name: PLUGIN_NAME.to_string(),
        description: PLUGIN_DESCRIPTION.to_string(),
        version: 1.00,
        api_version: 1.00,
        callbacks: callbackvec,
        communication: Some(sharedtypes::PluginSharedData { thread: sharedtypes::PluginThreadType::Daemon, com_channel: Some(sharedtypes::PluginCommunicationChannel::pipe("beans".to_string())) }),
    }
}

#[no_mangle]

pub fn on_start(reader: &mut os_pipe::PipeReader,writer: &mut os_pipe::PipeWriter) {
    println!("Starting QR Generator");
    writer.write_all(b"benas");
    let mut output = String::new();
    reader.read_to_string(&mut output).unwrap();
    dbg!(output);
    call();
}

use axum::{
    routing::get,
    Router,
};
use futures::executor::block_on;
use tokio::task;
use tokio::time::{sleep_until, Instant, Duration};

#[tokio::main]
async fn call() {
    dbg!("a");
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:9000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
        
    //sleep_until(Instant::now() + Duration::from_secs(100)).await;
    println!("100 ms have elapsed");
    dbg!("b");

}