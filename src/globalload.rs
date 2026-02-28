use crate::RwLock;
use crate::{
    database::database::{self, Main},
    jobs::Jobs,
    logging, server,
    sharedtypes::{self, GlobalPluginScraper},
};
use libloading::Library;
use std::sync::Arc;
use std::{collections::HashMap, path::Path, thread};
use std::{path::PathBuf, thread::JoinHandle};
///
/// Runs the on_start callback
///
fn c_run_onstart(path: &Path, global: &sharedtypes::GlobalPluginScraper) {
    let liba;
    unsafe {
        liba = Library::new(path).unwrap();
    }
    unsafe {
        let plugindatafunc: libloading::Symbol<
            unsafe extern "C" fn(&sharedtypes::GlobalPluginScraper),
        > = match liba.get(b"on_start") {
            Ok(good) => good,
            Err(_) => {
                logging::log(format!(
                    "Cannot find on_start for path: {}",
                    path.to_string_lossy()
                ));
                return;
            }
        };
        liba.get::<libloading::Symbol<unsafe extern "C" fn(&sharedtypes::GlobalPluginScraper)>>(
            b"on_start",
        )
        .unwrap();
        plugindatafunc(global);
    };
}

/*///
/// Runs on importing a file
///
pub fn callback_on_import(
    manager_arc: Arc<RwLock<GlobalLoad>>,
    db: Main,
    bytes: &bytes::Bytes,
    hash: &String,
) {
}*/

///
/// Gets called onstartup of the software
///
pub fn on_start(libloading: &libloading::Library, site_struct: &sharedtypes::GlobalPluginScraper) {
    let temp: libloading::Symbol<unsafe extern "C" fn(&sharedtypes::GlobalPluginScraper)> =
        match unsafe { libloading.get(b"on_start\0") } {
            Err(_) => {
                logging::error_log_silent(format!(
                    "Cannot run on_start for name: {}",
                    site_struct.name
                ));
                return;
            }
            Ok(lib) => lib,
        };

    logging::log(format!(
        "Running on_start call for name: {}",
        site_struct.name
    ));
    unsafe { temp(site_struct) }
}

#[derive(Clone)]
pub struct GlobalLoad {
    db: Main,
    callback:
        Arc<RwLock<HashMap<sharedtypes::GlobalCallbacks, Vec<sharedtypes::GlobalPluginScraper>>>>,
    callback_cross:
        Arc<RwLock<HashMap<sharedtypes::GlobalPluginScraper, Vec<sharedtypes::CallbackInfo>>>>,
    callback_storage: Arc<
        RwLock<HashMap<String, Vec<(sharedtypes::CallbackInfo, sharedtypes::GlobalPluginScraper)>>>,
    >,
    sites: Arc<RwLock<HashMap<sharedtypes::GlobalPluginScraper, Vec<String>>>>,
    library_path: Arc<RwLock<HashMap<sharedtypes::GlobalPluginScraper, PathBuf>>>,
    library_lib:
        Arc<RwLock<HashMap<sharedtypes::GlobalPluginScraper, Arc<RwLock<libloading::Library>>>>>,
    default_load: Arc<RwLock<LoadableType>>,
    thread: Arc<RwLock<HashMap<sharedtypes::GlobalPluginScraper, JoinHandle<()>>>>,
    ipc_server: Arc<RwLock<Option<JoinHandle<()>>>>,
    regex_storage: Arc<
        RwLock<
            HashMap<
                (
                    (Option<String>, Option<sharedtypes::RegexStorage>),
                    Vec<usize>,
                    Vec<usize>,
                ),
                Vec<sharedtypes::GlobalPluginScraper>,
            >,
        >,
    >,
    jobmanager: Arc<RwLock<Jobs>>,
}

///
/// Determines what we should return from our get_loadable_paths function
///
enum LoadableType {
    Release,
    Debug,
}

impl GlobalLoad {
    pub fn new(db: Main, jobs: Arc<RwLock<Jobs>>) -> Self {
        logging::log("Starting IPC Server.".to_string());

        GlobalLoad {
            db,
            callback: Arc::new(RwLock::new(HashMap::new())),
            callback_cross: Arc::new(RwLock::new(HashMap::new())),
            callback_storage: Arc::new(RwLock::new(HashMap::new())),
            sites: Arc::new(RwLock::new(HashMap::new())),
            library_path: Arc::new(RwLock::new(HashMap::new())),
            library_lib: Arc::new(RwLock::new(HashMap::new())),
            default_load: Arc::new(RwLock::new(LoadableType::Release)),
            thread: Arc::new(RwLock::new(HashMap::new())),
            ipc_server: Arc::new(RwLock::new(None)),
            regex_storage: Arc::new(RwLock::new(HashMap::new())),
            jobmanager: jobs,
        }
    }

