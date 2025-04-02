use libloading::{self, Library};
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
//use std::sync::{mpsc, Arc, Mutex};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::{fs, thread};

use crate::database::{self, Main};
use crate::jobs::Jobs;
use crate::logging;
use crate::sharedtypes::{self, CallbackInfo};

use crate::server;

use regex::Regex;

///
/// Determines what we should return from our get_loadable_paths function
///
pub enum LoadableType {
    Release,
    Debug,
}

///
/// Gets valid paths for plugins
///
pub fn get_loadable_paths(pluginsloc: &String, loadable: &LoadableType) -> Vec<PathBuf> {
    let mut out = Vec::new();

    let ext = ["rlib", "so", "dylib", "dll"];
    let plugin_path = Path::new(pluginsloc);

    // Errors out if I cant create a folder
    if !plugin_path.exists() {
        let path_check = fs::create_dir_all(plugin_path);
        match path_check {
            Ok(_) => (),
            Err(_) => panic!(
                "{}",
                format!("CANNOT CREATE FOLDER: {} DUE TO PERMISSIONS.", &pluginsloc)
            ),
        }
    }

    for entry in walkdir::WalkDir::new(plugin_path).max_depth(4) {
        // Filters each entry if we're looking for release or debug libs
        if let Ok(entry) = entry {
            let loadable_string = match loadable {
                LoadableType::Release => "release",
                LoadableType::Debug => "debug",
            };
            if !entry.path().to_string_lossy().contains(loadable_string) {
                continue;
            }

            unsafe {
                if libloading::Library::new(entry.path()).is_ok() {
                    out.push(entry.path().to_path_buf());
                }
            }
        }
    }

    out
}

pub struct PluginManager {
    _plugin: HashMap<String, libloading::Library>,
    _plugin_coms: HashMap<String, Option<sharedtypes::PluginSharedData>>,
    _callback: HashMap<sharedtypes::PluginCallback, Vec<String>>,
    _database: Arc<Mutex<database::Main>>,
    _thread: HashMap<String, JoinHandle<()>>, // ONLY INSERT INTO ME FOR THE STARTING PLUGIN.
    _thread_path: HashMap<String, String>,    // Will be used for storing path of plugin name.
    _thread_data_share: HashMap<String, (os_pipe::PipeReader, os_pipe::PipeWriter)>,
    callbackstorage: HashMap<String, Vec<CallbackInfo>>,
    tagstorageregex: Vec<(
        Regex,
        String,
        Option<String>,
        Option<String>,
        sharedtypes::PluginCallback,
    )>,
}

///
/// Plugin Manager Handler
///
impl PluginManager {
    pub fn new(
        pluginsloc: String,
        main_db: Arc<Mutex<database::Main>>,
        jobs: Arc<Mutex<Jobs>>,
    ) -> Arc<Mutex<Self>> {
        let reftoself = Arc::new(Mutex::new(PluginManager {
            _plugin: HashMap::new(),
            _callback: HashMap::new(),
            _plugin_coms: HashMap::new(),
            _database: main_db.clone(),
            _thread: HashMap::new(),
            _thread_path: HashMap::new(),
            _thread_data_share: HashMap::new(),
            callbackstorage: HashMap::new(),
            tagstorageregex: Vec::new(),
        }));

        reftoself.lock().unwrap().load_plugins(&pluginsloc);

        // Needed for thread move because it moves the value
        let threa = reftoself.clone();

        logging::log(&"Starting IPC Server.".to_string());
        let srv = std::thread::spawn(move || {
            let mut ipc_coms = server::PluginIpcInteract::new(main_db.clone(), threa, jobs);
            //let _ = rcv.recv();

            //println!("v");
            ipc_coms.spawn_listener()
        });
        reftoself
    }

    ///
    /// Debug info for plugins
    ///
    pub fn debug(&self) {
        dbg!(&self._plugin);
        dbg!(&self._callback);
        dbg!(&self._plugin_coms);
        dbg!(&self._thread);
        dbg!(&self._thread_path);
        dbg!(&self._thread_data_share);
        dbg!(&self.callbackstorage);
    }

