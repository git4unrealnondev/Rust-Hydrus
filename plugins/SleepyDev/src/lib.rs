#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

static PLUGIN_NAME: &str = "SleepyDev";
static PLUGIN_DESCRIPTION: &str = "Just makes the plugin manager wait. Dev use only lol";

#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut main = sharedtypes::return_default_globalpluginparser();
    main.name = PLUGIN_NAME.to_string();
    main.version = 0;
    main.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_type: sharedtypes::PluginThreadType::Spawn,
            com_channel: true,
            redirect: None,
        },
    ));
    main.callbacks = vec![sharedtypes::GlobalCallbacks::Start];
    let out = vec![main];

    out
}

#[no_mangle]
pub fn on_start() {
    use std::{thread, time};
    let wait = time::Duration::from_secs(1);
    loop {
        thread::sleep(wait);
    }
}
