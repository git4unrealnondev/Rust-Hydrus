use crate::{
    database::{self, Main},
    jobs::{self, Jobs},
    logging, server,
    sharedtypes::{self, GlobalPluginScraper},
};
use libloading::Library;
use regex::Regex;
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

///
/// Calls a regex function
///
fn c_regex_match(
    db: &mut Main,
    tag: &String,
    tag_ns: &String,
    regex_match: &String,
    plugin_callback: &Option<sharedtypes::SearchType>,
    liba: &libloading::Library,
    scraper: &GlobalPluginScraper,
    jobmanager: Arc<Mutex<Jobs>>,
) {
    dbg!(tag, tag_ns, regex_match);
    let output;
    unsafe {
        let plugindatafunc: libloading::Symbol<
            unsafe extern "C" fn(
                &String,
                &String,
                &String,
                &Option<sharedtypes::SearchType>,
            ) -> Vec<sharedtypes::DBPluginOutputEnum>,
        > = match liba.get(b"on_regex_match") {
            Ok(good) => good,
            Err(_) => {
                return;
            }
        };
        liba.get::<libloading::Symbol<
            unsafe extern "C" fn(
                &String,
                &String,
                &String,
                &Option<sharedtypes::SearchType>,
            ) -> Vec<sharedtypes::DBPluginOutputEnum>,
        >>(b"on_regex_match")
            .unwrap();
        output = plugindatafunc(tag, tag_ns, regex_match, plugin_callback);
    };
    parse_plugin_output_andmain(output, db, scraper, jobmanager);
}

///
/// Gets a scraper to output any URLs based on params
///
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
    arc_scrapermanager: Arc<Mutex<GlobalLoad>>,
    scraper: &sharedtypes::GlobalPluginScraper,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let mut out = Vec::new();
    let mut libstorage = Vec::new();

    // Loads the valid libraries
    let globalload = arc_scrapermanager.lock().unwrap();
    for loaded_scraper in globalload.scraper_get().iter() {
        if scraper == loaded_scraper {
            if let Some(lib) = globalload.library_get(loaded_scraper) {
                libstorage.push(lib);
            }
        }
    }

    for lib in libstorage {
        let temp: libloading::Symbol<
            unsafe extern "C" fn(
                &Vec<sharedtypes::ScraperParam>,
                &sharedtypes::ScraperData,
            ) -> Vec<(String, sharedtypes::ScraperData)>,
        > = unsafe { lib.get(b"url_dump\0").unwrap() };
        for item in unsafe { temp(params, scraperdata) } {
            out.push(item);
        }
    }

    /*let scrapermanager = arc_scrapermanager.lock().unwrap();
    let scraper_library = scrapermanager.library_get().get(scraper).unwrap();
    let temp: libloading::Symbol<
        unsafe extern "C" fn(
            &Vec<sharedtypes::ScraperParam>,
            &sharedtypes::ScraperData,
        ) -> Vec<(String, sharedtypes::ScraperData)>,
    > = unsafe { scraper_library.get(b"url_dump\0").unwrap() };
    unsafe { temp(params, scraperdata) }*/

    out
}

///
/// Parses output from plugin.
///
fn parse_plugin_output(
    plugin_data: Vec<sharedtypes::DBPluginOutputEnum>,
    unwrappy_locked: Arc<Mutex<Main>>,
    scraper: &GlobalPluginScraper,
    jobmanager: Arc<Mutex<Jobs>>,
) {
    let mut unwrappy = unwrappy_locked.lock().unwrap();
    //let mut unwrappy = self._database.lock().unwrap();
    parse_plugin_output_andmain(plugin_data, &mut unwrappy, scraper, jobmanager);
}