    ///
    /// Manages callings to external plugins.
    /// Allows cross communication between plugins.
    ///
    pub fn external_plugin_call(
        &mut self,
        func_name: &String,
        vers: &usize,
        input_data: &sharedtypes::CallbackInfoInput,
    ) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
        if let Some(valid_func) = self.callbackstorage.get_mut(func_name) {
            for each in valid_func.iter() {
                if *vers == each.vers {
                    let plugin_lib = self._plugin.get_mut(&each.name)?;
                    let plugininfo;
                    unsafe {
                        let plugindatafunc: libloading::Symbol<
                            unsafe extern "C" fn(
                                &sharedtypes::CallbackInfoInput,
                            ) -> Option<
                                HashMap<String, sharedtypes::CallbackCustomDataReturning>,
                            >,
                        > = plugin_lib.get(func_name.as_bytes()).unwrap();
                        plugininfo = plugindatafunc(input_data);
                    }
                    return plugininfo;
                }
            }
        }
        None
    }

    ///
    /// Returns if the thread manager have finished.
    /// Doesn't check if the threads have actually finished.
    ///
    pub fn return_thread(&self) -> bool {
        self._thread.is_empty()
    }

    ///
    /// Closes any threads in self._threads that have finished.
    ///
    pub fn thread_finish_closed(&mut self) {
        let mut finished_threads: Vec<String> = Vec::new();
        let thlist = self._thread.keys();
        for thread in thlist {
            if self._thread.get(thread).unwrap().is_finished() {
                finished_threads.push(thread.to_string());
            }
        }
        for thread in finished_threads {
            let th = self._thread.remove(&thread).unwrap();
            let _ = th.join();
        }
    }

    ///
    /// Loads plugins into plugin manager
    ///
    fn load_plugins(&mut self, pluginsloc: &String) {
        logging::log(&format!("Starting to load plugins at: {}", pluginsloc));

        for plugin_path in get_loadable_paths(pluginsloc, &LoadableType::Release).iter() {
            logging::log(&format!("Loading plugin at: {}", plugin_path.display()));
            self.load_plugin_from_path(plugin_path);
        }
    }

    ///
    /// Manages loading the plugin
    ///
    fn load_plugin_from_path(&mut self, path: &Path) {
        let plugininfo: sharedtypes::PluginInfo;
        let lib;

        if !path.exists() {
            return;
        }

        unsafe {
            lib = libloading::Library::new(path).unwrap();
            let plugindatafunc: libloading::Symbol<
                unsafe extern "C" fn() -> sharedtypes::PluginInfo,
            > = lib.get(b"return_info").unwrap();
            plugininfo = plugindatafunc();
        }

        let pluginname = plugininfo.name.clone();

        logging::log(&format!(
            "Loaded: {} With Description: {} Plugin Version: {} ABI: {} Comms: {:?}",
            &pluginname,
            &plugininfo.description,
            &plugininfo.version,
            &plugininfo.api_version,
            &plugininfo.communication,
        ));

        self._plugin_coms
            .insert(pluginname.clone(), plugininfo.communication);

        self._plugin.insert(pluginname.clone(), lib);
        self._thread_path
            .insert(pluginname.clone(), path.to_string_lossy().to_string());

        for each in plugininfo.callbacks {
            if std::mem::discriminant(&each)
                == std::mem::discriminant(&sharedtypes::PluginCallback::Tag(Vec::new()))
            {
                if let sharedtypes::PluginCallback::Tag(ontaginfo) = &each {
                    for (search_type, namespace, not_namespace) in ontaginfo {
                        if let Some(search) = search_type {
                            if let sharedtypes::SearchType::Regex(regexstring) = search {
                                let regexresult = Regex::new(regexstring);
                                if let Ok(regex) = regexresult {
                                    self.tagstorageregex.push((
                                        regex,
                                        pluginname.clone(),
                                        namespace.clone(),
                                        not_namespace.clone(),
                                        each.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            match self._callback.get_mut(&each) {
                Some(vec_plugin) => {
                    vec_plugin.push(pluginname.clone());
                }
                None => match each {
                    sharedtypes::PluginCallback::Callback(plugininfo) => {
                        match self.callbackstorage.get_mut(&plugininfo.func) {
                            Some(callvec) => {
                                callvec.push(plugininfo);
                            }
                            None => {
                                self.callbackstorage
                                    .insert(plugininfo.func.clone(), [plugininfo].to_vec());
                            }
                        }
                    }
                    _ => {
                        self._callback.insert(each, vec![pluginname.clone()]);
                    }
                },
            }
        }
    }

    ///
    /// Runs the callback on startup
    ///
    pub fn plugin_on_start(&mut self) {
        // IF theirs no functions with this callback registered then return
        if !self
            ._callback
            .contains_key(&sharedtypes::PluginCallback::Start)
        {
            return;
        }
        // Gets all callbacks related to a callback and checks if the plugin
        for plugin in self._callback[&sharedtypes::PluginCallback::Start].clone() {
            info!("Starting to run plugin: {}", &plugin);
            if !self._plugin.contains_key(&plugin) {
                error!("Could not call Plugin-OnStart");
                continue;
            }

            // Does a check to see if we need to determine how to pass data to and fro
            match &self._plugin_coms[&plugin] {
                None => {
                    let runloc = self._thread_path[&plugin].to_string();

                    c_run_onstart(&runloc);
                }
                Some(plugincoms) => {
                    match &plugincoms.com_channel {
                        None => {
                            // Starts plugin inline while will wait for it to finish.
                            let runloc = self._thread_path[&plugin].to_string();

                            c_run_onstart(&runloc);
                        }
                        Some(pluginchannel) => {
                            match pluginchannel {
                                sharedtypes::PluginCommunicationChannel::None => {}
                                sharedtypes::PluginCommunicationChannel::Pipe(pipe) => {
                                    // Have to do this wanky ness to allow me to spawn a thread that outlives the &mut self
                                    // Spawns the function in a seperate thread.
                                    // Have to get this outside of the thread spawn for
                                    // compatibility reasons with the calling funciton.
                                    let runloc = self._thread_path[&plugin].to_string();

                                    let thread = thread::spawn(move || {
                                        c_run_onstart(&runloc);
                                    });
                                    self._thread.insert(plugin.to_string(), thread);
                                }
                            }
                        }
                    }
                    continue;
                }
            }
        }
    }

    ///
    /// Reloads the plugins that are currently loaded
    ///
    pub fn reload_loaded_plugins(&mut self) {
        dbg!("Reload Loaded Plugins");
        let mut pluginlist = Vec::new();
        for threadname in self._plugin.keys() {
            if let Some(threadpath) = self._thread_path.get(threadname) {
                if Path::new(threadpath).exists() {
                    pluginlist.push((threadname.clone(), threadpath.clone()));
                }
            }
        }

        self._plugin_coms.clear();
        self._plugin.clear();
        self._thread_path.clear();
        self._callback.clear();
        self._thread_data_share.clear();
        self.callbackstorage.clear();
        for plugin in pluginlist {
            logging::log(&format!(
                "Reloading plugin: {} at path {}.",
                plugin.0, plugin.1
            ));

            self.load_plugin_from_path(Path::new(&plugin.1));
        }
    }
}
///
/// Parses output from plugin.
///
fn parse_plugin_output(
    plugin_data: Vec<sharedtypes::DBPluginOutputEnum>,
    unwrappy_locked: Arc<Mutex<Main>>,
) {
    let mut unwrappy = unwrappy_locked.lock().unwrap();
    //let mut unwrappy = self._database.lock().unwrap();

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
                            // dbg!(&namespace);
                            namespace_id = Some(unwrappy.namespace_add(
                                namespace.name,
                                namespace.description,
                                true,
                            ));
                        }
                    }
                    if let Some(temp) = names.file {
                        for files in temp {
                            if files.id.is_none() && files.hash.is_some() && files.ext.is_some() {
                                // Gets the extension id
                                let ext_id = unwrappy.extension_put_string(&files.ext.unwrap());

                                let storage_id = match files.location {
                                    Some(exists) => {
                                        unwrappy.storage_put(&exists);
                                        unwrappy.storage_get_id(&exists).unwrap()
                                    }
                                    None => {
                                        let exists = unwrappy.location_get();
                                        unwrappy.storage_put(&exists);
                                        unwrappy.storage_get_id(&exists).unwrap()
                                    }
                                };

                                let file = sharedtypes::DbFileStorage::NoIdExist(
                                    sharedtypes::DbFileObjNoId {
                                        hash: files.hash.unwrap(),
                                        ext_id,
                                        storage_id,
                                    },
                                );
                                unwrappy.file_add(file, true);
                            }
                        }
                    }
                    if let Some(temp) = names.tag {
                        for tags in temp {
                            let namespace_id = unwrappy.namespace_get(&tags.namespace).cloned();
                            //match namespace_id {}
                            //dbg!(&tags);
                            if tags.parents.is_none() && namespace_id.is_some() {
                                unwrappy.tag_add(&tags.name, namespace_id.unwrap(), true, None);
                            //                                    println!("plugins323 making tag: {}", tags.name);
                            } else {
                                for _parents_obj in tags.parents.unwrap() {
                                    unwrappy.tag_add(&tags.name, namespace_id.unwrap(), true, None);
                                }
                            }
                        }
                    }
                    if let Some(temp) = names.setting {
                        for settings in temp {
                            unwrappy.setting_add(
                                settings.name,
                                settings.pretty,
                                settings.num,
                                settings.param,
                                true,
                            );
                        }
                    }

                    if let Some(_temp) = names.jobs {}
                    if let Some(temp) = names.relationship {
                        for relations in temp {
                            let file_id = unwrappy.file_get_hash(&relations.file_hash).cloned();
                            let namespace_id = unwrappy.namespace_get(&relations.tag_namespace);
                            let tag_id = unwrappy
                                .tag_get_name(relations.tag_name.clone(), *namespace_id.unwrap())
                                .cloned();
                            /*println!(
                                "plugins356 relating: file id {:?} to {:?}",
                                file_id, relations.tag_name
                            );*/
                            unwrappy.relationship_add(file_id.unwrap(), tag_id.unwrap(), true);
                            //unwrappy.relationship_add(file, tag, addtodb)
                        }
                    }
                    if let Some(_temp) = names.parents {}
                }
            }
            sharedtypes::DBPluginOutputEnum::Del(name) => for _names in name {},
            sharedtypes::DBPluginOutputEnum::None => {}
        }
    }
}

///
/// Runs when we have a login needed
///
pub fn plugin_on_loginneeded(
    manager_arc: Arc<Mutex<PluginManager>>,
    login: &sharedtypes::LoginType,
) -> Vec<sharedtypes::LoginType> {
    let plugins_to_run;
    {
        let manager = manager_arc.lock().unwrap();
        plugins_to_run = manager
            ._callback
            .get(&sharedtypes::PluginCallback::LoginNeeded)
            .unwrap_or(&Vec::new())
            .clone();
    }

    let mut output = Vec::new();
    for plugin_name in plugins_to_run {
        let lib;
        unsafe {
            let mgr = manager_arc.lock().unwrap();
            lib = libloading::Library::new(mgr._thread_path.get(&plugin_name.clone()).unwrap())
                .unwrap();
        }
        unsafe {
            let plugindatafunc: libloading::Symbol<
                unsafe extern "C" fn(&sharedtypes::LoginType) -> Option<sharedtypes::LoginType>,
                //unsafe extern "C" fn(Cursor<Bytes>, &String, &String, Arc<Mutex<database::Main>>),
            > = match lib.get(b"on_loginneeded") {
                Ok(lib) => lib,
                Err(_) => {
                    logging::info_log(&format!(
                        "Could not find on_loginneeded for plugin: {}",
                        plugin_name
                    ));
                    continue;
                }
            };
            //unwrappy.
            if let Some(out) = plugindatafunc(login) {
                output.push(out);
            }
        }
        lib.close();
    }
    output
}

///
/// Threadsafe way to call callback on adding a tag into the db
///
pub fn plugin_on_tag(
    manager_arc: Arc<Mutex<PluginManager>>,
    db: &mut Main,
    tag: &String,
    tag_nsid: &usize,
) {
    let tagstorageregex;
    {
        let temp = manager_arc.lock().unwrap();
        tagstorageregex = temp.tagstorageregex.clone();
    }

    for (regex, plugin_name, namespace, not_namespace, plugin_callback) in tagstorageregex.iter() {
        if let Some(not_namespace) = not_namespace {
            if let Some(nsid) = db.namespace_get(not_namespace) {
                if tag_nsid == nsid {
                    return;
                }
            }
        }

        let regex_iter: Vec<&str> = regex.find_iter(tag).map(|m| m.as_str()).collect();
        for regex_match in regex_iter {
            let tag_ns = match db.namespace_get_string(tag_nsid) {
                None => continue,
                Some(namespace_name) => &namespace_name.name,
            };
            c_regex_match(
                manager_arc.clone(),
                plugin_name,
                tag,
                tag_ns,
                &regex_match.to_string(),
                plugin_callback.clone(),
            );
        }
    }
}

///
/// Hopefully a thread-safe way to call plugins per thread avoiding a lock.
///
pub fn plugin_on_download(
    manager_arc: Arc<Mutex<PluginManager>>,
    db: Arc<Mutex<Main>>,
    cursorpass: &[u8],
    hash: &String,
    ext: &String,
) {
    let callback;

    {
        let temp = manager_arc.lock().unwrap();
        callback = temp._callback.clone();
    }

    if !callback.contains_key(&sharedtypes::PluginCallback::Download) {
        return;
    }

    for plugin in callback[&sharedtypes::PluginCallback::Download].clone() {
        if !manager_arc.lock().unwrap()._plugin.contains_key(&plugin) {
            error!("Could not call Plugin-OnDownload");
            continue;
        }
        let lib;
        unsafe {
            let mgr = manager_arc.lock().unwrap();
            lib = libloading::Library::new(mgr._thread_path.get(&plugin).unwrap()).unwrap();
        }
        let output;
        unsafe {
            let plugindatafunc: libloading::Symbol<
                unsafe extern "C" fn(
                    &[u8],
                    &String,
                    &String,
                ) -> Vec<sharedtypes::DBPluginOutputEnum>,
                //unsafe extern "C" fn(Cursor<Bytes>, &String, &String, Arc<Mutex<database::Main>>),
            > = match lib.get(b"on_download") {
                Ok(lib) => lib,
                Err(_) => {
                    logging::info_log(&format!(
                        "Could not find on_download for plugin: {}",
                        plugin
                    ));
                    continue;
                }
            };
            //unwrappy.
            output = plugindatafunc(cursorpass, hash, ext);
        }
        parse_plugin_output(output, db.clone());
        lib.close();
    }
}

///
/// Starts running the onstart plugin.
/// Should only be called from a pluginmanager instance.
/// I'm lazy so this is the easiest way to make it worky.
///
fn c_run_onstart(path: &String) {
    let liba;
    unsafe {
        liba = Library::new(path).unwrap();
    }
    unsafe {
        let plugindatafunc: libloading::Symbol<unsafe extern "C" fn()> = match liba.get(b"on_start")
        {
            Ok(good) => good,
            Err(_) => {
                logging::log(&format!("Cannot find on_start for path: {}", path));
                return;
            }
        };
        liba.get::<libloading::Symbol<unsafe extern "C" fn()>>(b"on_start")
            .unwrap();
        plugindatafunc();
    };
}

fn c_regex_match(
    manager_arc: Arc<Mutex<PluginManager>>,
    lib_name: &String,
    tag: &String,
    tag_ns: &String,
    regex_match: &String,
    plugin_callback: sharedtypes::PluginCallback,
) {
    let pluginmanager = manager_arc.lock().unwrap();
    let liba;
    {
        match pluginmanager._plugin.get(lib_name) {
            None => {
                return;
            }
            Some(lib) => {
                liba = lib;
            }
        }
    }

    unsafe {
        let plugindatafunc: libloading::Symbol<
            unsafe extern "C" fn(&String, &String, &String, sharedtypes::PluginCallback),
        > = match liba.get(b"on_regex_match") {
            Ok(good) => good,
            Err(_) => {
                return;
            }
        };
        liba.get::<libloading::Symbol<
            unsafe extern "C" fn(&String, &String, &String, sharedtypes::PluginCallback),
        >>(b"on_regex_match")
            .unwrap();
        plugindatafunc(tag, tag_ns, regex_match, plugin_callback);
    };
}
