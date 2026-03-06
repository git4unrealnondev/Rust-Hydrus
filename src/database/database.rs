#![forbid(unsafe_code)]
use crate::Mutex;
use crate::RwLock;
use crate::database::inmemdbnew::NewinMemDB;
use crate::file;
use crate::globalload::GlobalLoad;
use crate::helpers;
use crate::helpers::check_url;
use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::DEFAULT_CACHECHECK;
use crate::sharedtypes::ScraperParam;
use eta::{Eta, TimeAcc};
use log::{error, info};
use r2d2::Pool;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::prelude::*;
pub use rusqlite::{Connection, Result, Transaction, params, types::Null};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
/// I dont want to keep writing .to_string on EVERY vector of strings. Keeps me
/// lazy. vec_of_strings["one", "two"];
#[macro_export]
macro_rules! vec_of_strings{
    ($($x: expr), *) =>(vec![$($x.to_string()), *]);
}

/*/// Returns an open tnection to use.
fn dbinit(dbpath: &String) -> tnection {
    // Engaging &Transaction Handling
    tnection::open(dbpath).unwrap()
}*/

#[derive(Clone, Debug)]
pub enum CacheType {
    // Default option. Will use in memory DB to make store cached data.
    InMemdb,
    // Not yet implmented will be used for using sqlite 3 inmemory db calls.
    InMemory(Pool<SqliteConnectionManager>),
    // Will be use to query the DB directly. No caching.
    Bare,
}

#[derive(Clone)]
/// Holder of database self variables
pub(crate) struct Main {
    pub(super) _dbpath: Option<String>,
    pub(super) _vers: usize,
    pub(super) pool: Pool<SqliteConnectionManager>,
    pub(in crate::database) write_conn: Arc<Mutex<PooledConnection<SqliteConnectionManager>>>,
    pub(super) write_conn_istransaction: Arc<Mutex<bool>>,
    pub(super) _active_vers: usize,
    pub(super) _inmemdb: Arc<RwLock<NewinMemDB>>,
    pub(super) tables_loaded: Arc<RwLock<Vec<sharedtypes::LoadDBTable>>>,
    pub(super) tables_loading: Arc<RwLock<Vec<sharedtypes::LoadDBTable>>>,
    pub(super) _cache: CacheType,
    pub globalload: Option<Arc<GlobalLoad>>,
    pub(super) localref: Option<Arc<RwLock<Main>>>,
    pub api_info: Arc<RwLock<Option<sharedtypes::ClientAPIInfo>>>,
    pub(in crate::database) popular_relationship_count: Arc<Mutex<Option<usize>>>,
}

/// Handles transactional pushes.
fn transaction_execute(trans: &Transaction, inp: String) {
    trans.execute(&inp, params![]).unwrap();
}

/// Contains DB functions.
impl Main {
    pub fn get_api_url(&self) -> sharedtypes::ClientAPIInfo {
        match self.settings_get_name(&"SYSTEM_API_URL".to_string()) {
            Some(setting) => {
                if let Some(param) = setting.param {
                    if let Ok(url) = SocketAddr::from_str(&param) {
                        return sharedtypes::ClientAPIInfo {
                            url,
                            authentication: None,
                        };
                    }
                }
            }
            None => {}
        }
        let url = "127.0.0.1:3030".to_string();

        if let Ok(url) = SocketAddr::from_str(&url) {
            self.setting_add("SYSTEM_API_URL".to_string(), Some("The url to connect everything to. Normally is 127.0.0.1:3030 or 0.0.0.0:3030 if you want it to be accessible from everywhere".to_string()), None, Some(url.to_string()));
            return sharedtypes::ClientAPIInfo {
                url,
                authentication: None,
            };
        } else {
            panic!("This should always parse properly. local api is 127.0.0.1:3030")
        }
    }

    /// Sets up new db instance.
    pub fn new(path: Option<String>, vers: usize) -> Self {
        // Initiates two tnections to the DB. Cheap workaround to avoid loading errors.
        let mut first_time_load_flag = false;
        let mut main = match path {
            Some(ref file_path) => {
                first_time_load_flag = Path::new(&file_path).exists();
                let memdb = Arc::new(RwLock::new(NewinMemDB::new()));
                let manager = SqliteConnectionManager::memory();
                let pool = r2d2::Builder::new().max_size(200).build(manager).unwrap();
                let write_conn = Arc::new(Mutex::new({
                    let mut pool = pool.get().unwrap();
                    pool.execute_batch(
                        "PRAGMA busy_timeout = 20000;
            PRAGMA page_size = 8192;
",
                    )
                    .unwrap();

                    pool
                }));
                let write_conn_istransaction = Arc::new(Mutex::new(false));
                let memdbmain = Main {
                    _dbpath: path.clone(),
                    _vers: vers,
                    pool,
                    write_conn,
                    write_conn_istransaction,
                    _active_vers: 0,
                    _inmemdb: memdb.clone(),
                    tables_loaded: Arc::new(vec![].into()),
                    tables_loading: Arc::new(vec![].into()),
                    _cache: CacheType::Bare,
                    globalload: None,
                    localref: None,
                    api_info: Arc::new(None.into()),
                    popular_relationship_count: Arc::new(None.into()),
                };
                //                let tnection = dbinit(file_path);
                let manager = SqliteConnectionManager::file(file_path).with_init(|conn| {
                    conn.execute_batch(
                        "
            PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 20000;
PRAGMA wal_autocheckpoint = 5000;
PRAGMA mmap_size = 1073741824; -- 1GB
",
                    )?;
                    /*
                                        // Enable SQL tracing
                                        conn.trace(Some(|sql| {
                                            println!("[SQL TRACE] {}", sql);
                                        }));
                    */
                    Ok(())
                });
                let pool = r2d2::Builder::new()
                    .idle_timeout(Some(Duration::from_secs(5)))
                    .max_size(10)
                    .build(manager)
                    .unwrap();
                let write_conn = Arc::new(Mutex::new(pool.get().unwrap()));
                let write_conn_istransaction = Arc::new(Mutex::new(false));
                let main = Main {
                    _dbpath: path,
                    _vers: vers,
                    pool,
                    write_conn,
                    write_conn_istransaction,
                    _active_vers: 0,
                    _inmemdb: memdb.clone(),
                    tables_loaded: Arc::new(vec![].into()),
                    tables_loading: Arc::new(vec![].into()),
                    _cache: CacheType::InMemdb,
                    globalload: None,
                    localref: None,
                    api_info: Arc::new(None.into()),
                    popular_relationship_count: Arc::new(None.into()),
                };
                main
            }
            None => {
                first_time_load_flag = false;
                let memdb = Arc::new(RwLock::new(NewinMemDB::new()));
                let db_uuid = uuid::Uuid::new_v4();
                let memdb_uri = format!("file:memdb_{}?mode=memory&cache=shared", db_uuid);

                let manager = SqliteConnectionManager::file(&memdb_uri).with_init(|conn| {
                    conn.execute_batch(
                        "
PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA secure_delete = 0;PRAGMA busy_timeout = 20000;
            PRAGMA page_size = 8192;
            PRAGMA cache_size = -500000;

                        ",
                    )?;
                    conn.trace(Some(|sql| println!("[SQL TRACE] {}", sql)));
                    Ok(())
                });

                let pool = r2d2::Builder::new().max_size(100).build(manager).unwrap();

                // Grab a "write" connection for operations that need exclusive access
                let write_conn = Arc::new(Mutex::new(pool.get().unwrap()));
                let write_conn_istransaction = Arc::new(Mutex::new(false));

                let main = Main {
                    _dbpath: None,
                    _vers: vers,
                    pool,
                    write_conn,
                    write_conn_istransaction,
                    _active_vers: 0,
                    _inmemdb: memdb.clone(),
                    tables_loaded: Arc::new(vec![].into()),
                    tables_loading: Arc::new(vec![].into()),
                    _cache: CacheType::Bare,
                    globalload: None,
                    localref: None,
                    api_info: Arc::new(None.into()),
                    popular_relationship_count: Arc::new(None.into()),
                };
                main
            }
        };
        // let path = String::from("./main.db");
        // Sets default settings for db settings.
        if !first_time_load_flag {
            {
                let mut write_conn = main.write_conn.lock();
                let mut transaction = write_conn.transaction().unwrap();
                main.first_db(&mut transaction);

                dbg!("made first db");
                main.updatedb(&mut transaction);
                dbg!("made update db");
                transaction.commit().unwrap();
            }
            main.create_default_source_url_ns_id();
            main.load_table(&sharedtypes::LoadDBTable::Settings);
            dbg!("made loaded table db");
            main.load_caching();
            dbg!("cached  db");
        } else {
            // Database does exist.
            logging::log(format!(
                "Database Exists: {} : Skipping creation.",
                first_time_load_flag
            ));
            main.load_table(&sharedtypes::LoadDBTable::Settings);
            main.load_caching();
        }

        // Manages to load the db count for migrating any count if they've changeed into the new
        // popular_fts or relaitonship table
        {
            let mut write_conn = main.write_conn.lock();
            let mut tn = write_conn.transaction().unwrap();
            main.migrate_relationships_based_on_count(&mut tn);
            tn.commit().unwrap();
        }

        // Loads api info into the settings table
        main.api_info = Arc::new(RwLock::new(Some(main.get_api_url())));

        main.transaction_flush();
        main
    }
    /// Adds file into Memdb instance.
    pub(in crate::database) fn file_add_db(
        &self,
        tn: &Transaction,
        file: sharedtypes::DbFileStorage,
    ) -> usize {
        match self._cache {
            CacheType::Bare => self.file_add_sql(tn, &file),
            _ => {
                self.file_add_sql(tn, &file);

                self._inmemdb.write().file_put(file)
            }
        }
    }