    ///
    /// This function gets called after a DB upgrade
    ///
    fn db_upgrade_call(
        &self,
        libloading: &RwLock<libloading::Library>,
        db_version: &usize,
        site_struct: &sharedtypes::GlobalPluginScraper,
    ) {
        let libloading = libloading.read();
        let temp: libloading::Symbol<
            unsafe extern "C" fn(&usize, &sharedtypes::GlobalPluginScraper),
        > = match unsafe { libloading.get(b"db_upgrade_call\0") } {
            Err(err) => {
                logging::error_log(format!(
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
    /// Hopefully a thread-safe way to call plugins per thread avoiding a lock.
    ///
    pub fn plugin_on_download(&self, db: Main, cursorpass: &[u8], hash: &String, ext: &String) {
        let (libpath, libscraper);
        {
            (libpath, libscraper) = (
                self.get_lib_path_from_callback(&sharedtypes::GlobalCallbacks::Download),
                self.get_scrapers_from_callback(&sharedtypes::GlobalCallbacks::Download),
            );
        }

        let api = db.api_info.read();
        for (cnt, lib_path) in libpath.iter().enumerate() {
            let lib;
            unsafe {
                match libloading::Library::new(lib_path) {
                    Ok(good_lib) => lib = good_lib,
                    Err(_) => {
                        logging::error_log(format!(
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
                    &sharedtypes::ClientAPIInfo
                    )
                        -> Vec<sharedtypes::DBPluginOutputEnum>,
                    //unsafe extern "C" fn(Cursor<Bytes>, &String, &String, database::Main),
                > = match lib.get(b"on_download") {
                    Ok(lib) => lib,
                    Err(_) => {
                        logging::info_log(format!(
                            "Could not find on_download for plugin: {}",
                            lib_path.to_string_lossy()
                        ));
                        continue;
                    }
                };
                //db.
                output = plugindatafunc(cursorpass, hash, ext, &api.clone());
            }

            let jobmanager;
            {
                jobmanager = self.jobmanager.clone();
            }
            self.parse_plugin_output(output, db.clone(), libscraper.get(cnt).unwrap(), jobmanager);
        }
    }

    ///
    /// Runs internal logic to parse output from plugins
    ///
    fn parse_plugin_output_andmain(
        &self,
        plugin_data: Vec<sharedtypes::DBPluginOutputEnum>,
        db: Main,
        scraper: &GlobalPluginScraper,
        jobmanager: Arc<RwLock<Jobs>>,
    ) {
        for each in plugin_data {
            match each {
                sharedtypes::DBPluginOutputEnum::Add(namespaces) => {
                    for names in namespaces {
                        // 1️⃣ Collect all DB writes first
                        let mut files_to_add = Vec::new();
                        let mut tags_to_add = Vec::new();
                        let mut relationships_to_add = Vec::new();
                        let mut jobs_to_add = Vec::new();

                        // Collect files
                        for files in names.file.iter() {
                            if files.id.is_none() && files.hash.is_some() && files.ext.is_some() {
                                files_to_add.push(files.clone());
                            }
                        }

                        // Collect tags
                        for tag in names.tag.iter() {
                            tags_to_add.push(tag.clone());
                        }

                        // Collect relationships
                        for rel in names.relationship.iter() {
                            relationships_to_add.push(rel.clone());
                        }

                        // Collect jobs
                        for job in names.jobs.iter() {
                            jobs_to_add.push(job.clone());
                        }

                        // 2️⃣ Apply file additions in bulk under one write lock
                        {
                            for files in files_to_add {
                                let ext_id = db.extension_put_string(&files.ext.clone().unwrap());
                                let storage_id = match &files.location {
                                    Some(exists) => {
                                        db.storage_put(exists);
                                        db.storage_get_id(exists).unwrap()
                                    }
                                    None => {
                                        let exists = db.location_get();
                                        db.storage_put(&exists);
                                        db.storage_get_id(&exists).unwrap()
                                    }
                                };
                                let file = sharedtypes::DbFileStorage::NoIdExist(
                                    sharedtypes::DbFileObjNoId {
                                        hash: files.hash.clone().unwrap(),
                                        ext_id,
                                        storage_id,
                                    },
                                );
                                db.file_add(file);
                            }
                            db.transaction_flush();
                        }

                        // 3️⃣ Add tags under one write lock
                        let mut tag_id_map = HashMap::new();
                        {
                            for tag in &tags_to_add {
                                tag_id_map.insert(tag.clone(), db.tag_add_tagobject(tag));
                            }
                        }

                        // Call plugin hook outside lock
                        for tag in &tags_to_add {
                            self.plugin_on_tag(tag);
                        }

                        // 4️⃣ Add jobs under one write lock
                        {
                            let mut jm = jobmanager.write();
                            for job in jobs_to_add {
                                jm.jobs_add_nolock(scraper.clone(), job);
                            }
                        }

                        // 5️⃣ Add relationships under one write lock
                        {
                            for rel in relationships_to_add {
                                if let Some(ns_id) = db.namespace_get(&rel.tag_namespace) {
                                    if let (Some(file_id), Some(tag_id)) = (
                                        db.file_get_hash(&rel.file_hash),
                                        db.tag_get_name(rel.tag_name.clone(), ns_id),
                                    ) {
                                        db.relationship_add(file_id, tag_id);
                                    }
                                }
                            }
                        }
                    }
                }
                sharedtypes::DBPluginOutputEnum::Del(_) => {} // handle deletes similarly if needed
                sharedtypes::DBPluginOutputEnum::Set(_) => {} // Sets the tags relationships or jobs
            }
        }
    }

    ///
    /// Tells a scraper that it should handle the "text" downloading
    ///
    pub fn text_scraping(
        &self,
        url_output: &str,
        actual_params: &[sharedtypes::ScraperParam],
        scraperdata: &sharedtypes::ScraperDataReturn,
        scraper: &GlobalPluginScraper,
    ) -> Vec<sharedtypes::ScraperReturn> {
        if let Some(lib) = self.library_get(scraper) {
            let lib = lib.read();
            let temp: libloading::Symbol<
                unsafe extern "C" fn(
                    &str,
                    &[sharedtypes::ScraperParam],
                    &sharedtypes::ScraperDataReturn,
                ) -> Vec<sharedtypes::ScraperReturn>,
            > = unsafe { lib.get(b"text_scraping\0").unwrap() };
            unsafe { temp(url_output, actual_params, scraperdata) }
        } else {
            vec![sharedtypes::ScraperReturn::Nothing]
        }
    }

    ///
    /// Parses output from plugin.
    ///
    fn parse_plugin_output(
        &self,
        plugin_data: Vec<sharedtypes::DBPluginOutputEnum>,
        db: Main,
        scraper: &GlobalPluginScraper,
        jobmanager: Arc<RwLock<Jobs>>,
    ) {
        //let db = self._database.lock().unwrap();
        self.parse_plugin_output_andmain(plugin_data, db, scraper, jobmanager);
    }

    ///
    /// Gets a scraper to output any URLs based on params
    ///
    pub fn url_dump(
        &self,
        params: &Vec<sharedtypes::ScraperParam>,
        scraperdata: &sharedtypes::ScraperDataReturn,
        scraper: &sharedtypes::GlobalPluginScraper,
    ) -> Result<Vec<sharedtypes::ScraperDataReturn>, libloading::Error> {
        let mut out = Vec::new();

        if let Some(lib) = self.library_get(scraper) {
            let lib = lib.read();
            let temp: libloading::Symbol<
                unsafe extern "C" fn(
                    &[sharedtypes::ScraperParam],
                    &sharedtypes::ScraperDataReturn,
                ) -> Vec<sharedtypes::ScraperDataReturn>,
            > = unsafe { lib.get(b"url_dump\0")? };
            for item in unsafe { temp(params, scraperdata) } {
                out.push(item);
            }
        }
        Ok(out)
    }

    ///
    /// Used by a scraper to download a file. ONLY ONE :/
    ///
    pub fn download_from(
        &self,
        file: &sharedtypes::FileObject,
        scraper: &sharedtypes::GlobalPluginScraper,
    ) -> Option<Vec<u8>> {
        if let Some(lib) = self.library_get(scraper) {
            let lib = lib.read();
            let temp: libloading::Symbol<
                unsafe extern "C" fn(&sharedtypes::FileObject) -> Option<Vec<u8>>,
            > = unsafe { lib.get(b"download_from\0").unwrap() };
            return unsafe { temp(file) };
        }
        None
    }

    ///
    /// Calls a parser to cleave information from a lib
    ///
    pub fn parser_call(
        &self,
        url_output: &str,
        source_url: &str,
        scraperdata: &sharedtypes::ScraperDataReturn,
        scraper: &GlobalPluginScraper,
    ) -> Vec<sharedtypes::ScraperReturn> {
        if let Some(scraper_library_rwlock) = self.library_get(scraper) {
            let scraper_library = scraper_library_rwlock.read();
            let temp: libloading::Symbol<
                unsafe extern "C" fn(
                    &str,
                    &str,
                    &sharedtypes::ScraperDataReturn,
                ) -> Vec<sharedtypes::ScraperReturn>,
            > = {
                unsafe {
                    match scraper_library.get(b"parser\0") {
                        Err(err) => {
                            return vec![sharedtypes::ScraperReturn::Stop(
                                "Missing parser block in scraper".to_string(),
                            )];
                        }
                        Ok(out) => out,
                    }
                }
            };
            unsafe { temp(url_output, source_url, scraperdata) }
        } else {
            vec![sharedtypes::ScraperReturn::Nothing]
        }
    }

    pub fn callback_on_import(&self, bytes: &bytes::Bytes, hash: &String) {
        let (libpath, libscraper);
        {
            (libpath, libscraper) = (
                self.get_lib_path_from_callback(&sharedtypes::GlobalCallbacks::Import),
                self.get_scrapers_from_callback(&sharedtypes::GlobalCallbacks::Import),
            );
        }

        for (cnt, lib_path) in libpath.iter().enumerate() {
            let lib;
            unsafe {
                match libloading::Library::new(lib_path) {
                    Ok(good_lib) => lib = good_lib,
                    Err(_) => {
                        logging::error_log(format!(
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
                    unsafe extern "C" fn(&[u8], &String) -> Vec<sharedtypes::DBPluginOutputEnum>,
                    //unsafe extern "C" fn(Cursor<Bytes>, &String, &String, database::Main),
                > = match lib.get(b"on_import") {
                    Ok(lib) => lib,
                    Err(_) => {
                        logging::info_log(format!(
                            "Could not find on_download for plugin: {}",
                            lib_path.to_string_lossy()
                        ));
                        continue;
                    }
                };
                //db.
                output = plugindatafunc(bytes, hash);
            }

            self.parse_plugin_output_local(output, libscraper.get(cnt).unwrap());
            /*parse_plugin_output(
                output,
                db.clone(),
                libscraper.get(cnt).unwrap(),
                jobmanager,
                manager_arc.clone(),
            );*/
            let _ = lib.close();
        }
    }

    fn run_regex(&self, name: &String, namespace: &sharedtypes::GenericNamespaceObj) {
        let mut storagemap = Vec::new();
        let tag_nsid = match self.db.namespace_get(&namespace.name) {
            Some(id) => id,
            None => return,
        };
        {
            let reg_store = self.return_regex_storage();
            'searchloop: for (((search_string, search_regex), ns, not_ns), pluginscraper_list) in
                reg_store.iter()
            {
                // Filtering for weather we should apply this to a tag of X namespace
                for ns in ns {
                    if *ns != tag_nsid {
                        continue 'searchloop;
                    }
                }
                for not_ns in not_ns {
                    if *not_ns == tag_nsid {
                        continue 'searchloop;
                    }
                }

                // Actual matching going on here
                if let Some(search) = search_string {
                    if name.contains(search) {}
                } else if let Some(regex) = search_regex {
                    let regex_iter: Vec<&str> =
                        regex.0.find_iter(name).map(|m| m.as_str()).collect();
                    for regexmatch in regex_iter {
                        for pluginscraper in pluginscraper_list {
                            storagemap.push((
                                pluginscraper.clone(),
                                regexmatch,
                                Some(sharedtypes::SearchType::Regex(regex.0.to_string())),
                            ));
                        }
                    }
                } else {
                    for pluginscraper in pluginscraper_list {
                        storagemap.push((pluginscraper.clone(), "", None));
                    }
                }
            }
        }

        // #TODO need to fix this. Is calling multiple times
        for (pluginscraper, regex, searchtype) in storagemap.iter() {
            let mut pluginscraper = pluginscraper.clone();
            if let Some(scraper_or_plugin) = &pluginscraper.storage_type
                && let sharedtypes::ScraperOrPlugin::Plugin(plugininfo) = scraper_or_plugin
                && let Some(redirect_site) = &plugininfo.redirect
                && let Some(good_scraper) = self.return_scraper_from_site(redirect_site)
            {
                pluginscraper = good_scraper.clone();
            }

            let tag_ns;
            {
                match self.db.namespace_get_string(&tag_nsid) {
                    None => {
                        continue;
                    }
                    Some(nso) => {
                        tag_ns = nso.name.clone();
                    }
                }
            }

            let liba;
            {
                match self.library_get_path(&pluginscraper) {
                    None => {
                        liba = None;
                    }
                    Some(libpath) => {
                        liba = Some(libpath.clone());
                    }
                }
            }
            if let Some(liba) = liba {
                self.c_regex_match(
                    name,
                    namespace,
                    regex,
                    searchtype,
                    &unsafe { libloading::Library::new(liba).unwrap() },
                    &pluginscraper,
                );
            }
        }
    }

    ///
    /// Threadsafe way to call callback on adding a tag into the db
    ///
    pub fn plugin_on_tag(
        &self,

        tag: &sharedtypes::TagObject, //tag: &String,tag_nsid: &usize,
    ) {
        // Designed to run regex on any tag that comes in. I'll leave the filtering to the plugins
        self.run_regex(&tag.tag, &tag.namespace);
        if let Some(relate) = &tag.relates_to {
            self.run_regex(&relate.tag, &relate.namespace);
            if let Some(limitto) = &relate.limit_to {
                self.run_regex(&limitto.tag, &limitto.namespace);
            }
        }
    }

    ///
    /// Calls a regex function
    ///
    fn c_regex_match(
        &self,

        tag: &str,
        tag_namespace: &sharedtypes::GenericNamespaceObj,
        regex_match: &str,
        plugin_callback: &Option<sharedtypes::SearchType>,
        liba: &libloading::Library,
        scraper: &GlobalPluginScraper,
    ) {
        let output;
        unsafe {
            let plugindatafunc: libloading::Symbol<
                unsafe extern "C" fn(
                    &str,
                    &sharedtypes::GenericNamespaceObj,
                    &str,
                    &Option<sharedtypes::SearchType>,
                ) -> Vec<sharedtypes::DBPluginOutputEnum>,
            > = match liba.get(b"on_regex_match") {
                Ok(good) => good,
                Err(_) => {
                    logging::error_log_silent(format!(
                        "Could not find function on_regex_match for plugin: {}",
                        scraper.name
                    ));
                    return;
                }
            };
            liba.get::<libloading::Symbol<
                unsafe extern "C" fn(
                    &str,
                    &sharedtypes::GenericNamespaceObj,
                    &str,
                    &Option<sharedtypes::SearchType>,
                ) -> Vec<sharedtypes::DBPluginOutputEnum>,
            >>(b"on_regex_match")
                .unwrap();
            output = plugindatafunc(tag, tag_namespace, regex_match, plugin_callback);
        };
        self.parse_plugin_output_local(output, scraper);
        // parse_plugin_output_andmain(output, db, scraper, jobmanager, &self)
    }

    ///
    /// Local plugin parser
    ///
    fn parse_plugin_output_local(
        &self,
        plugin_data: Vec<sharedtypes::DBPluginOutputEnum>,
        scraper: &GlobalPluginScraper,
    ) {
        for each in plugin_data {
            match each {
                sharedtypes::DBPluginOutputEnum::Add(name) => {
                    for names in name {
                        // Loops through the namespace objects and selects the last one that's valid.
                        // IF ONE IS NOT VALID THEN THEIR WILL NOT BE ONE ADDED INTO THE DB
                        for files in names.file {
                            if files.id.is_none() && files.hash.is_some() && files.ext.is_some() {
                                // Gets the extension id
                                let ext_id = self.db.extension_put_string(&files.ext.unwrap());

                                let storage_id = match files.location {
                                    Some(exists) => {
                                        self.db.storage_put(&exists);
                                        self.db.storage_get_id(&exists).unwrap()
                                    }
                                    None => {
                                        let exists = self.db.location_get();
                                        self.db.storage_put(&exists);
                                        self.db.storage_get_id(&exists).unwrap()
                                    }
                                };

                                let file = sharedtypes::DbFileStorage::NoIdExist(
                                    sharedtypes::DbFileObjNoId {
                                        hash: files.hash.unwrap(),
                                        ext_id,
                                        storage_id,
                                    },
                                );
                                self.db.file_add(file);
                            }
                        }
                        for tag in names.tag {
                            if tag.tag_type != sharedtypes::TagType::NormalNoRegex {
                                self.plugin_on_tag(&tag);
                            }
                            {
                                self.db.tag_add_tagobject(&tag);
                            }
                        }
                        for settings in names.setting {
                            self.db.setting_add(
                                settings.name,
                                settings.pretty,
                                settings.num,
                                settings.param,
                            );
                        }

                        for job in names.jobs {
                            self.jobmanager
                                .write()
                                .jobs_add_nolock(scraper.clone(), job);
                            //db.jobs_add_new(job);
                        }

                        let mut temp_vec: Vec<(Option<usize>, Option<usize>)> = Vec::new();
                        {
                            for relations in names.relationship {
                                let file_id = self.db.file_get_hash(&&relations.file_hash);
                                let namespace_id = self.db.namespace_get(&&relations.tag_namespace);
                                let tag_id = self.db.tag_get_name(
                                    relations.tag_name.clone(),
                                    namespace_id.unwrap(),
                                );
                                temp_vec.push((file_id, tag_id));
                                /*println!(
                                    "plugins356 relating: file id {:?} to {:?}",
                                    file_id, relations.tag_name
                                );*/
                                //db.relationship_add(file, tag, addtodb)
                            }
                        }
                        for (file_id, tag_id) in temp_vec {
                            self.db.relationship_add(file_id.unwrap(), tag_id.unwrap());
                        }
                    }
                }
                sharedtypes::DBPluginOutputEnum::Del(name) => for _names in name {},
                sharedtypes::DBPluginOutputEnum::Set(_) => {}
            }
        }
    }

    pub fn return_regex_storage(
        &self,
    ) -> HashMap<
        (
            (Option<String>, Option<sharedtypes::RegexStorage>),
            Vec<usize>,
            Vec<usize>,
        ),
        Vec<sharedtypes::GlobalPluginScraper>,
    > {
        self.regex_storage.read().clone()
    }

    ///
    /// Returns a scraper from a list of sites
    ///
    pub fn return_scraper_from_site(
        &self,
        site: &String,
    ) -> Option<sharedtypes::GlobalPluginScraper> {
        for (scraper, sites) in self.sites.read().iter() {
            if sites.contains(site) {
                return Some(scraper.clone());
            }
        }
        None
    }

    pub fn setup_ipc(&mut self, globalload: GlobalLoad, db: Main, jobs: Arc<RwLock<Jobs>>) {
        let srv = std::thread::spawn(move || {
            let mut ipc_coms = server::PluginIpcInteract::new(db.clone(), globalload, jobs);
            //let _ = rcv.recv();

            //println!("v");
            match ipc_coms.spawn_listener(db.clone()) {
                Ok(out) => out,
                Err(err) => {
                    logging::error_log(format!("ERROR: {:?}", err));
                    logging::panic_log("Failed to spawn IPC Server".to_string());
                }
            }
        });

        self.ipc_server = Arc::new(RwLock::new(Some(srv)));
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
        if let Some(callbacklist) = self.callback.read().get(callback) {
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
        if let Some(callbacklist) = self.callback.read().get(callback) {
            for callback_item in callbacklist {
                if let Some(libp) = self.library_path.read().get(callback_item) {
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
        &self,
        func_name: &str,
        vers: &usize,
        input_data: &sharedtypes::CallbackInfoInput,
    ) -> HashMap<String, sharedtypes::CallbackCustomDataReturning> {
        if let Some(callback_list) = self.callback_storage.read().get(func_name) {
            for (callback, global_plugin) in callback_list {
                if *vers == callback.vers {
                    if let Some(plugin_lib) = self.library_lib.read().get(global_plugin) {
                        let plugin_lib = plugin_lib.read();
                        let plugininfo;
                        unsafe {
                            let plugindatafunc: libloading::Symbol<
                                unsafe extern "C" fn(
                                    &sharedtypes::CallbackInfoInput,
                                ) -> HashMap<
                                    String,
                                    sharedtypes::CallbackCustomDataReturning,
                                >,
                            > = plugin_lib.get(func_name.as_bytes()).unwrap();
                            plugininfo = plugindatafunc(input_data);
                        }
                        return plugininfo;
                    }
                }
            }
        }

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
        HashMap::new()
    }

    pub fn return_external_plugin_call(&self, func_name: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(callback_list) = self.callback_storage.read().get(func_name) {
            for (callback, callbackscraper) in callback_list {}
        }

        out
    }
    ///
    /// Returns true if we have no threads that are running
    ///
    pub fn return_thread(&self) -> bool {
        self.thread.read().is_empty()
    }

    ///
    /// Returns a Library if it exists
    ///
    pub fn library_get(
        &self,
        global: &sharedtypes::GlobalPluginScraper,
    ) -> Option<Arc<RwLock<Library>>> {
        let lib = self.library_lib.read();
        lib.get(global).cloned()
    }

    ///
    /// Returns a library path if it exists
    ///
    pub fn library_get_path(&self, global: &sharedtypes::GlobalPluginScraper) -> Option<PathBuf> {
        self.library_path.read().get(global).cloned()
    }

    /* ///
    /// Returns the libraries raw
    ///
    pub fn library_get_raw(&self) -> &HashMap<GlobalPluginScraper, RwLock<Library>> {
        &self.library_lib.read().cloned()
    }*/

    pub fn run_upgrade_logic(&self, db_version: &usize) {
        for internal_scraper in self.library_lib.read().keys() {
            if let Some(scraper_library) = self.library_lib.read().get(internal_scraper) {
                self.db_upgrade_call(scraper_library, &db_version, internal_scraper);
            }
        }
    }

    ///
    /// Closes any open threads that might still be open
    ///
    pub fn thread_finish_closed(&mut self) {
        let mut finished_threads = Vec::new();
        for (scraper, thread) in self.thread.read().iter() {
            if thread.is_finished() {
                finished_threads.push(scraper.clone());
            }
        }

        for thread in finished_threads.iter() {
            let th = self.thread.write().remove(thread).unwrap();
            let _ = th.join();
        }
    }

    pub fn return_all_sites(&self) -> Vec<(sharedtypes::GlobalPluginScraper, String)> {
        let mut out = Vec::new();
        for scraper in self.scraper_get().iter() {
            for site in self.sites_get(scraper) {
                out.push((scraper.clone(), site));
            }
        }
        out
    }

    ///
    /// Returns all loaded scrapers
    ///
    pub fn scraper_get(&self) -> Vec<sharedtypes::GlobalPluginScraper> {
        let mut out = Vec::new();
        for each in self.sites.read().keys() {
            out.push(each.clone());
        }
        out
    }

    ///
    /// Returns all sites for a scraper
    ///
    pub fn sites_get(&self, global: &sharedtypes::GlobalPluginScraper) -> Vec<String> {
        match self.sites.read().get(global) {
            None => Vec::new(),
            Some(out) => out.clone(),
        }
    }

    ///
    /// Triggers the on_start for the plugins
    ///
    pub fn pluginscraper_on_start(&mut self) {
        for (callback, list) in self.callback.read().iter() {
            if let sharedtypes::GlobalCallbacks::Start(thread_type) = callback {
                for to_run in list {
                    logging::log(format!("Starting Call Start for: {}", &to_run.name));
                    let file = self.library_path.read().get(to_run).unwrap().clone();
                    match thread_type {
                        sharedtypes::StartupThreadType::Spawn => {
                            let run = to_run.clone();
                            let to_run = to_run.clone();
                            let thread = thread::spawn(move || {
                                c_run_onstart(&file, &to_run.clone());
                            });
                            self.thread.write().insert(run.clone(), thread);
                        }
                        sharedtypes::StartupThreadType::SpawnInline => {
                            let run = to_run.clone();
                            let to_run = to_run.clone();
                            let thread = thread::spawn(move || {
                                c_run_onstart(&file, &to_run);
                            });
                            self.thread.write().insert(run.clone(), thread);
                        }
                        sharedtypes::StartupThreadType::Inline => {
                            c_run_onstart(&file, to_run);
                        }
                    }
                }
            }
        }
    }

    ///
    /// Waits if a flag was set if we should wait for the thread to finish
    ///
    pub fn plugin_on_start_should_wait(&mut self) -> bool {
        self.thread_finish_closed();
        for (check, list) in self.callback.read().iter() {
            if let sharedtypes::GlobalCallbacks::Start(thread) = check {
                for item in list {
                    if &sharedtypes::StartupThreadType::SpawnInline == thread
                        && self.thread.read().contains_key(item)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    ///
    /// Clears the regex cache and callbacks
    ///
    fn clear_regex(&self) {
        self.callback.write().clear();
        self.callback_cross.write().clear();
        self.regex_storage.write().clear();
    }

    ///
    /// Reloads the regex stores
    ///
    pub fn reload_regex(&self) {
        self.clear_regex();
        {
            let db = self.db.clone();
            let scraper_folder;
            let plugin_folder;
            {
                scraper_folder = db.loaded_scraper_folder();
                plugin_folder = db.loaded_plugin_folder();
            }
            self.load_folder(&scraper_folder);
            self.load_folder(&plugin_folder);
        }
    }

    ///
    /// Actually parses the Library
    /// TODO needs to make easy pulls for scraper and plugin info
    ///
    fn parse_lib(&self, lib: Library, path: &Path) {
        if let Some(items) = self.get_info(&lib, path) {
            if items.is_empty() {
                logging::error_log(format!(
                    "Was unable to pull any sites from: {}",
                    path.to_string_lossy()
                ));
                return;
            }
            for global in items {
                match global.storage_type {
                    None => {
                        logging::error_log(format!(
                            "Skipping parsing of name: {} due to storage_type not being set.From {}",
                            global.name,
                            path.to_string_lossy()
                        ));

                        continue;
                    }
                    Some(ref scraperplugin) => match scraperplugin {
                        sharedtypes::ScraperOrPlugin::Scraper(scraperinfo) => {
                            self.sites
                                .write()
                                .insert(global.clone(), scraperinfo.sites.clone());
                        }
                        sharedtypes::ScraperOrPlugin::Plugin(_plugininfo) => {}
                    },
                }

                for callbacks in global.callbacks.iter() {
                    match callbacks {
                        sharedtypes::GlobalCallbacks::Callback(callback_info) => {
                            // Stores the callbacks pertaining to externals
                            {
                                let mut callback_storage = self.callback_storage.write();
                                match callback_storage.get_mut(&callback_info.func) {
                                    None => {
                                        callback_storage.insert(
                                            callback_info.func.to_string(),
                                            vec![(callback_info.clone(), global.clone())],
                                        );
                                    }
                                    Some(vec) => {
                                        vec.push((callback_info.clone(), global.clone()));
                                    }
                                }
                            }

                            // Some sort of goofy regex callback maybe not used
                            {
                                let mut callback_cross = self.callback_cross.write();
                                match callback_cross.get_mut(&global) {
                                    None => {
                                        callback_cross
                                            .insert(global.clone(), vec![callback_info.clone()]);
                                    }
                                    Some(list) => {
                                        list.push(callback_info.clone());
                                    }
                                }
                            }
                        }
                        _ => {
                            let mut callback = self.callback.write();
                            match callback.get_mut(callbacks) {
                                None => {
                                    let temp = vec![global.clone()];
                                    callback.insert(callbacks.clone(), temp);
                                }
                                Some(plugin_list) => {
                                    plugin_list.push(global.clone());
                                }
                            }
                        }
                    }

                    if let sharedtypes::GlobalCallbacks::Tag((searchtype, ns, not_ns)) = callbacks {
                        self.db.load_table(&sharedtypes::LoadDBTable::Namespace);
                        let mut ns_u = Vec::new();
                        let mut ns_not_u = Vec::new();
                        for ns in ns {
                            ns_u.push(self.db.namespace_add(ns, &None));
                        }
                        for ns in not_ns {
                            ns_not_u.push(self.db.namespace_add(ns, &None));
                        }
                        let searchtype = match searchtype {
                            Some(searchtype) => match searchtype {
                                sharedtypes::SearchType::String(temp) => (Some(temp.clone()), None),
                                sharedtypes::SearchType::Regex(temp) => {
                                    let regex = regex::Regex::new(temp);

                                    if let Ok(regex) = regex {
                                        (None, Some(sharedtypes::RegexStorage(regex)))
                                    } else {
                                        logging::error_log(format!(
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
                        let mut regex_storage = self.regex_storage.write();
                        match regex_storage.get_mut(&(
                            searchtype.clone(),
                            ns_u.clone(),
                            ns_not_u.clone(),
                        )) {
                            None => {
                                regex_storage.insert(
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

                self.library_path
                    .write()
                    .insert(global.clone(), path.to_path_buf());
                let lib;
                unsafe {
                    lib = libloading::Library::new(path).unwrap();
                }
                self.library_lib
                    .write()
                    .insert(global, Arc::new(RwLock::new(lib)));
            }
        }
    }

    fn get_info(
        &self,
        lib: &Library,
        path: &Path,
    ) -> Option<Vec<sharedtypes::GlobalPluginScraper>> {
        logging::log(format!(
            "Trying to load library at path: {}",
            path.to_string_lossy()
        ));
        let temp: libloading::Symbol<
            unsafe extern "C" fn() -> Vec<sharedtypes::GlobalPluginScraper>,
        > = match unsafe { lib.get(b"get_global_info\0") } {
            Err(_) => {
                logging::error_log_silent(format!(
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
    pub fn load_folder(&self, folder: &Path) {
        if !folder.exists() {
            let path_check = std::fs::create_dir_all(folder);
            match path_check {
                Ok(_) => (),
                Err(_) => {
                    logging::error_log(format!(
                        "CANNOT CREATE FOLDER: {:?} DUE TO PERMISSIONS. STOPPING SEARCH",
                        folder.to_str()
                    ));
                    return;
                }
            }
        }
        if folder.is_file() {
            logging::error_log(format!(
                "THIS IS A FILE DUM DUM. PATH: {:?}.... STOPPING SEARCH",
                folder.to_str()
            ));
            return;
        }
        let loadable_string = match *self.default_load.read() {
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

    pub fn filter_sites_return_lib(&self, site: &String) -> Option<Arc<RwLock<Library>>> {
        for scraper in self.scraper_get().iter() {
            if let Some(ref storage_type) = scraper.storage_type
                && let sharedtypes::ScraperOrPlugin::Scraper(scraperinfo) = &storage_type
                && scraperinfo.sites.contains(site)
            {
                return self.library_get(scraper);
            }
        }
        None
    }
}

///
/// Returns filehashes that have to be regenned.
/// I don't think this gets used?
///
pub fn scraper_file_regen(lib: &RwLock<libloading::Library>) -> sharedtypes::ScraperFileRegen {
    let libloading = lib.read();
    let temp: libloading::Symbol<unsafe extern "C" fn() -> sharedtypes::ScraperFileRegen> =
        unsafe { libloading.get(b"scraper_file_regen\0").unwrap() };
    unsafe { temp() }
}
///
/// Used to generate a download link given the input data
///
pub fn scraper_file_return(
    lib: &RwLock<libloading::Library>,
    regen: &sharedtypes::ScraperFileInput,
) -> sharedtypes::SubTag {
    let libloading = lib.read();
    let temp: libloading::Symbol<
        unsafe extern "C" fn(&sharedtypes::ScraperFileInput) -> sharedtypes::SubTag,
    > = unsafe { libloading.get(b"scraper_file_return\0").unwrap() };
    unsafe { temp(regen) }
}

#[cfg(test)]
pub(crate) mod test_globalload {

    use super::*;
    pub fn emulate_loaded(db: database::Main, jobs: Arc<RwLock<Jobs>>) -> GlobalLoad {
        GlobalLoad::new(db, jobs)
    }
}
