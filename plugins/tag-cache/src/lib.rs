use fuzzy_search::automata::LevenshteinAutomata;
use std::thread

#[path = "../../../src/sharedtypes.rs"]
mod sharedtypes;

#[path = "../../../src/client.rs"]
mod client;

static PLUGIN_NAME: &str = "Tag Cache";

#[unsafe(no_mangle)]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let mut main = sharedtypes::return_default_globalpluginparser();
    main.name = PLUGIN_NAME.to_string();
    main.version = 0;
    main.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_channel: true,
            redirect: None,
        },
    ));
    main.callbacks = vec![
        sharedtypes::GlobalCallbacks::Start(sharedtypes::StartupThreadType::SpawnInline),
        sharedtypes::GlobalCallbacks::Tag((None, vec![], vec![])),
        sharedtypes::GlobalCallbacks::Callback(sharedtypes::CallbackInfo {
            func: "plugin_tag_cache_get_tag".to_string(),
            vers: 0,
            data_name: vec![
                "plugin_tag_cache_search".to_string(),
                "plugin_tag_cache_add_tag".to_string(),
            ],
            data: vec![
                sharedtypes::CallbackCustomData::VString,
                sharedtypes::CallbackCustomData::VString,
            ],
        }),
    ];
    let out = vec![main];

    out
}

#[unsafe(no_mangle)]
pub fn on_start() {}
