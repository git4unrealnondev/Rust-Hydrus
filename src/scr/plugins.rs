use bytes::Bytes;
use libloading;
use log::{error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

use crate::sharedtypes;
use crate::{database, download};

pub struct PluginManager {
    _plugin: HashMap<String, libloading::Library>,
    _callback: HashMap<sharedtypes::PluginCallback, Vec<String>>,
    _database: Arc<Mutex<database::Main>>,
}

///
/// Plugin Manager Handler
///
impl PluginManager {
    pub fn new(pluginsloc: String, MainDb: Arc<Mutex<database::Main>>) -> Self {
        let mut reftoself = PluginManager {
            _plugin: HashMap::new(),
            _callback: HashMap::new(),
            _database: MainDb.clone(),
        };

        reftoself.load_plugins(&pluginsloc);

        reftoself
    }

    ///
    /// Parses output from plugin.
    ///
    fn ParsePluginOutput(&mut self, plugin_data: Vec<sharedtypes::DBPluginOutputEnum>) {
        let mut unwrappy = self._database.lock().unwrap();

        for each in plugin_data {
            match each {
                sharedtypes::DBPluginOutputEnum::Add(name) => {
                    for names in name {
                        let mut namespace_id: Option<usize> = Some(0); // holder of namespace

                        // Loops through the namespace objects and selects the last one that's valid.
                        // IF ONE IS NOT VALID THEN THEIR WILL NOT BE ONE ADDED INTO THE DB
                        if let Some(temp) = names.namespace {
                            for namespace in temp {
                                // IF Valid ID && Name && Description info are valid then we have a valid namespace id to pull
                                if namespace.id == None
                                    && namespace.name != None
                                    && namespace.description != None
                                    && namespace_id != None
                                {
                                    namespace_id = Some(unwrappy.namespace_add(
                                        &namespace.name.unwrap(),
                                        &namespace.description.unwrap(),
                                        true,
                                    ));
                                } else {
                                    namespace_id = None;
                                    continue;
                                }
                            }
                        }
                        if let Some(temp) = names.file {
                            for files in temp {
                                if files.id == None && files.hash != None && files.ext != None {
                                    if files.location == None {
                                        //let string_dlpath = download::getfinpath(&loc, &files.hash.as_ref().unwrap());
                                        let location = unwrappy
                                            .settings_get_name(&"FilesLoc".to_string())
                                            .unwrap()
                                            .param;
                                        unwrappy.file_add(
                                            None,
                                            files.hash.unwrap(),
                                            files.ext.unwrap(),
                                            location.unwrap(),
                                            true,
                                        );
                                    } else {
                                        unwrappy.file_add(
                                            files.id,
                                            files.hash.unwrap(),
                                            files.ext.unwrap(),
                                            files.location.unwrap(),
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                        if let Some(temp) = names.tag {
                            for tags in temp {
                                let namespace_id = unwrappy.namespace_get(&tags.namespace);
                                if tags.parents == None && !namespace_id.is_none() {
                                    unwrappy.tag_add(
                                        tags.name,
                                        "".to_string(),
                                        namespace_id.unwrap(),
                                        true,
                                        None,
                                    );
                                } else {
                                    for parents_obj in tags.parents.unwrap() {
                                        unwrappy.tag_add(
                                            tags.name.to_string(),
                                            parents_obj.relate_tag_id,
                                            namespace_id.unwrap(),
                                            true,
                                            None,
                                        );
                                    }
                                }
                            }
                        }
                        if let Some(temp) = names.setting {
                            for settings in temp {
                                unwrappy.setting_add(
                                    settings.name,
                                    settings.pretty.unwrap(),
                                    settings.num,
                                    settings.param.unwrap(),
                                    true,
                                );
                            }
                        }

                        if let Some(temp) = names.jobs {}
                        if let Some(temp) = names.relationship {
                            for relations in temp {
                                let file_id = unwrappy.file_get_hash(&relations.file_hash);
                                let namespace_id = unwrappy.namespace_get(&relations.tag_namespace);
                                let tag_id = unwrappy
                                    .tag_get_name(relations.tag_name, namespace_id.unwrap());
                                unwrappy.relationship_add(file_id.0, tag_id.unwrap(), true);
                                //unwrappy.relationship_add(file, tag, addtodb)
                            }
                        }
                        if let Some(temp) = names.parents {}
                    }
                }
                sharedtypes::DBPluginOutputEnum::Del(name) => for names in name {},
                sharedtypes::DBPluginOutputEnum::None => {}
            }
        }
    }

    ///
    /// Runs plugin and
    ///
    pub fn plugin_on_download(&mut self, cursorpass: &[u8], hashs: &String, exts: &String) {
        if !self
            ._callback
            .contains_key(&sharedtypes::PluginCallback::OnDownload)
        {
            return;
        }

        for plugin in self._callback[&sharedtypes::PluginCallback::OnDownload].clone() {
            if !self._plugin.contains_key(&plugin) {
                error!("Could not call Plugin-OnDownload");
                continue;
            }
            let lib = self._plugin.get_mut(&plugin).unwrap();
            let output;
            unsafe {
                let plugindatafunc: libloading::Symbol<
                    unsafe extern "C" fn(
                        &[u8],
                        &String,
                        &String,
                    )
                        -> Vec<sharedtypes::DBPluginOutputEnum>,
                    //unsafe extern "C" fn(Cursor<Bytes>, &String, &String, Arc<Mutex<database::Main>>),
                > = lib.get(b"on_download").unwrap();
                //unwrappy.
                output = plugindatafunc(cursorpass, hashs, exts);
            }

            self.ParsePluginOutput(output);
        }
    }

    ///
    /// Loads plugins into plugin manager
    ///
    fn load_plugins(&mut self, pluginsloc: &String) {
        println!("Starting to load plugins at: {}", pluginsloc);

        let ext = ["rlib", "so", "dylib", "dll"];

        let plugin_path = Path::new(pluginsloc);

        // Errors out if I cant create a folder
        if !plugin_path.exists() {
            let path_check = fs::create_dir_all(&plugin_path);
            match path_check {
                Ok(_) => (),
                Err(_) => panic!(
                    "{}",
                    format!("CANNOT CREATE FOLDER: {} DUE TO PERMISSIONS.", &pluginsloc)
                ),
            }
        }

        let dirs = fs::read_dir(&plugin_path).unwrap();

        for entry in dirs {
            let root: String = entry.as_ref().unwrap().path().display().to_string();
            let name = root.split('/');
            let vec: Vec<&str> = name.collect();

            let plugin_loading_path = format!(
                "{}{}/target/release/lib{}",
                &pluginsloc,
                vec[vec.len() - 1],
                vec[vec.len() - 1]
            );

            dbg!(&root, &pluginsloc, vec[vec.len() - 1]);

            dbg!(&plugin_loading_path);

            let mut finalpath: Option<String> = None;
            'extloop: for exts in ext {
                let testpath = format!("{}.{}", plugin_loading_path, exts);

                // Loading Logic goes here.
                if Path::new(&testpath).exists() {
                    info!("Loading scraper at: {}", &testpath);
                    finalpath = Some(testpath);
                    break 'extloop;
                } else {
                    warn!(
                        "Loading scraper at: {} FAILED due to path not existing",
                        &testpath
                    );
                    finalpath = None;
                }
            }
            if let Some(pathe) = &finalpath {
                let plugininfo: sharedtypes::PluginInfo;
                let lib;
                unsafe {
                    lib = libloading::Library::new(pathe).unwrap();
                    let plugindatafunc: libloading::Symbol<
                        unsafe extern "C" fn() -> sharedtypes::PluginInfo,
                    > = lib.get(b"return_info").unwrap();
                    plugininfo = plugindatafunc();
                }

                let pluginname = plugininfo.name.clone();

                info!(
                    "Loaded: {} With Description: {} Plugin Version: {} ABI: {}",
                    &pluginname,
                    &plugininfo.description,
                    &plugininfo.version,
                    &plugininfo.api_version
                );

                self._plugin.insert(pluginname.clone(), lib);

                for each in plugininfo.callbacks {
                    match self._callback.get_mut(&each) {
                        Some(vec_plugin) => {
                            vec_plugin.push(pluginname.clone());
                        }
                        None => {
                            self._callback.insert(each, vec![pluginname.clone()]);
                        }
                    }
                }
            }
            dbg!(&self._callback);
            dbg!(&finalpath);
            //let path = format!("{}{}lib{}.{}", &root, &pluginsloc, vec[vec.len() - 1], &libext);
        }
    }
}