///
/// Hopefully a thread-safe way to call plugins per thread avoiding a lock.
///
pub fn plugin_on_download(
    manager_arc: Arc<Mutex<GlobalLoad>>,
    db: Arc<Mutex<Main>>,
    cursorpass: &[u8],
    hash: &String,
    ext: &String,
) {
    let (libpath, libscraper);
    {
        let manager = manager_arc.lock().unwrap();
        (libpath, libscraper) = (
            manager.get_lib_path_from_callback(&sharedtypes::GlobalCallbacks::Download),
            manager.get_scrapers_from_callback(&sharedtypes::GlobalCallbacks::Download),
        );
    }

    for (cnt, lib_path) in libpath.iter().enumerate() {
        let lib;
        unsafe {
            match libloading::Library::new(&lib_path) {
                Ok(good_lib) => lib = good_lib,
                Err(_) => {
                    logging::error_log(&format!(
                        "Cannot load library at path: {}",
                        lib_path.to_string_lossy()
                    ));
                    continue;
                }
            }
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
                        lib_path.to_string_lossy()
                    ));
                    continue;
                }
            };
            //unwrappy.
            output = plugindatafunc(cursorpass, hash, ext);
        }

        let mut jobmanager;
        {
            let mut manager = manager_arc.lock().unwrap();
            jobmanager = manager.jobmanager.clone();
        }
        parse_plugin_output(output, db.clone(), libscraper.get(cnt).unwrap(), jobmanager);
        lib.close();
    }
}

///
/// Gets called onstartup of the software
///
pub fn on_start(libloading: &libloading::Library, site_struct: &sharedtypes::GlobalPluginScraper) {
    let temp: libloading::Symbol<unsafe extern "C" fn(&sharedtypes::GlobalPluginScraper)> =
        match unsafe { libloading.get(b"on_start\0") } {
            Err(_) => {
                logging::error_log_silent(&format!(
                    "Cannot run on_start for name: {}",
                    site_struct.name
                ));
                return;
            }
            Ok(lib) => lib,
        };

    logging::log(&format!(
        "Running on_start call for name: {}",
        site_struct.name
    ));
    unsafe { temp(site_struct) }
}

///
/// This function gets called after a DB upgrade
///
pub fn db_upgrade_call(
    libloading: &libloading::Library,
    db_version: &usize,
    site_struct: &sharedtypes::GlobalPluginScraper,
) {
    let temp: libloading::Symbol<unsafe extern "C" fn(&usize, &sharedtypes::GlobalPluginScraper)> =
        match unsafe { libloading.get(b"db_upgrade_call\0") } {
            Err(err) => {
                logging::error_log(&format!(
                    "Could not run scraper upgrade for db version {} because of {}.",
                    db_version, err
                ));
                return;
            }
            Ok(lib) => lib,
        };

    unsafe { temp(db_version, site_struct) }
}