    /// Condesnes relationships between tags & files. Changes tag id's removes spaces
    /// inbetween tag id's and their relationships.
    /// NOTE Make this an exclusive transaction otherwise we could drop data
    pub(in crate::database) fn condense_db_all_internal(&self, tn: &Transaction) {
        self.clear_empty_tags(tn);
        //self.condense_namespace();
        self.condense_tags_internal(tn);

        //self.condense_file_locations();
        //self.vacuum();
    }
    /// Wrapper for inmemdb and parents_add_internal_db
    pub(in crate::database) fn parents_add_internal(
        &self,
        tn: &Transaction,
        par: sharedtypes::DbParentsObj,
    ) -> usize {
        match self._cache {
            CacheType::Bare => {
                let tagid = self.parents_get_id_list_sql(&par);

                if tagid.is_empty() {
                    self.parents_add_sql(tn, &par)
                } else {
                    let tags: Vec<usize> = tagid.into_iter().collect();
                    tags[0]
                }
            }
            _ => {
                let inmemdb = self._inmemdb.read();
                let parent = inmemdb.parents_get(&par);
                if parent.is_none() {
                    self.parents_add_sql(tn, &par);
                }
                self.parents_add_internal_db(par)
            }
        }
    }
    ///
    /// Migrates a tag to a new tag from an old tag
    ///
    pub(in crate::database) fn migrate_relationship_tag(
        &self,
        tn: &Transaction,
        old_tag_id: &usize,
        new_tag_id: &usize,
    ) {
        match self._cache {
            CacheType::Bare => {
                self.migrate_relationship_tag_sql(tn, old_tag_id, new_tag_id);
            }
            _ => {
                let fileids = self.relationship_get_fileid(old_tag_id);
                for file_id in fileids {
                    self.add_relationship_sql(tn, &file_id, &new_tag_id);
                    self.delete_relationship_sql(tn, &file_id, old_tag_id);
                }
            }
        }
        self.parents_migration(tn, old_tag_id, new_tag_id);
    }
    ///
    /// Removes parent from db
    ///
    pub(in crate::database) fn parents_tagid_remove_internal(
        &self,
        tn: &Transaction,
        tag_id: &usize,
    ) -> HashSet<sharedtypes::DbParentsObj> {
        match self._cache {
            CacheType::Bare => {
                let out = self.parents_tagid_tag_get(tag_id);

                self.parents_delete_tag_id_sql(tn, tag_id);
                out
            }
            _ => {
                self.parents_delete_tag_id_sql(tn, tag_id);
                self._inmemdb.write().parents_tagid_remove(tag_id)
            }
        }
    }

    ///
    /// More modern way to add a file into the db
    ///
    pub(in crate::database) fn tag_add_tagobject_internal(
        &self,
        tn: &Transaction,
        tag: &sharedtypes::TagObject,
    ) -> Option<usize> {
        let mut limit_to = None;

        // If the tag is empty then dont push.
        // this is a stupid check but I don't trust myself
        if tag.tag.is_empty() || tag.namespace.name.is_empty() {
            return None;
        }

        self.tag_run_on_tag(tag);
        let nsid = self.namespace_add_namespaceobject(tn, tag.namespace.clone());
        let tag_id = self.tag_add_internal(tn, &tag.tag, nsid, None);

        // Parent tag adding
        if let Some(subtag) = &tag.relates_to {
            if subtag.tag.is_empty() || subtag.namespace.name.is_empty() {
                return None;
            }

            let nsid = self.namespace_add_namespaceobject(tn, subtag.namespace.clone());
            let relate_tag_id = self.tag_add_internal(tn, &subtag.tag, nsid, None);
            if let Some(limitto) = &subtag.limit_to {
                if limitto.tag.is_empty() || limitto.namespace.name.is_empty() {
                    return None;
                }

                let nsid = self.namespace_add_namespaceobject(tn, limitto.namespace.clone());
                limit_to = Some(self.tag_add_internal(tn, &limitto.tag, nsid, None));
            }
            let par = sharedtypes::DbParentsObj {
                tag_id,
                relate_tag_id,
                limit_to,
            };

            self.parents_add_internal(tn, par);
        }
        Some(tag_id)
    }

