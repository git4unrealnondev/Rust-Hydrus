use crate::{
    logging,
    sharedtypes::{self, GlobalPluginScraper},
};
use libloading::Library;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, path::Path, thread};
use std::{path::PathBuf, thread::JoinHandle};

///
/// Runs the on_start callback
///
fn c_run_onstart(path: &Path) {
    let liba;
    unsafe {
        liba = Library::new(path).unwrap();
    }
    unsafe {
        let plugindatafunc: libloading::Symbol<unsafe extern "C" fn()> = match liba.get(b"on_start")
        {
            Ok(good) => good,
            Err(_) => {
                logging::log(&format!(
                    "Cannot find on_start for path: {}",
                    path.to_string_lossy()
                ));
                return;
            }
        };
        liba.get::<libloading::Symbol<unsafe extern "C" fn()>>(b"on_start")
            .unwrap();
        plugindatafunc();
    };
}

///
/// Calls a parser to cleave information from a lib
///
pub fn parser_call(
    url_output: &String,
    actual_params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
    globalload: Arc<Mutex<GlobalLoad>>,
    scraper: &GlobalPluginScraper,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let scrapermanager = globalload.lock().unwrap();
    if let Some(scraper_library) = scrapermanager.library_get(scraper) {
        let temp: libloading::Symbol<
            unsafe extern "C" fn(
                &String,
                &Vec<sharedtypes::ScraperParam>,
                &sharedtypes::ScraperData,
            )
                -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn>,
        > = unsafe { scraper_library.get(b"parser\0").unwrap() };
        unsafe { temp(url_output, actual_params, scraperdata) }
    } else {
        Err(sharedtypes::ScraperReturn::Nothing)
    }
}

pub struct GlobalLoad {
    callback: HashMap<sharedtypes::GlobalCallbacks, Vec<sharedtypes::GlobalPluginScraper>>,
    callback_cross: HashMap<sharedtypes::GlobalPluginScraper, Vec<sharedtypes::CallbackInfo>>,
    sites: HashMap<sharedtypes::GlobalPluginScraper, Vec<String>>,
    library_path: HashMap<sharedtypes::GlobalPluginScraper, PathBuf>,
    library_lib: HashMap<sharedtypes::GlobalPluginScraper, libloading::Library>,
    default_load: LoadableType,
    thread: HashMap<sharedtypes::GlobalPluginScraper, JoinHandle<()>>,
}

///
/// Determines what we should return from our get_loadable_paths function
///
enum LoadableType {
    Release,
    Debug,
}