///
/// Threadsafe way to call callback on adding a tag into the db
///
pub fn plugin_on_tag(
    manager_arc: Arc<Mutex<GlobalLoad>>,
    db: &mut Main,
    tag: &String,
    tag_nsid: &usize,
) {
    let tagstorageregex;
    {
        let temp = manager_arc.lock().unwrap();
        tagstorageregex = temp.regex_storage.clone();
    }

    let mut storagemap = Vec::new();

    'searchloop: for (((search_string, search_regex), ns, not_ns), pluginscraper_list) in
        tagstorageregex.iter()
    {
        // Filtering for weather we should apply this to a tag of X namespace
        for ns in ns {
            if ns != tag_nsid {
                continue 'searchloop;
            }
        }

        for not_ns in not_ns {
            if not_ns == tag_nsid {
                continue 'searchloop;
            }
        }

        // Actual matching going on here
        if let Some(search) = search_string {
            if tag.contains(search) {}
        } else if let Some(regex) = search_regex {
            let regex_iter: Vec<&str> = regex.0.find_iter(tag).map(|m| m.as_str()).collect();
            for regexmatch in regex_iter {
                for pluginscraper in pluginscraper_list {
                    storagemap.push((
                        pluginscraper,
                        regexmatch,
                        Some(sharedtypes::SearchType::Regex(regex.0.to_string())),
                    ));
                }
            }
        } else {
            for pluginscraper in pluginscraper_list {
                storagemap.push((pluginscraper, &"", None));
            }
        }
    }

    for (pluginscraper, regex, searchtype) in storagemap {
        let tag_ns;
        {
            match db.namespace_get_string(tag_nsid) {
                None => {
                    continue;
                }
                Some(nso) => {
                    tag_ns = nso.name.clone();
                }
            }
        }

        let jobmanager;
        let liba;
        {
            let temp = manager_arc.lock().unwrap();
            jobmanager = temp.jobmanager.clone();
            match temp.library_get_path(pluginscraper) {
                None => {
                    liba = None;
                }
                Some(libpath) => {
                    liba = Some(unsafe { libloading::Library::new(libpath).unwrap() });
                }
            }
        }
        if let Some(liba) = liba {
            c_regex_match(
                db,
                tag,
                &tag_ns,
                &regex.to_string(),
                &searchtype,
                &liba,
                pluginscraper,
                jobmanager,
            );
        }
    }
}
fn parse_plugin_output_andmain(
    plugin_data: Vec<sharedtypes::DBPluginOutputEnum>,
    db: &mut Main,
    scraper: &GlobalPluginScraper,
    jobmanager: Arc<Mutex<Jobs>>,
) {
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
                            namespace_id =
                                Some(db.namespace_add(namespace.name, namespace.description, true));
                        }
                    }
                    if let Some(temp) = names.file {
                        for files in temp {
                            if files.id.is_none() && files.hash.is_some() && files.ext.is_some() {
                                // Gets the extension id
                                let ext_id = db.extension_put_string(&files.ext.unwrap());

                                let storage_id = match files.location {
                                    Some(exists) => {
                                        db.storage_put(&exists);
                                        db.storage_get_id(&exists).unwrap()
                                    }
                                    None => {
                                        let exists = db.location_get();
                                        db.storage_put(&exists);
                                        db.storage_get_id(&exists).unwrap()
                                    }
                                };

                                let file = sharedtypes::DbFileStorage::NoIdExist(
                                    sharedtypes::DbFileObjNoId {
                                        hash: files.hash.unwrap(),
                                        ext_id,
                                        storage_id,
                                    },
                                );
                                db.file_add(file, true);
                            }
                        }
                    }
                    if let Some(temp) = names.tag {
                        for tags in temp {
                            let namespace_id = db.namespace_get(&tags.namespace).cloned();
                            //match namespace_id {}
                            //dbg!(&tags);
                            if tags.parents.is_none() && namespace_id.is_some() {
                                db.tag_add(&tags.name, namespace_id.unwrap(), true, None);
                            //                                    println!("plugins323 making tag: {}", tags.name);
                            } else {
                                for _parents_obj in tags.parents.unwrap() {
                                    db.tag_add(&tags.name, namespace_id.unwrap(), true, None);
                                }
                            }
                        }
                    }
                    if let Some(temp) = names.setting {
                        for settings in temp {
                            db.setting_add(
                                settings.name,
                                settings.pretty,
                                settings.num,
                                settings.param,
                                true,
                            );
                        }
                    }

                    if let Some(temp) = names.jobs {
                        for job in temp {
                            db.jobs_add(
                                None,
                                job.time,
                                job.reptime.unwrap_or(0),
                                job.site,
                                job.param,
                                job.committype
                                    .unwrap_or(sharedtypes::CommitType::StopOnNothing),
                                job.system_data,
                                job.user_data,
                                job.jobmanager,
                            );
                        }
                    }
                    if let Some(temp) = names.relationship {
                        for relations in temp {
                            let file_id = db.file_get_hash(&relations.file_hash).cloned();
                            let namespace_id = db.namespace_get(&relations.tag_namespace);
                            let tag_id = db
                                .tag_get_name(relations.tag_name.clone(), *namespace_id.unwrap())
                                .cloned();
                            /*println!(
                                "plugins356 relating: file id {:?} to {:?}",
                                file_id, relations.tag_name
                            );*/
                            db.relationship_add(file_id.unwrap(), tag_id.unwrap(), true);
                            //unwrappy.relationship_add(file, tag, addtodb)
                        }
                    }
                    if let Some(temp) = names.parents {
                        for parent in temp {}
                    }
                }
            }
            sharedtypes::DBPluginOutputEnum::Del(name) => for _names in name {},
            sharedtypes::DBPluginOutputEnum::None => {}
        }
    }
}