    ///
    /// Adds all tags to a fileid
    /// If theirs no fileid then it just adds the tag
    ///
    pub(in crate::database) fn add_tags_to_fileid_internal(
        &self,
        tn: &Transaction,
        file_id: Option<usize>,
        tag_actions: &Vec<sharedtypes::FileTagAction>,
    ) {
        for tag_action in tag_actions {
            match tag_action.operation {
                sharedtypes::TagOperation::Add => {
                    // Simple add operation
                    for tag in &tag_action.tags {
                        if matches!(
                            tag.tag_type,
                            sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex
                        ) {
                            if let Some(tag_id) = self.tag_add_tagobject_internal(tn, tag) {
                                if let Some(file_id) = file_id {
                                    self.add_relationship_sql(tn, &file_id, &tag_id);
                                }
                            }
                        }
                    }
                }

                sharedtypes::TagOperation::Set => {
                    // We need a valid file_id for Set
                    let file_id = match file_id {
                        Some(fid) => fid,
                        None => return,
                    };

                    // 1️⃣ Build parser tags grouped by namespace
                    let mut namespace_tags: HashMap<String, Vec<usize>> = HashMap::new();
                    for tag in &tag_action.tags {
                        // Ignore special tag types
                        if !matches!(
                            tag.tag_type,
                            sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex
                        ) {
                            continue;
                        }

                        if let Some(tag_id) = self.tag_add_tagobject_internal(tn, tag) {
                            namespace_tags
                                .entry(tag.namespace.name.clone())
                                .or_insert_with(Vec::new)
                                .push(tag_id);
                        }
                    }

                    // 2️⃣ Fetch current tags for file grouped by namespace
                    let mut namespace_file_tags: HashMap<String, Vec<usize>> = HashMap::new();
                    for tag_id in self.relationship_get_tagid(&file_id) {
                        if let Some(tag_obj) = self.tag_id_get(&tag_id) {
                            if let Some(namespace) = self.namespace_get_string(&tag_obj.namespace) {
                                namespace_file_tags
                                    .entry(namespace.name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(tag_id);
                            }
                        }
                    }

                    // 3️⃣ Remove ignored namespaces
                    for ignored in ["source_url", ""] {
                        namespace_tags.remove(ignored);
                        namespace_file_tags.remove(ignored);
                    }

                    // 4️⃣ Synchronize tags
                    for (namespace, parser_tags) in &namespace_tags {
                        let file_tags = namespace_file_tags.get_mut(namespace);

                        // Convert parser_tags to a HashSet for efficient lookup
                        let parser_tag_set: HashSet<_> = parser_tags.iter().copied().collect();

                        match file_tags {
                            Some(file_tags) => {
                                // a) Add new tags from parser that aren't already in file
                                for &tag_id in parser_tags {
                                    if !file_tags.contains(&tag_id) {
                                        logging::log(format!(
                                            "Adding tag_id {} to file_id {} per scraper",
                                            tag_id, file_id
                                        ));
                                        self.add_relationship_sql(tn, &file_id, &tag_id);
                                        file_tags.push(tag_id); // Update in-memory vector
                                    }
                                }

                                // b) Remove tags from file that are no longer in parser
                                let to_remove: Vec<_> = file_tags
                                    .iter()
                                    .filter(|&&tag_id| !parser_tag_set.contains(&tag_id))
                                    .copied()
                                    .collect();

                                for tag_id in to_remove {
                                    logging::log(format!(
                                        "Removing tag_id {} from file_id {} per scraper",
                                        tag_id, file_id
                                    ));
                                    self.delete_relationship_sql(tn, &file_id, &tag_id);
                                    file_tags.retain(|&id| id != tag_id); // Update in-memory vector
                                }
                            }

                            None => {
                                // Namespace doesn't exist for this file, just add all parser tags
                                for &tag_id in parser_tags {
                                    logging::log(format!(
                                        "Adding tag_id {} to file_id {} per scraper",
                                        tag_id, file_id
                                    ));
                                    self.add_relationship_sql(tn, &file_id, &tag_id);
                                }
                            }
                        }
                    }
                }

                sharedtypes::TagOperation::Del => {
                    let file_id = match file_id {
                        Some(fid) => fid,
                        None => return,
                    };
                    for tag in tag_action.tags.iter() {
                        if let Some(ns_id) = self.namespace_get(&tag.namespace.name) {
                            if let Some(tag_id) = self.tag_get_name(tag.tag.clone(), ns_id) {
                                logging::log(format!(
                                    "Removing tag_id {} to file_id {} per scraper",
                                    tag_id, file_id
                                ));

                                self.delete_relationship_sql(tn, &file_id, &tag_id);
                            }
                        }
                    }
                }
            }
        }
    }

    ///
    /// Wrapper for inmemdb
    ///
    pub(in crate::database) fn parents_reltagid_remove(
        &self,
        tn: &Transaction,
        reltag: &usize,
    ) -> HashSet<sharedtypes::DbParentsObj> {
        match self._cache {
            CacheType::Bare => {
                let out = self.parents_relate_tag_get(reltag);

                self.parents_delete_relate_tag_id_sql(tn, reltag);
                out
            }
            _ => {
                self.parents_delete_relate_tag_id_sql(tn, reltag);
                self._inmemdb.write().parents_reltagid_remove(reltag)
            }
        }
    }

    pub(in crate::database) fn parents_limitto_remove(
        &self,
        tn: &Transaction,
        limit_to: Option<usize>,
    ) -> HashSet<sharedtypes::DbParentsObj> {
        match self._cache {
            CacheType::Bare => {
                if let Some(limit_to) = limit_to {
                    let temp = self.parents_limitto_tag_get(&limit_to);

                    self.parents_delete_limit_to_sql(tn, &limit_to);
                    temp
                } else {
                    HashSet::new()
                }
            }
            _ => {
                if let Some(limit_to) = limit_to {
                    self.parents_delete_limit_to_sql(tn, &limit_to);
                    self._inmemdb.write().parents_limitto_remove(&limit_to)
                } else {
                    HashSet::new()
                }
            }
        }
    }

    /// Clears in memdb structures
    pub(in crate::database) fn clear_cache(&self) {
        match self._cache {
            CacheType::Bare => {}
            _ => {
                self._inmemdb.write().clear_all();
            }
        }
    }

    ///
    /// Gets one tnection from the sqlite pool
    ///
    pub(in crate::database) fn get_database_connection(
        &self,
    ) -> PooledConnection<SqliteConnectionManager> {
        loop {
            match self.pool.get() {
                Ok(out) => {
                    return out;
                }
                Err(_) => {
                    dbg!("a");
                }
            }
        }
    }

    ///
    /// Loads the cache configuration
    ///
    fn load_caching(&mut self) {
        let temp;

        let mut write_conn = self.write_conn.lock();
        let mut tn = write_conn.transaction().unwrap();

        loop {
            let cache = match self.settings_get_name(&"dbcachemode".into()) {
                None => {
                    self.setup_default_cache(&mut tn);
                    self.settings_get_name(&"dbcachemode".into())
                        .unwrap()
                        .param
                        .clone()
                }
                Some(setting) => setting.param.clone(),
            };

            if let Some(ref cache) = cache {
                let cachemode = match cache.as_str() {
                    "Bare" => Some(CacheType::Bare),
                    "InMemdb" => Some(CacheType::InMemdb),
                    "InMemory" => {
                        let manager = SqliteConnectionManager::memory();

                        let pool = r2d2::Builder::new()
                            .idle_timeout(Some(Duration::from_secs(5)))
                            .max_size(1)
                            .build(manager)
                            .unwrap();

                        Some(CacheType::InMemory(pool))
                    }

                    _ => {
                        self.setup_default_cache(&mut tn);
                        None
                    }
                };
                if let Some(cachemode) = cachemode {
                    temp = cachemode;
                    break;
                }
            } else {
                self.setup_default_cache(&mut tn);
            }
        }
        tn.commit().unwrap();
        self._cache = temp
    }
    ///
    /// Sets up the default location to dump misbehaving files
    ///
    pub(in crate::database) fn setup_defaut_misplaced_location(&self) -> String {
        let mut conn = self.get_database_connection();
        let loc = self.location_get();
        let outpath = Path::new(&loc);
        let out = outpath.parent().unwrap().join(Path::new("Files-Dump"));
        file::folder_make(&out.to_string_lossy().to_string());
        out.to_string_lossy().to_string()
    }

    ///
    /// Checks the relationships table for any dead tagids
    /// Only cleans the tags where they are linked to a fileid
    ///
    pub(in crate::database) fn check_relationship_tag_relations_internal(&self, tn: &Transaction) {
        self.load_table(&sharedtypes::LoadDBTable::Files);
        self.load_table(&sharedtypes::LoadDBTable::Relationship);
        self.load_table(&sharedtypes::LoadDBTable::Tags);
        let mut flag = false;
        logging::info_log("Relationship-Tag-Relations checker starting check");

        for fid in self.file_get_list_id().iter() {
            for tid in self.relationship_get_tagid(fid).iter() {
                if self.tag_id_get(tid).is_none() {
                    flag = true;
                    logging::log(format!(
                        "Relationship-Tag-Relations checker found bad tid {tid} relating to fid: {fid} deleting empty relationship"
                    ));
                    self.delete_relationship_sql(tn, fid, tid);
                }
            }
        }

        if flag {
            let mut write_conn = self.write_conn.lock();
            let mut tn = write_conn.transaction().unwrap();
            logging::info_log("Relationship-Tag-Relations checker condensing tags");
            self.condense_tags_internal(&mut tn);
            tn.commit().unwrap();
            self.vacuum();
        }
        logging::info_log("Relationship-Tag-Relations checker ending check");
        self.transaction_flush();
    }

    /// Adds the job to the inmemdb
    fn jobs_add_new_todb(&self, job: sharedtypes::DbJobsObj) {
        match self._cache {
            /*CacheType::Bare => {
                self.jobs_add_sql( &job);
            }*/
            _ => {
                self._inmemdb.write().jobref_new(job);
            }
        }
    }

    ///
    /// Flips the running of a job by id
    /// Returns the status of the job if it exists
    ///
    fn jobs_flip_running(&self, id: &usize) -> Option<bool> {
        match self._cache {
            CacheType::Bare => {
                todo!("Bare implementation not implemineted");
            }
            _ => self._inmemdb.write().jobref_flip_isrunning(id),
        }
    }

    /// File Sanity Checker This will check that the files by id will have a matching
    /// location & hash.
    fn db_sanity_check_file(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Files);
        todo!("Need to fix this files are sucky");
        let flist = self.file_get_list_id();
        /*flist.par_iter().for_each(|feach| {
            // Check is needed to support if the file with nonexistant info was gotten from db
            if let Some(filestorage_obj) = self.file_get_id( feach) {
                let fileinfo = match filestorage_obj {
                    sharedtypes::DbFileStorage::Exist(file) => file,
                    _ => {
                        panic!("Pulled item that shouldnt exist: {:?}", filestorage_obj);
                    }
                };
                loop {
                    let location = self.storage_get_string( &fileinfo.storage_id).unwrap();
                    let temppath = &format!("{}/{}", location, fileinfo.hash);
                    if Path::new(temppath).exists() {
                        let fil = std::fs::read(temppath).unwrap();
                        let hinfo = download::hash_bytes(
                            &bytes::Bytes::from(fil),
                            &sharedtypes::HashesSupported::Sha512(fileinfo.hash.clone()),
                        );
                        if !hinfo.1 {
                            dbg!(format!(
                                "BAD HASH: ID: {}  HASH: {}   2ND HASH: {}",
                                fileinfo.id, fileinfo.hash, hinfo.0
                            ));
                        }
                    }
                }
            } else {
                dbg!(format!("File ID: {} Does not exist.", &feach));
            }
        });*/
    }

    ///
    ///Returns the max id of something inside of the db
    ///
    fn tags_max_id(&self) -> usize {
        match self._cache {
            CacheType::Bare => self.tags_max_return_sql(),
            _ => self._inmemdb.read().tags_max_return(),
        }
    }

    /// Returns next jobid from _inmemdb
    pub(in crate::database) fn jobs_get_max(&self) -> usize {
        match self._cache {
            //  CacheType::Bare => self.jobs_return_count_sql(),
            _ => *self._inmemdb.read().jobs_get_max(),
        }
    }

    /// Sets up first database interaction. Makes tables and does first time setup.
    fn first_db(&self, tn: &Transaction) {
        {
            // Making Tags Table
            let mut name = "Tags".to_string();
            let mut keys = vec_of_strings!["id", "name", "namespace"];
            let mut vals = vec_of_strings![
                "INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL ",
                "TEXT NOT NULL",
                "INTEGER NOT NULL, UNIQUE(name, namespace)"
            ];

            self.tag_create_v1(tn);

            // Making Parents Table. Relates tags to tag parents.
            self.parents_create_v2(tn);

            // Making Namespace Table
            name = "Namespace".to_string();
            keys = vec_of_strings!["id", "name", "description"];
            vals = vec_of_strings![
                "INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL",
                "TEXT NOT NULL UNIQUE",
                "TEXT"
            ];
            self.table_create(tn, &name, &keys, &vals);

            // Making Settings Table
            name = "Settings".to_string();
            keys = vec_of_strings!["name", "pretty", "num", "param"];
            vals = vec_of_strings!["TEXT PRIMARY KEY", "TEXT", "INTEGER", "TEXT"];
            self.table_create(tn, &name, &keys, &vals);

            // Making Jobs Table
            name = "Jobs".to_string();
            keys = vec_of_strings!(
                "id",
                "time",
                "reptime",
                "priority",
                "cachetime",
                "cachechecktype",
                "Manager",
                "site",
                "param",
                "SystemData",
                "UserData"
            );
            vals = vec_of_strings!(
                "INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER",
                "INTEGER NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL"
            );

            self.table_create(tn, &name, &keys, &vals);

            {
                tn.execute(
                "CREATE TABLE File 
            (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, hash TEXT, extension INTEGER, storage_id INTEGER, 
                CHECK (
                    (hash IS NOT NULL AND extension IS NOT NULL) OR
                    (hash IS NULL AND extension IS NULL)
                )
            )",
                [],
            )
            .unwrap();

                self.relationship_create_v3(tn);

                self.migrate_relationships_based_on_count(tn);

                tn.execute("CREATE INDEX idx_namespace ON Namespace (name)", [])
                    .unwrap();
            }

            self.enclave_create_database_v5(tn);

            // Making dead urls Table
            name = "dead_source_urls".to_string();
            keys = vec_of_strings!["id", "dead_url"];
            vals = vec_of_strings!["INTEGER PRIMARY KEY", "TEXT NOT NULL"];
            self.table_create(tn, &name, &keys, &vals);
            {
                tn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_file_hash ON File (hash)",
                    [],
                )
                .unwrap();
            }
        }
        self.tags_fts_create_v2(tn);
        self.namespace_properties_create_v1(tn);

