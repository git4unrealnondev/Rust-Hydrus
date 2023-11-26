#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

mod app;
mod fallback;
use crate::app::*;
//use crate::fallback::file_and_error_handler;

use actix_web::dev::Server;
use async_std::task;
use std::io::Read;
use std::io::Write;

static PLUGIN_NAME: &str = "WebAPI";
static PLUGIN_DESCRIPTION: &str = "Adds support for WebUI & WebAPI..";

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
            thread: sharedtypes::PluginThreadType::Daemon,
            com_channel: Some(sharedtypes::PluginCommunicationChannel::pipe(
                "beans".to_string(),
            )),
        }),
    }
}

#[cfg(feature = "ssr")]
#[no_mangle]
pub fn on_start(reader: &mut os_pipe::PipeReader, writer: &mut os_pipe::PipeWriter) {
    task::block_on(call());
}

use futures::executor::block_on;

use std::thread;

use std::path::PathBuf;
fn get_current_working_dir() -> std::io::Result<PathBuf> {
    use std::env;
    env::current_dir()
}
#[cfg(feature = "ssr")]
#[actix_web::main]
async fn call() -> Server {
    use actix_files::Files;
    use actix_web::dev::Server;
    use actix_web::*;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use tokio::time::{sleep_until, Duration, Instant};
    dbg!(get_current_working_dir());
    let conf = get_configuration(Some("./Plugins/webapi/Cargo.toml"))
        .await
        .unwrap();
    let addr = conf.leptos_options.site_addr;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);
    println!("listening on http://{}", &addr);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            // serve JS/WASM/CSS from `pkg`
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            // serve other assets from the `assets` directory
            .service(Files::new("/assets", site_root))
            // serve the favicon from /favicon.ico
            //.service(favicon)
            .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .app_data(web::Data::new(leptos_options.to_owned()))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)
    .unwrap()
    .run()
}