impl GlobalLoad {
    pub fn new() -> Self {
        GlobalLoad {
            callback: HashMap::new(),
            callback_cross: HashMap::new(),
            sites: HashMap::new(),
            library_path: HashMap::new(),
            library_lib: HashMap::new(),
            default_load: LoadableType::Release,
            thread: HashMap::new(),
        }
    }
    pub fn external_plugin_call(
        &mut self,
        func_name: &String,
        vers: &usize,
        input_data: &sharedtypes::CallbackInfoInput,
    ) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
        if let Some(valid_func) = self.callbackstorage.get_mut(func_name) {
            for each in valid_func.iter() {
                if *vers == each.vers {
                    let plugin_lib = self.library_lib.get_mut(&each.name)?;
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
    /// Returns true if we have no threads that are running
    ///
    pub fn return_thread(&self) -> bool {
        self.thread.is_empty()
    }

    ///
    /// Returns a Library if it exists
    ///
    pub fn library_get(&self, global: &sharedtypes::GlobalPluginScraper) -> Option<&Library> {
        self.library_lib.get(global)
    }
    ///
    /// Closes any open threads that might still be open
    ///
    pub fn thread_finish_closed(&mut self) {
        let mut finished_threads = Vec::new();
        for (scraper, thread) in self.thread.iter() {
            if thread.is_finished() {
                finished_threads.push(scraper.clone());
            }
        }

        for thread in finished_threads.iter() {
            let th = self.thread.remove(thread).unwrap();
            let _ = th.join();
        }
    }

    ///
    /// Returns all loaded scrapers
    ///
    pub fn scraper_get(&self) -> Vec<sharedtypes::GlobalPluginScraper> {
        let mut out = Vec::new();
        for each in self.sites.keys() {
            out.push(each.clone());
        }
        out
    }

    ///
    /// Returns all sites for a scraper
    ///
    pub fn sites_get(&self, global: &sharedtypes::GlobalPluginScraper) -> Vec<String> {
        match self.sites.get(global) {
            None => Vec::new(),
            Some(out) => out.clone(),
        }
    }

    pub fn plugin_on_start(&mut self) {
        if let Some(plugin_list) = self.callback.get(&sharedtypes::GlobalCallbacks::Start) {
            for plugin in plugin_list {
                logging::log(&format!("Starting to run: {}", plugin.name));
                if !self.library_path.contains_key(plugin) {
                    logging::error_log(&format!(
                        "Skipping plugin: {} due to library reference not having it loaded?",
                        plugin.name
                    ));
                    continue;
                }
                if let Some(stored_info) = &plugin.storage_type {
                    match stored_info {
                        sharedtypes::ScraperOrPlugin::Plugin(plugin_info) => {
                            if plugin_info.com_channel {
                                let file = self.library_path.get(plugin).unwrap().clone();
                                let thread = thread::spawn(move || {
                                    c_run_onstart(&file);
                                });
                                self.thread.insert(plugin.clone(), thread);
                            }
                        }
                        sharedtypes::ScraperOrPlugin::Scraper(_) => {}
                    }
                }
            }
        }
    }

    ///
    /// Actually parses the Library
    /// TODO needs to make easy pulls for scraper and plugin info
    ///
    fn parse_lib(&mut self, lib: Library, path: &Path) {
        match self.get_info(&lib, path) {
            Some(items) => {
                if items.is_empty() {
                    logging::error_log(&format!(
                        "Was unable to pull any sites from: {}",
                        path.to_string_lossy()
                    ));
                    return;
                }
                for global in items {
                    if global.storage_type.is_none() {
                        logging::error_log(&format!(
                    "Skipping parsing of name: {} due to storage_type not being set.From {}",
                    global.name,
                    path.to_string_lossy()
                ));

                        continue;
                    }

                    for callbacks in global.callbacks {
                        match callbacks {
                            sharedtypes::GlobalCallbacks::Callback(callback_info) => {
                                self.callback_cross.insert(global.clone(), callback_info);
                            }
                            _ => match self.callback.get_mut(&callbacks) {
                                None => {
                                    let mut temp = vec![global];
                                    self.callback.insert(callbacks, temp);
                                }
                                Some(plugin_list) => {
                                    plugin_list.push(global.clone());
                                }
                            },
                        }
                    }

                    self.library_path.insert(global.clone(), path.to_path_buf());
                    let lib;
                    unsafe {
                        lib = libloading::Library::new(path).unwrap();
                    }
                    self.library_lib.insert(global, lib);
                }
            }
            None => {
                return;
            }
        }
    }

    fn get_info(
        &self,
        lib: &Library,
        path: &Path,
    ) -> Option<Vec<sharedtypes::GlobalPluginScraper>> {
        let temp: libloading::Symbol<
            unsafe extern "C" fn() -> Vec<sharedtypes::GlobalPluginScraper>,
        > = match unsafe { lib.get(b"get_global_info\0") } {
            Err(_) => {
                logging::error_log_silent(&format!(
                    "Could not run global info pull for lib. {}",
                    path.to_string_lossy()
                ));
                return None;
            }
            Ok(lib) => lib,
        };
        unsafe { Some(temp()) }
    }

    ///
    /// Gets a valid folder path and tries to load it into the library
    ///
    pub fn load_folder(&mut self, folder: &Path) {
        if !folder.exists() {
            let path_check = std::fs::create_dir_all(folder);
            match path_check {
                Ok(_) => (),
                Err(_) => {
                    logging::error_log(&format!(
                        "CANNOT CREATE FOLDER: {:?} DUE TO PERMISSIONS. STOPPING SEARCH",
                        folder.to_str()
                    ));
                    return;
                }
            }
        }
        if folder.is_file() {
            logging::error_log(&format!(
                "THIS IS A FILE DUM DUM. PATH: {:?}.... STOPPING SEARCH",
                folder.to_str()
            ));
            return;
        }
        let loadable_string = match self.default_load {
            LoadableType::Release => "release",
            LoadableType::Debug => "debug",
        };

        for entry in walkdir::WalkDir::new(folder)
            .max_depth(4)
            .into_iter()
            .flatten()
        {
            if entry.path().is_file() && entry.path().to_string_lossy().contains(loadable_string) {
                // Going to try and load hopefully valid library
                unsafe {
                    if let Ok(lib) = libloading::Library::new(entry.path()) {
                        self.parse_lib(lib, entry.path());
                    }
                }
            }
        }
    }
}