        let count = self.get_relationship_popular_division_count(tn);

        // Setus up triggers for the migration of popular tags
        self.migrate_relationship_popular_count(tn, &922372036854775807, &count);
    }

    fn updatedb(&self, tn: &Transaction) {
        self.setting_add_internal(
            tn,
            "DBCOMMITNUM".to_string(),
            Some("Number of transactional items before pushing to db.".to_string()),
            Some(3000),
            None,
        );
        self.setting_add_internal(
            tn,
            "VERSION".to_string(),
            Some("Version that the database is currently on.".to_string()),
            Some(self._vers),
            None,
        );
        self.setting_add_internal(tn, "DEFAULTRATELIMIT".to_string(), None, Some(5), None);
        self.setting_add_internal(
            tn,
            "FilesLoc".to_string(),
            None,
            None,
            Some("Files".to_string()),
        );
        self.setting_add_internal(
            tn,
            "DEFAULTUSERAGENT".to_string(),
            None,
            None,
            Some("DIYHydrus/1.0".to_string()),
        );
        self.setting_add_internal(
            tn,
            "pluginloadloc".to_string(),
            Some("Where plugins get loaded into.".to_string()),
            None,
            Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
        );

        self.setting_add_internal(
            tn,
            "scraperloadloc".to_string(),
            Some("Where scrapers get loaded into.".to_string()),
            None,
            Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
        );

        self.setup_default_cache(tn);

        self.enclave_create_default_file_download(tn, self.location_get());
    }

    ///
    /// Default caching option for the db
    ///
    fn setup_default_cache(&self, tn: &Transaction) {
        self.setting_add_internal(
            tn,
            "dbcachemode".to_string(),
            Some("The database caching options. Supports: Bare, InMemdb and InMemory".to_string()),
            None,
            Some("Bare".to_string()),
        );
    }
    /// Checks if db version is consistent. If this function returns false signifies
    /// that we shouldn't run.
    pub fn check_version(&mut self) -> bool {
        let mut query_string = "SELECT num FROM Settings WHERE name='VERSION';";
        let query_string_manual = "SELECT num FROM Settings_Old WHERE name='VERSION';";
        let mut g1 = self.quer_int(query_string.to_string());
        if g1.len() != 1 {
            error!(
                "Could not check_version due to length of recieved version being less then one. Trying manually!!!"
            );

            // let out = self.execute("SELECT num from Settings WHERE
            // name='VERSION';".to_string());
            let tn = self.get_database_connection();
            let mut toexec = tn.prepare(query_string).unwrap();
            let mut rows = toexec.query(params![]).unwrap();
            g1.clear();
            while let Some(each) = rows.next().unwrap() {
                let ver: Result<String> = each.get(0);
                let vers: Result<usize> = each.get(0);

                // let izce;
                let izce = match &ver {
                    Ok(_string_ver) => ver.unwrap().parse::<usize>().unwrap(),
                    Err(_unk_err) => vers.unwrap(),
                };
                g1.push(izce.try_into().unwrap())
            }
        }
        if g1.len() != 1 {
            error!("Manual loading failed. Trying from old table.");
            println!("Manual loading failed. Trying from old table.");
            query_string = query_string_manual;
            let tn = self.get_database_connection();
            let mut toexec = tn.prepare(query_string).unwrap();
            let mut rows = toexec.query(params![]).unwrap();
            g1.clear();
            while let Some(each) = rows.next().unwrap() {
                let ver: String = each.get(0).unwrap();

                // let vers = ver.try_into().unwrap();
                let izce = ver.parse().unwrap();
                g1.push(izce)
            }
            logging::panic_log("check_version: Could not load DB properly PANICING!!!".to_string());
        }
        let mut db_vers = g1[0] as usize;
        self._active_vers = db_vers;
        logging::info_log(format!("check_version: Loaded version {}", db_vers));
        if self._active_vers != self._vers {
            let mut conn = self.get_database_connection();
            conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            logging::info_log(format!(
                "Starting upgrade from V{} to V{}",
                db_vers,
                db_vers + 1
            ));

            // Resets the DB internal Cache if their is any.
            self.clear_cache();
            self.load_table(&sharedtypes::LoadDBTable::Settings);

            self.transaction_flush();
            if db_vers == 1 {
                panic!("How did you get here vers is 1 did you do something dumb??")
            } else if db_vers == 2 {
                dbg!(self._vers, self._active_vers);
                self.db_update_two_to_three();
                db_vers += 1;
            } else if db_vers == 3 {
                self.db_update_three_to_four();
            } else if db_vers == 4 {
                self.db_update_four_to_five();
            } else if db_vers == 5 {
                self.db_update_five_to_six();
            } else if db_vers == 6 {
                self.db_update_six_to_seven();
            } else if db_vers == 7 {
                self.db_update_seven_to_eight();
            } else if db_vers == 8 {
                self.db_update_eight_to_nine();
            } else if db_vers == 9 {
                self.db_update_nine_to_ten();
            } else if db_vers == 10 {
                self.db_update_ten_to_eleven();
            }

            logging::info_log(format!("Finished upgrade to V{}.", db_vers));
            self.transaction_flush();
            if db_vers == self._vers {
                logging::info_log(format!("Successfully updated db to version {}", self._vers));
                return true;
            }
        } else {
            info!("Database Version is: {}", g1[0]);
            return true;
        }
        false
    }

    pub(in crate::database) fn check_default_source_urls_internal(
        &self,
        tn: &Transaction,
        action: &sharedtypes::CheckSourceUrlsEnum,
    ) {
        self.load_table(&sharedtypes::LoadDBTable::Namespace);
        self.load_table(&sharedtypes::LoadDBTable::Tags);
        let source_nsid = self.create_default_source_url_ns_id();

        for tag_id in self.namespace_get_tagids(&source_nsid).iter() {
            if let Some(tag) = self.tag_id_get(tag_id) {
                if !check_url(&tag.name) {
                    match action {
                        sharedtypes::CheckSourceUrlsEnum::Print => {
                            logging::info_log(format!(
                                "Tagid - {} name - {} doesn't look like a valid url",
                                tag_id, &tag.name
                            ));
                        }
                        sharedtypes::CheckSourceUrlsEnum::Delete => {
                            self.load_table(&sharedtypes::LoadDBTable::All);
                            self.delete_tag_sql(tn, tag_id);
                        }
                    }
                }
            }
        }
    }
    ///
    /// Adds a namespace into the db if it may or may not exist
    ///
    pub(in crate::database) fn namespace_add_internal(
        &self,
        tn: &Transaction,
        name: &String,
        description: &Option<String>,
    ) -> usize {
        if let Some(id) = self.namespace_get(name) {
            return id;
        }

        match self._cache {
            CacheType::Bare => self.namespace_add_sql(tn, name, description, None),
            _ => {
                let out = self.namespace_add_inmemdb(tn, name.clone(), description.clone());

                out
            }
        }
    }

    /// Creates a table name: The table name key: List of Collumn lables. dtype: List
    /// of Collumn types. NOTE Passed into SQLITE DIRECTLY THIS IS BAD :C
    pub(in crate::database) fn table_create(
        &self,
        tn: &Transaction,
        name: &String,
        key: &[String],
        dtype: &[String],
    ) {
        // Sanity checking...
        assert_eq!(
            key.len(),
            dtype.len(),
            "Warning table create was 2 Vecs weren't balanced. Lengths: {} {}",
            key.len(),
            dtype.len()
        );

        // Not sure if theirs a better way to dynamically allocate a string based on two
        // vec strings at run time. Let me know if im doing something stupid.
        let mut concat = true;
        let mut c = 0;
        let mut stocat = "".to_string();
        while concat {
            let ke = &key[c];
            let dt = &dtype[c];
            stocat = [
                stocat,
                ke.to_string(),
                " ".to_string(),
                dt.to_string(),
                ", ".to_string(),
            ]
            .concat();
            c += 1;
            if c >= key.len() - 1 {
                concat = false;
            }
        }
        let ke = &key[key.len() - 1];
        let dt = &dtype[dtype.len() - 1];
        let endresult = [
            "CREATE TABLE IF NOT EXISTS ".to_string(),
            name.to_string(),
            " (".to_string(),
            stocat,
            ke.to_string(),
            " ".to_string(),
            dt.to_string(),
            ");".to_string(),
        ]
        .concat();
        dbg!(&endresult);
        info!("Creating table as: {}", endresult);
        tn.execute_batch(&endresult).unwrap();
        //stocat = endresult;
        //self.execute(stocat);
    }

    /// Alters a tables name
    pub(super) fn alter_table(&self, original_table: &String, new_table: &String) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        tn.execute(
            "ALTER TABLE ? RENAME TO ?",
            params![original_table, new_table],
        )
        .unwrap();
        tn.commit().unwrap();
    }

    /// Checks if table exists in DB if it do then delete.
    pub(super) fn check_table_exists(&self, table: String) -> bool {
        let mut out = false;
        let query_string = "SELECT * FROM sqlite_master WHERE type='table' AND name=? ;";
        dbg!("setup");
        let tn = self.get_database_connection();
        dbg!("connection");
        dbg!(&self._cache);
        dbg!(&table);
        dbg!(self.pool.state());
        //panic!();
        let mut toexec = tn.prepare(query_string).unwrap();
        dbg!("query");
        let mut rows = toexec.query(params![table]).unwrap();
        dbg!("rows");
        if let Some(_each) = rows.next().unwrap() {
            out = true;
        }
        out
    }

    /// Gets the names of a collumn in a table
    pub(super) fn db_table_collumn_getnames(&self, table: &String) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        {
            let tn = self.get_database_connection();
            let stmt = tn
                .prepare(&format!("SELECT * FROM {} LIMIT 1", table))
                .unwrap();
            for collumn in stmt.column_names() {
                out.push(collumn.to_owned());
            }
        }
        out
    }

    /// Sets DB Version
    pub(super) fn db_version_set(&mut self, version: usize) {
        logging::log(format!("Setting DB Version to: {}", &version));
        self._active_vers = version;

        let mut write_conn = self.write_conn.lock();
        let mut tn = write_conn.transaction().unwrap();

        self.setting_add_internal(
            &mut tn,
            "VERSION".to_string(),
            Some("Version that the database is currently on.".to_string()),
            Some(version),
            None,
        );
        tn.commit();
    }

    pub(super) fn db_drop_table(&self, table: &String) {
        let tn = self.write_conn.lock();
        let query_string = format!("DROP TABLE IF EXISTS {};", table);
        let mut toexec = tn.prepare(&query_string).unwrap();
        toexec.execute(params![]).unwrap();
    }

    /// NOTE USES PASSED tnECTION FROM FUNCTION NOT THE DB CONNECTION GETS ARROUND
    /// MEMROY SAFETY ISSUES WITH CLASSES IN RUST
    pub(in crate::database) fn load_files(&self) {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }
    }

    ///
    /// Gets an extension id and creates it if it does not exist
    ///
    pub(in crate::database) fn extension_put_string_internal(
        &self,
        tn: &Transaction,
        ext: &String,
    ) -> usize {
        match self.extension_get_id_sql(tn, ext) {
            Some(id) => id,
            None => self.extension_put_id_ext_sql(tn, None, ext),
        }
    }

    pub fn extension_put_string(&self, ext: &String) -> usize {
        if let Some(out) = self.extension_get_id(ext) {
            return out;
        }

        let out = self.extension_put_string(ext);
        out
    }

    ///
    /// Gets an ID if a extension string exists
    ///
    pub fn extension_get_id(&self, ext: &String) -> Option<usize> {
        let mut db = self.get_database_connection();
        let tn = db.transaction().unwrap();
        match self._cache {
            CacheType::Bare => self.extension_get_id_sql(&tn, ext),
            _ => self._inmemdb.read().extension_get_id(ext).copied(),
        }
    }

    /// Puts extension into mem cache
    pub(in crate::database) fn extension_load(&self, tn: &Transaction, id: usize, ext: String) {
        self.extension_put_id_ext_sql(tn, Some(id), &ext);
    }

    /// Same as above
    pub(in crate::database) fn load_namespace(&self) {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }

        let mut nses: Vec<sharedtypes::DbNamespaceObj> = vec![];
        logging::info_log("Database is Loading: Namespace".to_string());
        {
            let tn = self.get_database_connection();
            let temp = tn.prepare("SELECT * FROM Namespace");
            if let Ok(mut con) = temp {
                let namespaces = con
                    .query_map([], |row| {
                        Ok(sharedtypes::DbNamespaceObj {
                            id: row.get(0).unwrap(),
                            name: row.get(1).unwrap(),
                            description: row.get(2).unwrap(),
                        })
                    })
                    .unwrap();
                for each in namespaces {
                    if let Ok(res) = each {
                        nses.push(res);
                    } else {
                        error!("Bad Namespace cant load {:?}", each);
                    }
                }
            }
        }

        for ns in nses {
            self.namespace_add_id_exists(ns);
            //self.namespace_add(res.name, res.description);
        }
    }

    /// Loads jobs in from DB tnection
    pub(in crate::database) fn load_jobs(&self) {
        logging::info_log("Database is Loading: Jobs".to_string());
        let tn = self.get_database_connection();
        let temp = tn.prepare("SELECT * FROM Jobs");
        if let Ok(mut con) = temp {
            let jobs = con
                .query_map([], |row| {
                    let id = row.get(0).unwrap();
                    let time = row.get(1).unwrap();
                    let reptime = row.get(2).unwrap();
                    let priority = row.get(3).unwrap_or(sharedtypes::DEFAULT_PRIORITY);
                    let cachetime = row.get(4).unwrap_or_default();
                    //let cachechecktype: String = row.get(5).unwrap();
                    let cachechecktype = "TimeReptimeParam".to_string();
                    let manager: String = row.get(6).unwrap();
                    let man = serde_json::from_str(&manager).unwrap();
                    let site = row.get(7).unwrap();
                    let param: String = row.get(8).unwrap();
                    let system_data_string: String = row.get(9).unwrap();
                    let user_data_string: String = row.get(10).unwrap();
                    let system_data = serde_json::from_str(&system_data_string).unwrap();
                    let user_data = serde_json::from_str(&user_data_string).unwrap();
                    Ok(sharedtypes::DbJobsObj {
                        id,
                        time,
                        reptime,
                        priority,
                        cachetime,
                        cachechecktype: DEFAULT_CACHECHECK,
                        //cachechecktype: serde_json::from_str(&cachechecktype).unwrap(),
                        site,
                        param: serde_json::from_str(&param).unwrap(),
                        jobmanager: man,
                        isrunning: false,
                        user_data,
                        system_data,
                    })
                })
                .unwrap();
            for each in jobs {
                if let Ok(res) = each {
                    self.jobs_add_new_todb(res);
                } else {
                    error!("Bad Job cant load {:?}", each);
                }
            }
        }
    }
    ///
    /// Gets a fileid from a hash
    ///
    pub fn file_get_hash_internal(&self, tn: &Transaction, hash: &String) -> Option<usize> {
        match self._cache {
            CacheType::Bare => self.file_get_id_sql_internal(tn, hash),
            _ => self._inmemdb.read().file_get_hash(hash).copied(),
        }
    }

    /// Adds a file into the db sqlite. Do this first.
    pub(in crate::database) fn file_add_internal(
        &self,
        tn: &Transaction,
        file: &sharedtypes::DbFileStorage,
    ) -> usize {
        match file {
            sharedtypes::DbFileStorage::Exist(file_obj) => {
                if self.file_get_hash_internal(tn, &file_obj.hash).is_none() {
                    self.file_add_sql(tn, file)
                } else {
                    file_obj.id
                }
            }
            sharedtypes::DbFileStorage::NoIdExist(noid_obj) => {
                if self.file_get_hash_internal(tn, &noid_obj.hash).is_none() {
                    self.file_add_sql(tn, file)
                } else {
                    self.file_get_hash(&noid_obj.hash).unwrap()
                }
            }
            sharedtypes::DbFileStorage::NoExist(_) => {
                panic!();
                /*let id = self.file_add_db(file.clone());
                if addtodb {
                    self.file_add_sql(&None, &None, &None, &id);
                }
                id*/
            }
            sharedtypes::DbFileStorage::NoExistUnknown => {
                panic!();
                /*let id = self.file_add_db(file.clone());
                if addtodb {
                    self.file_add_sql(&None, &None, &None, &id);
                }
                id*/
            }
        }
    }

    /// Adds namespace into DB. Returns the ID of the namespace.
    pub(super) fn namespace_add_inmemdb(
        &self,
        tn: &Transaction,
        name: String,
        description: Option<String>,
    ) -> usize {
        let namespace_grab = {
            let inmemdb = self._inmemdb.read();
            inmemdb.namespace_get(&name).copied()
        };
        match namespace_grab {
            None => {}
            Some(id) => return id.to_owned(),
        }

        let ns_id = self._inmemdb.read().namespace_get_max();
        let ns = sharedtypes::DbNamespaceObj {
            id: ns_id,
            name,
            description,
        };
        if namespace_grab.is_none() {
            self.namespace_add_sql(tn, &ns.name, &ns.description, Some(ns_id));
        }
        //self.namespace_add_db(ns)
        self._inmemdb.write().namespace_put(ns)
    }

    /// Wrapper for inmemdb adding
    pub(in crate::database) fn parents_add_internal_db(
        &self,
        parent: sharedtypes::DbParentsObj,
    ) -> usize {
        self._inmemdb.write().parents_put(parent)
    }

    /// Prints db info
    fn debugdb(&self) {
        self._inmemdb.read().dumpe_data();
    }

    fn namespace_add_namespaceobject(
        &self,
        tn: &Transaction,
        namespace_obj: sharedtypes::GenericNamespaceObj,
    ) -> usize {
        self.namespace_add_internal(tn, &namespace_obj.name, &namespace_obj.description)
    }

    /*///
    /// Adds tags to a file id
    ///
    fn relationship_tag_add(&self,tn: &Transaction, fid: &usize, tags: Vec<sharedtypes::TagObject>) {
        match self._cache {
            CacheType::Bare => {
                self.file_tag_relationship(tn, &fid, tags);
            }
            _ => {
                for tag in tags.iter() {
                    if let Some(ref tid) = self.tag_add_tagobject(tag) {
                        self.relationship_add_sql(tn, fid, tid);
                    }
                }
            }
        }
    }*/

    /// Runs regex mostly when a tag gets added
    fn tag_run_on_tag(&self, tag: &sharedtypes::TagObject) {
        if tag.tag_type != sharedtypes::TagType::NormalNoRegex {
            if let Some(ref globalload) = self.globalload {
                globalload.plugin_on_tag(tag);
            }
        }
    }

    /// Adds tag into DB if it doesn't exist in the memdb.
    pub(in crate::database) fn tag_add_internal(
        &self,
        tn: &Transaction,
        tags: &String,
        namespace: usize,
        id: Option<usize>,
    ) -> usize {
        let tag = self.tag_get_name(tags.to_string(), namespace);
        match tag {
            None => self.tag_add_no_id_sql(tn, tags, namespace),
            Some(id) => id,
        }
    }

    /// Wrapper for inmemdb relationship adding
    pub(super) fn relationship_add_db(&self, file: usize, tag: usize) {
        self._inmemdb.write().relationship_add(file, tag);
    }

    ///
    /// Adds a job into a DB.
    /// Going to make a better job adding function
    ///
    pub fn jobs_add(
        &self,

        id: Option<usize>,
        time: usize,
        reptime: usize,
        priority: usize,
        cachetime: Option<usize>,
        cachechecktype: sharedtypes::JobCacheType,
        site: String,
        param: Vec<ScraperParam>,
        system_data: BTreeMap<String, String>,
        user_data: BTreeMap<String, String>,
        jobsmanager: sharedtypes::DbJobsManager,
    ) -> usize {
        let id = match id {
            None => self.jobs_get_max(),
            Some(id) => id,
        };

        for each in self.jobs_get_all() {
            if time == each.1.time
                && reptime == each.1.reptime
                && site == each.1.site
                && param == each.1.param
            {
                return id;
            }
        }

        let jobs_obj: sharedtypes::DbJobsObj = sharedtypes::DbJobsObj {
            id: Some(id),
            time,
            reptime,
            priority,
            cachetime,
            cachechecktype,
            site: site.clone(),
            param: param.clone(),
            jobmanager: jobsmanager.clone(),
            isrunning: false,
            system_data: system_data.clone(),
            user_data: user_data.clone(),
        };
        self.jobs_update_db(jobs_obj.clone());

        // let jobsmanager = sharedtypes::DbJobsManager { jobtype: *dbjobtype, recreation:
        // None, additionaldata: None, };
        id
    }
    /// Adds global load into db
    pub fn setup_globalload(&mut self, globalload: GlobalLoad) {
        self.globalload = Some(globalload.into());
    }

    /// Wrapper for inmemdb insert.
    pub(in crate::database) fn setting_add_internal_db(
        &self,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        self._inmemdb.write().settings_add(name, pretty, num, param);
    }

    /// Returns db location as String refernce.
    pub(in crate::database) fn get_db_loc(&self) -> Option<String> {
        self._dbpath.clone()
    }

    /// Parses data from search query to return an id
    fn search_for_namespace(&self, search: &sharedtypes::DbSearchObject) -> Option<usize> {
        match &search.namespace {
            None => search.namespace_id,
            Some(id_string) => self.namespace_get(id_string),
        }
    }

    /*/// Raw Call to database. Try to only use internally to this file only. Doesn't
    /// support params nativly. Will not write changes to DB. Have to call write().
    /// Panics to help issues.
    fn execute(&self, inp: String) -> usize {
        let tn = self.conn.lock();
        let out = tn.execute(&inp, params![]);
        match out {
            Err(_out) => {
                println!("SQLITE STRING:: {}", inp);
                println!("BAD CALL {}", _out);
                error!("BAD CALL {}", _out);
                panic!("BAD CALL {}", _out);
            }
            Ok(_out) => _out,
        }
    }*/

    /*/// Deletes an item from jobs table. critera is the searchterm and collumn is the
    /// collumn to target. Doesn't remove from inmemory database
    fn del_from_jobs_table(&self, job: &sharedtypes::JobScraper) {
        self.del_from_jobs_table_sql(&job.site, &job.param);
    }*/

    /// Removes job from inmemdb Removes by id
    pub(in crate::database) fn del_from_jobs_inmemdb(&self, id: &usize) {
        self._inmemdb.write().jobref_remove(id)
    }

    ///
    /// Migrates one tag per file to a new tagid
    ///
    pub(in crate::database) fn migrate_relationship_file_tag_internal(
        &self,
        tn: &Transaction,
        file_id: &usize,
        old_tag_id: &usize,
        new_tag_id: &usize,
    ) {
        match self._cache {
            CacheType::Bare => {
                self.migrate_relationship_file_tag_sql(tn, file_id, old_tag_id, new_tag_id);
            }
            _ => {
                self.add_relationship_sql(tn, file_id, new_tag_id);
                self.delete_relationship_sql(tn, file_id, old_tag_id);
            }
        }
        self.parents_migration(tn, old_tag_id, new_tag_id);
    }

    ///
    /// Migrates a tag from one ID to another
    /// NOTE Make this an exclusive transaction otherwise we could drop data
    ///
    pub(in crate::database) fn migrate_tag_internal(
        &self,
        tn: &Transaction,
        old_tag_id: &usize,
        new_tag_id: &usize,
    ) {
        if !matches!(self._cache, CacheType::InMemdb) {
            tn.execute(
                "UPDATE Tags SET id = ? WHERE id = ?",
                params![new_tag_id, old_tag_id],
            )
            .unwrap();
            return;
        }
        let tag = match self.tag_id_get(old_tag_id) {
            Some(out) => out,
            None => {
                return;
            }
        };

        if self.tag_id_get(new_tag_id).is_none() {
            panic!(
                "Old tagid {},new tagid {} new tagid already exists cannot delete",
                old_tag_id, new_tag_id
            );
            return;
        } else {
            self.tag_add_internal(tn, &tag.name.clone(), tag.namespace, Some(*new_tag_id));
        }

        logging::log(format!("Moving tagid: {} to {}", old_tag_id, new_tag_id));
        self.parents_migration(tn, old_tag_id, new_tag_id);
        self.migrate_relationship_tag(tn, old_tag_id, new_tag_id);
        self.delete_tag_sql(tn, old_tag_id);
    }

    /// Removes tag & relationship from db.
    fn delete_tag_relationship(&self, tn: &Transaction, tag_id: &usize) {
        let relationships = self.relationship_get_fileid(tag_id);

        // Gets list of fileids from internal db.

        for _fileids in relationships.iter() {
            logging::log(format!(
                "Found {} relationships's effected for tagid: {}.",
                relationships.len(),
                tag_id
            ));

            // let mut sql = String::new();
            for file_id in relationships.clone() {
                logging::log(format!(
                    "Removing file: {} tagid: {} from db.",
                    file_id, tag_id
                ));

                logging::log("Removing relationship sql".to_string());

                self.delete_relationship_sql(tn, &file_id, tag_id);
            }

            // self.tn.lock().execute_batch(&sql).unwrap();
            logging::log("Relationship Loop".to_string());
        }
    }

    ///
    /// Migrates a parent from a old tagid to a new id
    ///
    fn parents_migration(&self, tn: &Transaction, old_tag_id: &usize, new_tag_id: &usize) {
        // Removes parent by ID and readds it with the new id
        for parent in self.parents_tagid_remove_internal(tn, old_tag_id) {
            logging::log(format!(
                "T Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                parent.tag_id,
                parent.relate_tag_id,
                parent.limit_to,
                new_tag_id,
                parent.relate_tag_id,
                parent.limit_to
            ));
            let par = sharedtypes::DbParentsObj {
                tag_id: *new_tag_id,
                relate_tag_id: parent.relate_tag_id,
                limit_to: parent.limit_to,
            };
            self.parents_add_internal(tn, par);
        }

        // Removes parent by ID and readds it with the new id
        for parent in self.parents_reltagid_remove(tn, old_tag_id) {
            logging::log(format!(
                "R Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                parent.tag_id,
                parent.relate_tag_id,
                parent.limit_to,
                parent.tag_id,
                new_tag_id,
                parent.limit_to
            ));
            let par = sharedtypes::DbParentsObj {
                tag_id: parent.tag_id,
                relate_tag_id: *new_tag_id,
                limit_to: parent.limit_to,
            };
            self.parents_add_internal(tn, par);
        }
        // Kinda hacky but nothing bad will happen if we have nothing in the limit
        // slot
        for parent in self.parents_limitto_remove(tn, Some(*old_tag_id)) {
            logging::log(format!(
                "L Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                parent.tag_id,
                parent.relate_tag_id,
                parent.limit_to,
                parent.tag_id,
                parent.relate_tag_id,
                new_tag_id
            ));
            let par = sharedtypes::DbParentsObj {
                tag_id: parent.tag_id,
                relate_tag_id: parent.relate_tag_id,
                limit_to: Some(*new_tag_id),
            };
            self.parents_add_internal(tn, par);
        }
    }

    /// Deletes namespace by id Removes tags & relationships assocated.
    pub(in crate::database) fn namespace_delete_id(&self, tn: &Transaction, id: &usize) {
        logging::info_log(format!("Starting deletion work on namespace id: {}", id));

        // self.vacuum(); self.tn.lock().execute("create index ffid on
        // Relationship(fileid);", []);
        if self.namespace_get_string(id).is_none() {
            logging::info_log("Stopping because I cannot get ns string.".to_string());
            return;
        }
        let tagids = self.namespace_get_tagids(id);
        for each in tagids.clone().iter() {
            self.delete_tag_sql(tn, each);
        }

        // elf.tn.lock().execute_batch(&tag_sql).unwrap();
        self._inmemdb.write().namespace_delete(id);
        self.delete_namespace_sql(id);
    }

    ///
    /// Condenses and corrects file locations
    ///
    fn condense_file_locations(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Files);

        let mut file_id_list: Vec<usize> = self.file_get_list_id().into_iter().collect();

        file_id_list.par_sort_unstable();

        for (ref cnt, key) in file_id_list.iter().enumerate() {
            if cnt != key {
                dbg!("mismatch", &key, &cnt);
            }
        }
    }

    fn get_starting_tag_id(&self) -> usize {
        let mut out = None;

        for id in 0..self.tags_max_id() {
            if self.tag_id_get(&id).is_some() {
                out = Some(id);
                break;
            }
        }

        if out.is_none() {
            out = Some(0);
        }

        out.unwrap()
    }

    /// Removes all empty tags in the db
    fn clear_empty_tags(&self, tn: &Transaction) {
        logging::info_log("Starting to clear any unlinked tags from the db");
        let empty_tags = self.get_empty_tagids();
        logging::info_log(format!("Found {} empty tags will clear.", empty_tags.len()));
        for tag_id in empty_tags.iter() {
            self.delete_tag_sql(tn, tag_id);
        }
    }

    ///
    /// Condenses tag ids into a solid column
    /// NOTE Make this an exclusive transaction otherwise we could drop data
    ///
    pub(in crate::database) fn condense_tags_internal(&self, tn: &Transaction) {
        self.load_table(&sharedtypes::LoadDBTable::Tags);

        // Stopping automagically updating the db

        let tag_max = self.tags_max_id();

        let mut flag = false;

        logging::info_log("Starting preliminary tags scanning".to_string());
        for id in self.get_starting_tag_id()..tag_max - 1 {
            if self.tag_id_get(&id).is_none() {
                logging::info_log(format!(
                    "Disjointed tags detected. initting fixing badid: {}",
                    &id
                ));
                flag = true;
                break;
            }
        }

        if flag {
            logging::info_log("Started loading files, relationship and parents table".to_string());
            self.load_table(&sharedtypes::LoadDBTable::Files);
            self.load_table(&sharedtypes::LoadDBTable::Relationship);
            self.load_table(&sharedtypes::LoadDBTable::Parents);
            flag = false;
        }

        let mut cnt: usize = self.get_starting_tag_id();
        let mut last_highest: usize = cnt;
        let mut eta = Eta::new(tag_max - self.get_starting_tag_id(), TimeAcc::MILLI);
        let count_percent = tag_max.div_ceil(100 / 2);
        for id in self.get_starting_tag_id()..tag_max + 1 {
            let tagnns = self.tag_id_get(&id);
            match tagnns {
                None => {
                    flag = true;
                }
                Some(tag) => {
                    if flag {
                        logging::log(format!("Migrating tag id {id} to {cnt}"));
                        self.migrate_tag_internal(tn, &id, &cnt);
                        last_highest = cnt;
                    }
                    cnt += 1;
                }
            }
            eta.step();
            if (cnt % count_percent) == 0 || cnt == 5 || cnt == tag_max {
                logging::info_log(format!("{}", &eta));
            }
        }

        // Updating internal caches highest tag value
        match self._cache {
            CacheType::Bare => {}
            _ => {
                self._inmemdb.write().tag_max_set(last_highest + 1);
            }
        }
    }

    fn condense_namespace(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Namespace);
        self.load_table(&sharedtypes::LoadDBTable::Tags);

        //self.namespace_delete_id(&2);

        for namespace_id in self.namespace_keys() {
            dbg!(&namespace_id);
            // if self.namespace_get_tagids(&namespace_id).is_empty() {
            //     dbg!(&namespace_id);
            // }
        }
    }

    /*/// Recreates the db with only one ns in it.
    fn drop_recreate_ns(&self, id: &usize) {
        // self.load_table(&sharedtypes::LoadDBTable::Relationship);
        // self.load_table(&sharedtypes::LoadDBTable::Parents);
        // self.load_table(&sharedtypes::LoadDBTable::Tags);
        self.db_drop_table(&"Relationship".to_string());
        self.db_drop_table(&"Tags".to_string());
        self.db_drop_table(&"Parents".to_string());

        // Recreates tables with new defaults
        self.first_db();
        self.transaction_flush();

        // let tag_max = self._inmemdb.tags_max_return().clone();
        self._inmemdb.tags_max_reset();
        let tida = self.namespace_get_tagids(id);
        for (cnt, tid) in tida.iter().enumerate() {
            let file_listop = self._inmemdb.relationship_get_fileid(tid).clone();
            let tag = self._inmemdb.tags_get_data(tid).unwrap().clone();
            self.tag_add_sql(&cnt, &tag.name, &tag.namespace);
            for file in file_listop {
                self.relationship_add_sql(file, cnt);
            }
        }
        self._inmemdb.tags_clear();
        self._inmemdb.parents_clear();
        self._inmemdb.relationships_clear();
        self.vacuum();
        self.transaction_flush();

        // self.condese_relationships_tags();
        self.transaction_flush();
    }*/

    fn print_all_tables(&self) -> Result<()> {
        use rusqlite::Row;
        let conn = self.write_conn.lock();
        // Step 1: Get all table names
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';",
        )?;

        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        for table in table_names {
            println!("Table: {}", table);
            println!("Columns and constraints:");

            // Step 2: Get columns and constraints
            let mut col_stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
            let columns = col_stmt.query_map([], |row: &Row| {
                let cid: i32 = row.get(0)?;
                let name: String = row.get(1)?;
                let dtype: String = row.get(2)?;
                let notnull: i32 = row.get(3)?; // 1 if NOT NULL
                let dflt_value: Option<String> = row.get(4)?;
                let pk: i32 = row.get(5)?; // 1 if part of PK
                Ok((name, dtype, notnull != 0, dflt_value, pk != 0))
            })?;

            for col in columns {
                let (name, dtype, notnull, dflt, pk) = col?;
                println!(
                    "- {} {}{}{}",
                    name,
                    dtype,
                    if notnull { " NOT NULL" } else { "" },
                    if pk { " PRIMARY KEY" } else { "" }
                );
                if let Some(d) = dflt {
                    println!("  Default: {}", d);
                }
            }

            // Step 3: Get unique indexes
            let mut idx_stmt = conn.prepare(&format!("PRAGMA index_list({})", table))?;

            let indexes: Vec<(String, bool)> = idx_stmt
                .query_map([], |row: &Row| {
                    let name: String = row.get(1)?; // index name
                    let unique: i32 = row.get(2)?; // 1 if UNIQUE
                    Ok((name, unique != 0))
                })?
                .collect::<Result<_, _>>()?; // collect into Vec, propagating any errors

            for (name, unique) in indexes {
                if unique {
                    let mut info_stmt = conn.prepare(&format!("PRAGMA index_info({})", name))?;
                    let cols: Vec<String> = info_stmt
                        .query_map([], |row: &Row| row.get(2))?
                        .collect::<Result<_, _>>()?;
                    println!("- UNIQUE index on columns: {:?}", cols);
                }
            }

            // Step 4: Print table data
            let mut data_stmt = conn.prepare(&format!("SELECT * FROM {}", table))?;
            let column_names: Vec<String> = data_stmt
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            println!("Data columns: {:?}", column_names);

            let rows = data_stmt.query_map([], |row: &Row| {
                let mut values = Vec::new();
                for i in 0..column_names.len() {
                    let val: Result<String, _> = row.get(i);
                    values.push(val.unwrap_or_else(|_| "NULL".to_string()));
                }
                Ok(values)
            })?;

            for row in rows {
                println!("{:?}", row?);
            }
            println!("-----------------------------------");
        }

        Ok(())
    }

    /// Gets the popular tag count relationship division.
    /// IE if tag count is bigger then X then put in the relationship_popular table
    pub(in crate::database) fn get_relationship_popular_division_count(
        &self,
        tn: &Transaction,
    ) -> usize {
        if let Some(settingobj) =
            self.settings_get_name(&"SYSTEM_tag_count_popular_division".to_string())
            && let Some(number) = settingobj.num
        {
            return number;
        }

        let popular_number = 10000;
        self.setting_add_internal(
            tn,
            "SYSTEM_tag_count_popular_division".to_string(),
            Some("defines the division between popular tags an non popular tags".to_string()),
            Some(popular_number),
            None,
        );
        self.setting_add_internal(
            tn,
            "SYSTEM_tag_count_popular_division_old".to_string(),
            Some("defines the division between popular tags an non popular tags. If different then new number then start migration inside of db".to_string()),
            Some(popular_number),
            None
        );

        popular_number
    }

    /// The old number just check this to see if it matches the new one
    pub(in crate::database) fn get_relationship_popular_division_count_old(
        &self,
        tn: &Transaction,
    ) -> usize {
        if let Some(settingobj) =
            self.settings_get_name(&"SYSTEM_tag_count_popular_division_old".to_string())
            && let Some(number) = settingobj.num
        {
            return number;
        }

        let popular_number = 10000;
        self.setting_add_internal(
             tn,
            "SYSTEM_tag_count_popular_division_old".to_string(),
            Some("defines the division between popular tags an non popular tags. If different then new number then start migration inside of db".to_string()),
            Some(popular_number),
            None
        );

        popular_number
    }

    /// Moves relationship items into the new popular table based on count
    pub(in crate::database) fn migrate_relationships_based_on_count(&self, tn: &Transaction) {
        let new_count = self.get_relationship_popular_division_count(tn);
        let old_count = self.get_relationship_popular_division_count_old(tn);

        // Updates inmemory count
        {
            let mut count = self.popular_relationship_count.lock();
            *count = Some(new_count);
        }
        if new_count != old_count {
            self.migrate_relationship_popular_count(tn, &old_count, &new_count);
            self.setting_add_internal(
             tn,
            "SYSTEM_tag_count_popular_division_old".to_string(),
            Some("defines the division between popular tags an non popular tags. If different then new number then start migration inside of db".to_string()),
            Some(new_count),
            None
        );
        }
    }

    /// Adds a setting to the Settings Table. name: str   , Setting name pretty: str ,
    /// Fancy Flavor text optional num: u64    , unsigned u64 largest int is
    /// 18446744073709551615 smallest is 0 param: str  , Parameter to allow (value)
    pub(in crate::database) fn setting_add_internal(
        &self,
        tn: &Transaction,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        self.setting_add_sql(tn, name.to_string(), &pretty, num, &param);

        // Adds setting into memdbb
        self.setting_add_internal_db(name, pretty, num, param);
    }
}