pub struct GlobalLoad {
    db: Arc<Mutex<Main>>,
    callback: HashMap<sharedtypes::GlobalCallbacks, Vec<sharedtypes::GlobalPluginScraper>>,
    callback_cross: HashMap<sharedtypes::GlobalPluginScraper, Vec<sharedtypes::CallbackInfo>>,
    sites: HashMap<sharedtypes::GlobalPluginScraper, Vec<String>>,
    library_path: HashMap<sharedtypes::GlobalPluginScraper, PathBuf>,
    library_lib: HashMap<sharedtypes::GlobalPluginScraper, libloading::Library>,
    default_load: LoadableType,
    thread: HashMap<sharedtypes::GlobalPluginScraper, JoinHandle<()>>,
    ipc_server: Option<JoinHandle<()>>,
    regex_storage: HashMap<
        (
            (Option<String>, Option<sharedtypes::RegexStorage>),
            Vec<usize>,
            Vec<usize>,
        ),
        Vec<sharedtypes::GlobalPluginScraper>,
    >,
    jobmanager: Arc<Mutex<Jobs>>,
}

///
/// Determines what we should return from our get_loadable_paths function
///
enum LoadableType {
    Release,
    Debug,
}

impl GlobalLoad {
    pub fn new(db: Arc<Mutex<database::Main>>, jobs: Arc<Mutex<Jobs>>) -> Arc<Mutex<Self>> {
        logging::log(&"Starting IPC Server.".to_string());

        Arc::new(Mutex::new(GlobalLoad {
            db,
            callback: HashMap::new(),
            callback_cross: HashMap::new(),
            sites: HashMap::new(),
            library_path: HashMap::new(),
            library_lib: HashMap::new(),
            default_load: LoadableType::Release,
            thread: HashMap::new(),
            ipc_server: None,
            regex_storage: HashMap::new(),
            jobmanager: jobs,
        }))
    }

    pub fn setup_ipc(
        &mut self,
        globalload: Arc<Mutex<GlobalLoad>>,
        db: Arc<Mutex<Main>>,
        jobs: Arc<Mutex<Jobs>>,
    ) {
        let globalload = globalload.clone();
        let srv = std::thread::spawn(move || {
            let mut ipc_coms = server::PluginIpcInteract::new(db.clone(), globalload.clone(), jobs);
            //let _ = rcv.recv();

            //println!("v");
            match ipc_coms.spawn_listener() {
                Ok(out) => out,
                Err(err) => {
                    logging::panic_log(&format!("Failed to spawn IPC Server"));
                }
            }
        });

        self.ipc_server = Some(srv);
    }

    ///
    /// Debug function for development
    ///
    pub fn debug(&self) {
        dbg!(
            &self.callback,
            &self.sites,
            &self.library_path,
            &self.regex_storage
        );
    }

    ///
    ///
    ///
    fn get_scrapers_from_callback(
        &self,
        callback: &sharedtypes::GlobalCallbacks,
    ) -> Vec<GlobalPluginScraper> {
        if let Some(callbacklist) = self.callback.get(callback) {
            callbacklist.clone()
        } else {
            Vec::new()
        }
    }

