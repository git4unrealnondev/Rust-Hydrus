#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

    mod app;
    mod fallback;
    use crate::app::*;
    //use crate::fallback::file_and_error_handler;

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
    //writer.write_all(b"benas");
    //let mut output = String::new();
    //reader.read_to_string(&mut output).unwrap();
    //dbg!(output);
    call();
}

use axum::{
    routing::get,
    Router,
};
use futures::executor::block_on;
use tokio::task;
use tokio::time::{sleep_until, Instant, Duration};

//#[cfg(feature = "ssr")]
#[tokio::main]
async fn call() {
    
    use axum::{routing::post, Router};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    
    dbg!("test");


    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        .leptos_routes(&leptos_options, routes, || view! { <App/> })
        //.fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    println!("listening on http://{}", &addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    
   /* // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:9000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
        */


}