#[cfg(test)]
pub(crate) mod test_database {
    use super::*;
    use crate::{VERS, client::relationship_get_tagid};

    pub fn setup_default_db() -> Vec<Main> {
        let mut out = Vec::new();
        for (cnt, cachetype) in [CacheType::Bare].iter().enumerate() {
            let mut db = Main::new(Some(format!("test{}.db", cnt)), VERS);
            db._cache = cachetype.clone();

            db.tag_add(&"test".to_string(), 1, None);
            db.tag_add(&"test1".to_string(), 1, None);
            db.tag_add(&"test2".to_string(), 1, None);
            db.transaction_flush();

            out.push(db);
        }
        out
    }

    #[test]
    fn db_relationship() {
        for mut main in setup_default_db() {
            main.add_relationship_sql(0, 0);
            dbg!(&main._cache, main.relationship_get_fileid(&0));
            assert_eq!(main.relationship_get_fileid(&0).len(), 1);
            assert_eq!(main.relationship_get_tagid(&0).len(), 1);
            let mut test_hashset: HashSet<usize> = HashSet::new();
            test_hashset.insert(0);
            assert_eq!(main.relationship_get_fileid(&0), test_hashset);
            assert_eq!(main.relationship_get_tagid(&0), test_hashset);
        }
    }