    ///
    /// Gets a library path from each valid callback
    ///
    pub fn get_lib_path_from_callback(
        &self,
        callback: &sharedtypes::GlobalCallbacks,
    ) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(callbacklist) = self.callback.get(callback) {
            for callback_item in callbacklist {
                if let Some(libp) = self.library_path.get(callback_item) {
                    out.push(libp.clone());
                }
            }
        }
        out
    }

    /*///
    /// Returns a tag callback list based on limitations passed in by the end search
    ///
    pub fn get_tag_callback(
        &self,
        searchtype: Option<sharedtypes::SearchType>,
        namespace: Option<String>,
        not_namespace: Option<String>,
    ) -> Vec<sharedtypes::GlobalPluginScraper> {
        let mut out = Vec::new();
        for (callback, pluginscrapers) in self.callback.iter() {
            if let sharedtypes::GlobalCallbacks::Tag((st, ns, not_ns)) = callback {
                if let Some(searchtype) = &searchtype {
                    if let Some(st) = st {
                        if st != searchtype {
                            continue;
                        }
                    }
                }
                if let Some(namespace) = &namespace {
                    for ns in ns {
                        if ns != namespace {
                            continue;
                        }
                    }
                }
                if let Some(not_namespace) = &not_namespace {
                    for not_ns in not_ns {
                        if not_ns != not_namespace {
                            continue;
                        }
                    }
                }

                for each in pluginscrapers {
                    out.push(each.clone());
                }
            }
        }

        out
    }*/

    ///
    /// Calls a plugin from another plugin
    ///
    pub fn external_plugin_call(
        &mut self,
        func_name: &String,
        vers: &usize,
        input_data: &sharedtypes::CallbackInfoInput,
    ) -> Option<HashMap<String, sharedtypes::CallbackCustomDataReturning>> {
        /*if let Some(valid_func) = self.callbackstorage.get_mut(func_name) {
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
        }*/
        todo!();
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
    /// Returns a library path if it exists
    ///
    pub fn library_get_path(&self, global: &sharedtypes::GlobalPluginScraper) -> Option<&PathBuf> {
        self.library_path.get(global)
    }

    ///
    /// Returns the libraries raw
    ///
    pub fn library_get_raw(&self) -> &HashMap<GlobalPluginScraper, Library> {
        &self.library_lib
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

    ///
    /// Triggers the on_start for the plugins
    ///
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
    /// Clears the regex cache and callbacks
    ///
    fn clear_regex(&mut self) {
        self.callback.clear();
        self.callback_cross.clear();
        self.regex_storage.clear();
    }

    ///
    /// Reloads the regex stores
    ///
    pub fn reload_regex(&mut self) {
        self.clear_regex();
        {
            let scraper_folder;
            let plugin_folder;
            {
                let mut unwrappy = self.db.lock().unwrap();
                scraper_folder = unwrappy.loaded_scraper_folder();
                plugin_folder = unwrappy.loaded_plugin_folder();
            }
            self.load_folder(&scraper_folder);
            self.load_folder(&plugin_folder);
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
                    match global.storage_type {
                        None => {
                            logging::error_log(&format!(
                    "Skipping parsing of name: {} due to storage_type not being set.From {}",
                    global.name,
                    path.to_string_lossy()
                ));

                            continue;
                        }
                        Some(ref scraperplugin) => match scraperplugin {
                            sharedtypes::ScraperOrPlugin::Scraper(scraperinfo) => {
                                self.sites.insert(global.clone(), scraperinfo.sites.clone());
                            }
                            sharedtypes::ScraperOrPlugin::Plugin(plugininfo) => {}
                        },
                    }

                    for callbacks in global.callbacks.iter() {
                        match callbacks {
                            sharedtypes::GlobalCallbacks::Callback(callback_info) => {
                                match self.callback_cross.get_mut(&global) {
                                    None => {
                                        self.callback_cross
                                            .insert(global.clone(), vec![callback_info.clone()]);
                                    }
                                    Some(list) => {
                                        list.push(callback_info.clone());
                                    }
                                }
                            }
                            _ => match self.callback.get_mut(&callbacks) {
                                None => {
                                    let temp = vec![global.clone()];
                                    self.callback.insert(callbacks.clone(), temp);
                                }
                                Some(plugin_list) => {
                                    plugin_list.push(global.clone());
                                }
                            },
                        }

                        if let sharedtypes::GlobalCallbacks::Tag((searchtype, ns, not_ns)) =
                            callbacks
                        {
                            let mut unwrappy = self.db.lock().unwrap();
                            unwrappy.load_table(&sharedtypes::LoadDBTable::Namespace);
                            let mut ns_u = Vec::new();
                            let mut ns_not_u = Vec::new();
                            for ns in ns {
                                if let Some(nsid) = unwrappy.namespace_get(ns) {
                                    ns_u.push(nsid.clone());
                                }
                            }
                            for ns in not_ns {
                                if let Some(nsid) = unwrappy.namespace_get(ns) {
                                    ns_not_u.push(nsid.clone());
                                }
                            }
                            let searchtype = match searchtype {
                                Some(searchtype) => match searchtype {
                                    sharedtypes::SearchType::String(temp) => {
                                        (Some(temp.clone()), None)
                                    }
                                    sharedtypes::SearchType::Regex(temp) => {
                                        let regex = regex::Regex::new(temp);

                                        if let Ok(regex) = regex {
                                            (None, Some(sharedtypes::RegexStorage(regex)))
                                        } else {
                                            logging::error_log(&format!(
                                                "Cannot load the regex from plugin: {} at path: {}",
                                                &global.name,
                                                path.to_string_lossy()
                                            ));
                                            continue;
                                        }
                                    }
                                },
                                None => {
                                    todo!();
                                }
                            };
                            match self.regex_storage.get_mut(&(
                                searchtype.clone(),
                                ns_u.clone(),
                                ns_not_u.clone(),
                            )) {
                                None => {
                                    self.regex_storage.insert(
                                        (searchtype.clone(), ns_u, ns_not_u),
                                        vec![global.clone()],
                                    );
                                }
                                Some(temp) => {
                                    temp.push(global.clone());
                                }
                            }
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
        logging::log(&format!(
            "Trying to load library at path: {}",
            path.to_string_lossy()
        ));
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

    pub fn filter_sites_return_lib(&self, site: &String) -> Option<&Library> {
        for scraper in self.scraper_get().iter() {
            if let Some(ref storage_type) = scraper.storage_type {
                if let sharedtypes::ScraperOrPlugin::Scraper(ref scraperinfo) = storage_type {
                    if scraperinfo.sites.contains(site) {
                        return self.library_get(scraper);
                    }
                }
            }
        }
        None
    }
}

///
/// Returns filehashes that have to be regenned.
/// I don't think this gets used?
///
pub fn scraper_file_regen(libloading: &libloading::Library) -> sharedtypes::ScraperFileRegen {
    let temp: libloading::Symbol<unsafe extern "C" fn() -> sharedtypes::ScraperFileRegen> =
        unsafe { libloading.get(b"scraper_file_regen\0").unwrap() };
    unsafe { temp() }
}
///
/// Used to generate a download link given the input data
///
pub fn scraper_file_return(
    libloading: &libloading::Library,
    regen: &sharedtypes::ScraperFileInput,
) -> sharedtypes::SubTag {
    let temp: libloading::Symbol<
        unsafe extern "C" fn(&sharedtypes::ScraperFileInput) -> sharedtypes::SubTag,
    > = unsafe { libloading.get(b"scraper_file_return\0").unwrap() };
    unsafe { temp(regen) }
}

#[cfg(test)]
pub(crate) mod test_globalload {

    use super::*;
    pub fn emulate_loaded(
        db: Arc<Mutex<database::Main>>,
        jobs: Arc<Mutex<Jobs>>,
    ) -> Arc<Mutex<GlobalLoad>> {
        GlobalLoad::new(db, jobs)
    }
}