    #[test]
    fn db_parents_tagid_remove() {
        for mut main in setup_default_db() {
            main.parents_tagid_remove(&1);
            assert_eq!(main.parents_rel_get(&1), HashSet::new());
            assert_eq!(main.parents_tag_get(&1), HashSet::new());
        }
    }

    #[test]
    fn db_test_load_tags() {
        for mut main in setup_default_db() {
            dbg!(&main._cache);
            dbg!(main.tags_max_id());
            dbg!(main.tag_id_get(&0));
            dbg!(main.tag_id_get(&1));
            dbg!(main.tag_id_get(&2));
            dbg!(main.tag_id_get(&3));
            dbg!(main.tag_id_get(&4));
            assert_eq!(main.tags_max_id(), 4);
            let id = main.tag_add(&"test3".to_string(), 3, None);
            assert_eq!(id, 4);

            assert!(main.tag_id_get(&2).is_some());
        }
    }
    /* #[test]
    fn condense_tags_internal_test() {
        let test_id = 10;
        for mut main in setup_default_db() {
            let max_tag = main.tags_max_id();
            let id = main.tag_add(&"test3".to_string(), 3,  Some(test_id));
            let parents_id = main.parents_add_internal(sharedtypes::DbParentsObj {
                tag_id: id,
                relate_tag_id: 3,
                limit_to: None,
            });
            for i in 0..test_id + 1 {
                dbg!(&i, main.tag_id_get(&i));
            }

            main.parents_add_internal(sharedtypes::DbParentsObj {
                tag_id: id,
                relate_tag_id: 1,
                limit_to: None,
            });
            main.parents_add_internal(sharedtypes::DbParentsObj {
                tag_id: 2,
                relate_tag_id: id,
                limit_to: None,
            });
            main.parents_add_internal(sharedtypes::DbParentsObj {
                tag_id: 3,
                relate_tag_id: 1,
                limit_to: Some(id),
            });
            main.transaction_flush();
            dbg!(&main._cache, &max_tag, &id);
            main.condense_tags_internal();

            assert!(
                main.parents_get(&sharedtypes::DbParentsObj {
                    tag_id: 3,
                    relate_tag_id: 1,
                    limit_to: Some(max_tag),
                })
                .is_some(),
            );
            assert!(
                main.parents_get(&sharedtypes::DbParentsObj {
                    tag_id: 2,
                    relate_tag_id: max_tag,
                    limit_to: None,
                })
                .is_some(),
            );
            assert!(
                main.parents_get(&sharedtypes::DbParentsObj {
                    tag_id: max_tag,
                    relate_tag_id: 1,
                    limit_to: None,
                })
                .is_some(),
            );

            // Check to see if the parents actually migrated.
            if let Some(parentid) = main.parents_get(&sharedtypes::DbParentsObj {
                tag_id: main.tags_max_id(),
                relate_tag_id: 3,
                limit_to: None,
            }) {
                assert_eq!(parentid, parents_id);
            }

            assert_eq!(main.tags_max_id(), max_tag + 1);
            assert!(main.tag_id_get(&id).is_none());

            for i in 0..max_tag + 1 {
                dbg!(&i, main.tag_id_get(&i));
            }

            assert!(main.tag_id_get(&(max_tag)).is_some());

            if let Some(tag) = main.tag_id_get(&(max_tag)) {
                assert_eq!(tag.name, "test3");
            }
        }
    }*/
    #[test]
    fn db_namespace() {
        for mut main in setup_default_db() {
            dbg!(&main._cache);
            let testid = main.namespace_add(&"test".to_string(), &Some("woohoo".into()));
            let descid = main.namespace_add(&"desc".to_string(), &None);

            dbg!(testid, descid);
            assert!(main.namespace_get(&"test".into()).is_some());
            assert!(main.namespace_get(&"desc".into()).is_some());
        }
    }

    ///
    /// This test does not deduplicate data. We rely on the jobs obj to deduplicate
    ///
    #[test]
    fn db_jobs_check_id() {
        for mut main in setup_default_db() {
            dbg!(&main._cache);
            dbg!(main.jobs_get_max());
            let current_job_max = main.jobs_get_max();
            main.jobs_add(
                None,
                0,
                0,
                sharedtypes::DEFAULT_PRIORITY,
                sharedtypes::DEFAULT_CACHETIME,
                sharedtypes::DEFAULT_CACHECHECK,
                "".to_string(),
                vec![],
                BTreeMap::new(),
                BTreeMap::new(),
                sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::NoScrape,
                    recreation: None,
                },
            );
            dbg!(main.jobs_get_max());
            dbg!(main.jobs_get(&0));
            assert_eq!(main.jobs_get_max(), current_job_max + 1);
            main.jobs_add(
                None,
                0,
                0,
                sharedtypes::DEFAULT_PRIORITY,
                sharedtypes::DEFAULT_CACHETIME,
                sharedtypes::DEFAULT_CACHECHECK,
                "yeet".to_string(),
                vec![],
                BTreeMap::new(),
                BTreeMap::new(),
                sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::NoScrape,
                    recreation: None,
                },
            );
            dbg!(main.jobs_get_max());
            dbg!(main.jobs_get_all());
            assert_eq!(main.jobs_get_max(), current_job_max + 2);

            // Checks if all jobs exist at current time
            for i in current_job_max..main.jobs_get_max() {
                assert!(main.jobs_get(&i).is_some());
            }
        }
    }

    #[test]
    fn db_dead_jobs() {
        let mut mains = setup_default_db();
        mains.push(Main::new(Some("test1.db".to_string()), VERS));

        for mut main in mains {
            main.add_dead_url(&"test".to_string());

            assert!(main.check_dead_url(&"test".to_string()));
            assert!(!main.check_dead_url(&"Null".to_string()));
        }
    }

    #[test]
    fn file_add_test() {
        let mut mains = setup_default_db();

        for mut main in mains {
            dbg!(&main._cache);
            let id = main.file_add_db(sharedtypes::DbFileStorage::NoIdExist(
                sharedtypes::DbFileObjNoId {
                    hash: "yeet".to_string(),
                    ext_id: 1,
                    storage_id: 2,
                },
            ));
            let id1 = main.file_add_db(sharedtypes::DbFileStorage::NoIdExist(
                sharedtypes::DbFileObjNoId {
                    hash: "yeet".to_string(),
                    ext_id: 1,
                    storage_id: 2,
                },
            ));

            assert_eq!(id, id1);
        }
    }
}
