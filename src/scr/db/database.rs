#![forbid(unsafe_code)]
use crate::globalload::GlobalLoad;
use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::ScraperParam;
use crate::Mutex;
use crate::RwLock;
use eta::{Eta, TimeAcc};
use log::{error, info};
use rayon::prelude::*;
pub use rusqlite::types::ToSql;
pub use rusqlite::{params, types::Null, Connection, Result, Transaction};
use std::borrow::BorrowMut;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub mod enclave;
pub mod helpers;
pub mod inmemdbnew;
pub mod sqlitedb;
pub mod updatehandler;

use crate::database::inmemdbnew::NewinMemDB;
/// I dont want to keep writing .to_string on EVERY vector of strings. Keeps me
/// lazy. vec_of_strings["one", "two"];
#[macro_export]
macro_rules! vec_of_strings{
    ($($x: expr), *) =>(vec![$($x.to_string()), *]);
}

/// Returns an open connection to use.
pub fn dbinit(dbpath: &String) -> Connection {
    // Engaging Transaction Handling
    Connection::open(dbpath).unwrap()
}

#[derive(Clone)]
pub enum CacheType {
    // Default option. Will use in memory DB to make store cached data.
    InMemdb,
    // Not yet implmented will be used for using sqlite 3 inmemory db calls.
    InMemory,
    // Will be use to query the DB directly. No caching.
    Bare(String),
}
/// Holder of database self variables
pub struct Main {
    _dbpath: Option<String>,
    pub _conn: Arc<Mutex<Connection>>,
    _vers: usize,
    _active_vers: usize,
    // inmem db with ahash low lookup/insert time. Alernative to hashmap
    _inmemdb: NewinMemDB,
    _dbcommitnum: usize,
    _dbcommitnum_static: Option<usize>,
    _tables_loaded: Vec<sharedtypes::LoadDBTable>,
    _tables_loading: Vec<sharedtypes::LoadDBTable>,
    _cache: CacheType,
    pub globalload: Option<Arc<RwLock<GlobalLoad>>>,
    localref: Option<Arc<RwLock<Main>>>,
}

/// Contains DB functions.
impl Main {
    /// Sets up new db instance.
    pub fn new(path: Option<String>, vers: usize) -> Self {
        // Initiates two connections to the DB. Cheap workaround to avoid loading errors.

        let mut first_time_load_flag = false;

        let mut main = match path {
            Some(ref file_path) => {
                first_time_load_flag = Path::new(&file_path).exists();
                let connection = dbinit(file_path);
                let memdb = NewinMemDB::new();
                let memdbmain = Main {
                    _dbpath: path.clone(),
                    _conn: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                    _vers: vers,
                    _active_vers: 0,
                    _inmemdb: memdb,
                    _dbcommitnum: 0,
                    _dbcommitnum_static: Some(3000),
                    _tables_loaded: vec![],
                    _tables_loading: vec![],
                    _cache: CacheType::Bare(file_path.to_string()),
                    globalload: None,
                    localref: None,
                };
                let mut main = Main {
                    _dbpath: path,
                    _conn: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                    _vers: vers,
                    _active_vers: 0,
                    _inmemdb: memdbmain._inmemdb,
                    _dbcommitnum: 0,
                    _dbcommitnum_static: Some(3000),
                    _tables_loaded: vec![],
                    _tables_loading: vec![],
                    _cache: CacheType::InMemdb,
                    globalload: None,
                    localref: None,
                };
                main._conn = Arc::new(Mutex::new(connection));
                main
            }
            None => {
                let memdb = NewinMemDB::new();
                let memdbmain = Main {
                    _dbpath: None,
                    _conn: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                    _vers: vers,
                    _active_vers: 0,
                    _inmemdb: memdb,
                    _dbcommitnum: 0,
                    _dbcommitnum_static: Some(3000),
                    _tables_loaded: vec![],
                    _tables_loading: vec![],
                    _cache: CacheType::InMemory,
                    globalload: None,
                    localref: None,
                };
                let mut main = Main {
                    _dbpath: None,
                    _conn: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                    _vers: vers,
                    _active_vers: 0,
                    _inmemdb: memdbmain._inmemdb,
                    _dbcommitnum: 0,
                    _dbcommitnum_static: Some(3000),
                    _tables_loaded: vec![],
                    _tables_loading: vec![],
                    _cache: CacheType::InMemdb,
                    globalload: None,
                    localref: None,
                };
                main._conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
                main
            }
        };

        // let path = String::from("./main.db");

        // Sets default settings for db settings.
        main.db_open();
        if !first_time_load_flag {
            dbg!("AAA");
            // Database Doesn't exist
            main.transaction_start();
            main.first_db();
            main.updatedb();
            main.db_commit_man_set();
        } else {
            // Database does exist.
            main.transaction_start();
            logging::log(&format!(
                "Database Exists: {} : Skipping creation.",
                first_time_load_flag
            ));
        }
        main
    }

    /// Returns the db version number
    pub fn db_vers_get(&self) -> usize {
        self._active_vers
    }

    /// Clears in memdb structures
    pub fn clear_cache(&mut self) {
        self._inmemdb.clear_all();
    }

    /// Backs up the DB file.
    pub fn backup_db(&mut self) {
        use chrono::prelude::*;

        // If we don't have a location set then eazy out
        let dbloc = match self.get_db_loc() {
            None => {
                return;
            }
            Some(location) => location,
        };

        let current_date = Utc::now();
        let year = current_date.year();
        let month = current_date.month();
        let day = current_date.day();
        let dbbackuploc = self.settings_get_name(&"db_backup_location".to_string());

        // Default location for the DB
        let defaultloc = String::from("dbbackup");

        // Gets the DB file location for copying.
        let mut add_backup_location = None;

        // Gets the db backup folder from DB or uses the "defaultloc" variable
        let backupfolder = match dbbackuploc {
            None => {
                add_backup_location = Some(defaultloc.clone());
                defaultloc
            }
            Some(dbsetting) => match &dbsetting.param {
                None => {
                    add_backup_location = Some(defaultloc.clone());
                    defaultloc
                }
                Some(loc) => loc.to_string(),
            },
        };
        let properbackuplocation;
        let properbackupfile;

        // Starting to do localization. gets changed at compile time. Super lazy way to do
        // it tho
        let mut cnt = 0;
        loop {
            if cfg!(target_os = "windows") {
                if Path::new(&format!(
                    "{}\\{}\\{}\\{}\\db{}.db",
                    backupfolder, year, month, day, cnt
                ))
                .exists()
                {
                    cnt += 1;
                } else {
                    properbackupfile = format!(
                        "{}\\{}\\{}\\{}\\db{}.db",
                        backupfolder, year, month, day, cnt
                    );
                    properbackuplocation =
                        format!("{}\\{}\\{}\\{}\\", backupfolder, year, month, day);
                    break;
                }
            } else if Path::new(&format!(
                "{}/{}/{}/{}/db{}.db",
                backupfolder, year, month, day, cnt
            ))
            .exists()
            {
                cnt += 1;
            } else {
                properbackupfile =
                    format!("{}/{}/{}/{}/db{}.db", backupfolder, year, month, day, cnt);
                properbackuplocation = format!("{}/{}/{}/{}/", backupfolder, year, month, day);
                break;
            }
        }

        // Flushes anything pending to disk.
        self.transaction_flush();
        self.transaction_close();

        // Creates and copies the DB into the backup folder.
        std::fs::create_dir_all(properbackuplocation.clone()).unwrap();
        logging::info_log(&format!(
            "Copying db from: {} to: {}",
            &dbloc, &properbackupfile
        ));
        std::fs::copy(dbloc, properbackupfile).unwrap();
        self.transaction_start();
        if let Some(newbackupfolder) = add_backup_location {
            self.setting_add(
                "db_backup_location".to_string(),
                Some("The location that the DB get's backed up to".to_string()),
                None,
                Some(newbackupfolder),
                true,
            )
        }
        self.transaction_flush();
        logging::info_log(&"Finished backing up the DB.".to_string());
    }

    /// Returns a files bytes if the file exists. Note if called from intcom then this
    /// locks the DB while getting the file. One workaround it to use get_file and read
    /// bytes in manually in seperate thread. that way minimal locking happens.
    pub fn get_file_bytes(&self, file_id: &usize) -> Option<Vec<u8>> {
        let loc = self.get_file(file_id);
        if let Some(loc) = loc {
            return Some(std::fs::read(loc).unwrap());
        }
        None
    }

    /// Gets the location of a file in the file system
    pub fn get_file(&self, file_id: &usize) -> Option<String> {
        let file = self.file_get_id(file_id);
        if let Some(file_obj) = file {
            // Checks that the file with existing info exists
            let file = match file_obj {
                sharedtypes::DbFileStorage::Exist(file) => file,
                _ => return None,
            };

            let location = self.storage_get_string(&file.storage_id).unwrap();

            let sup = [location, self.location_get()];
            for each in sup {
                // Cleans the file path if it contains a '/' in it
                let loc = if each.ends_with('/') | each.ends_with('\\') {
                    each[0..each.len() - 1].to_string()
                } else {
                    each.to_string()
                };
                let folderloc = helpers::getfinpath(&loc, &file.hash);
                let out;
                if cfg!(unix) {
                    out = format!("{}/{}", folderloc, file.hash);
                } else if cfg!(windows) {
                    out = format!("{}\\{}", folderloc, file.hash);
                } else {
                    logging::error_log(&"UNSUPPORTED OS FOR GETFILE CALLING.".to_string());
                    return None;
                }

                // New revision of the downloader adds the extension to the file downloaded.
                // This will rename the file if it uses the old file ext
                if let Some(ext_str) = self.extension_get_string(&file.ext_id) {
                    if Path::new(&out).with_extension(ext_str).exists() {
                        return Some(
                            Path::new(&out)
                                .with_extension(ext_str)
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                }

                if Path::new(&out).exists() {
                    let out = std::fs::canonicalize(out)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    return Some(out);
                }
            }
        }
        None
    }

    ///
    /// Adds a dead url into the db
    ///
    pub fn add_dead_url(&mut self, url: &String) {
        self.add_dead_url_sql(url);
        self.add_dead_url_internal(url.to_string());
    }

    fn add_dead_url_internal(&mut self, url: String) {
        self._inmemdb.add_dead_source_url(url);
    }

    ///
    ///Checks if a url is dead
    ///
    pub fn check_dead_url(&self, url: &String) -> bool {
        match self._inmemdb.does_dead_source_exist(url) {
            true => {
                return true;
            }
            false => {
                let conn = self._conn.lock().unwrap();
                conn.query_row(
                    "SELECT id from dead_source_urls WHERE dead_url = ?",
                    params![url],
                    |row| Ok(row.get(0).unwrap_or(false)),
                )
                .unwrap_or(false)
            }
        }

        /**/
    }

    /// Adds the job to the inmemdb
    pub fn jobs_add_new_todb(&mut self, job: sharedtypes::DbJobsObj) {
        // let querya = query.split(' ').map(|s| s.to_string()).collect(); let wrap =
        // jobs::JobsRef{};
        self._inmemdb.jobref_new(job);
        // self._inmemdb.jobref_new( site.to_string(), querya, current_time, time_offset,
        // committype.clone(), );
    }

    // fn jobs_add_new_sql( &mut self, site: &String, query: &String, _time: &str,
    // committype: &sharedtypes::CommitType, current_time: usize, time_offset: usize,
    // jobtype: sharedtypes::DbJobType, ) { let inp = "INSERT INTO Jobs VALUES(?, ?,
    // ?, ?, ?, ?)"; let _out = self._conn.lock().unwrap().borrow_mut().execute( inp,
    // params![ current_time.to_string(), time_offset.to_string(), site, query,
    // committype.to_string(), jobtype.to_string() ], ); self.db_commit_man(); }
    /// Flips the status of if a job is running
    pub fn jobs_flip_inmemdb(&mut self, id: &usize) -> Option<bool> {
        self._inmemdb.jobref_flip_isrunning(id)
    }

    /// Gets all running jobs in the db
    pub fn jobs_get_isrunning(&self) -> HashSet<&sharedtypes::DbJobsObj> {
        self._inmemdb.jobref_get_isrunning()
    }

    /// File Sanity Checker This will check that the files by id will have a matching
    /// location & hash.
    pub fn db_sanity_check_file(&mut self) {
        use crate::download;

        self.load_table(&sharedtypes::LoadDBTable::Files);
        let flist = self.file_get_list_id();
        flist.par_iter().for_each(|feach| {
            // Check is needed to support if the file with nonexistant info was gotten from db
            if let Some(filestorage_obj) = self.file_get_id(feach) {
                let fileinfo = match filestorage_obj {
                    sharedtypes::DbFileStorage::Exist(file) => file,
                    _ => {
                        panic!("Pulled item that shouldnt exist: {:?}", filestorage_obj);
                    }
                };
                loop {
                    let location = self.storage_get_string(&fileinfo.storage_id).unwrap();
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
        });
    }

    /// Handles the searching of the DB dynamically. Returns the file id's associated
    /// with the search.
    pub fn search_db_files(
        &self,
        search: sharedtypes::SearchObj,
        //limit: Option<usize>,
        //offset: Option<usize>,
    ) -> Option<HashSet<usize>> {
        let mut stor: Vec<sharedtypes::SearchHolder> = Vec::with_capacity(search.searches.len());
        let mut fin: HashSet<usize> = HashSet::new();
        let mut fin_temp: HashMap<usize, HashSet<usize>> = HashMap::new();
        let mut searched: Vec<(usize, usize)> = Vec::with_capacity(search.searches.len());
        if search.search_relate.is_none() {
            if search.searches.len() == 1 {
                stor.push(sharedtypes::SearchHolder::And((0, 0)));
            } else {
                // Assume AND search
                for each in 0..search.searches.len() {
                    stor.push(sharedtypes::SearchHolder::And((each, each + 1)));
                }
            }
        } else {
            stor = search.search_relate.unwrap();
        }
        for (cnt, un) in search.searches.into_iter().enumerate() {
            match un {
                sharedtypes::SearchHolder::Not((a, b)) => {
                    let fa = self.relationship_get_fileid(&a);
                    let fb = self.relationship_get_fileid(&b);
                    fin_temp.insert(cnt, fa.difference(&fb).cloned().collect());
                    searched.push((cnt, a));
                    searched.push((cnt, b));
                }
                sharedtypes::SearchHolder::And((a, b)) => {
                    let fa = self.relationship_get_fileid(&a);
                    let fb = self.relationship_get_fileid(&b);
                    fin_temp.insert(cnt, fa.intersection(&fb).cloned().collect());

                    searched.push((cnt, a));
                    searched.push((cnt, b));
                }
                sharedtypes::SearchHolder::Or((a, b)) => {
                    let fa = self.relationship_get_fileid(&a);
                    let fb = self.relationship_get_fileid(&b);
                    fin_temp.insert(cnt, fa.union(&fb).cloned().collect());
                    searched.push((cnt, a));
                    searched.push((cnt, b));
                }
            }
        }
        for each in stor {
            match each {
                sharedtypes::SearchHolder::Or((_a, _b)) => {}
                sharedtypes::SearchHolder::And((a, b)) => {
                    let fa = fin_temp.get(&a).unwrap();
                    let fb = fin_temp.get(&b).unwrap();
                    let tem = fa.intersection(fb);
                    for each in tem {
                        fin.insert(*each);
                    }
                }
                sharedtypes::SearchHolder::Not((_a, _b)) => {}
            }
        }
        if !fin.is_empty() {
            return Some(fin);
        }
        None
    }

    /// Wrapper
    pub fn jobs_get_all(&self) -> &HashMap<usize, sharedtypes::DbJobsObj> {
        match &self._cache {
            CacheType::InMemdb => self._inmemdb.jobs_get_all(),
            CacheType::InMemory => {
                todo!();
            }
            CacheType::Bare(_dbpath) => {
                todo!();
            }
        }
    }

    /// Pull job by id TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    pub fn jobs_get(&self, id: &usize) -> Option<&sharedtypes::DbJobsObj> {
        self._inmemdb.jobs_get(id)
    }

    pub fn tag_id_get(&self, uid: &usize) -> Option<&sharedtypes::DbTagNNS> {
        self._inmemdb.tags_get_data(uid)
    }

    pub fn tags_max_id(&self) -> usize {
        self._inmemdb.tags_max_return()
    }

    ///
    /// Returns a list of loaded tag ids
    ///
    pub fn tags_get_list_id(&self) -> HashSet<usize> {
        self._inmemdb.tags_get_list_id()
    }

    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(&self, tag: &usize) -> HashSet<usize> {
        self._inmemdb.relationship_get_fileid(tag)
    }

    pub fn relationship_get_one_fileid(&self, tag: &usize) -> Option<&usize> {
        self._inmemdb.relationship_get_one_fileid(tag)
    }

    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(&self, tag: &usize) -> HashSet<usize> {
        self._inmemdb.relationship_get_tagid(tag)
    }

    pub fn settings_get_name(&self, name: &String) -> Option<&sharedtypes::DbSettingObj> {
        self._inmemdb.settings_get_name(name)
    }

    /// Returns next jobid from _inmemdb
    pub fn jobs_get_max(&self) -> &usize {
        self._inmemdb.jobs_get_max()
    }

    /// Vacuums database. cleans everything.
    fn vacuum(&mut self) {
        logging::info_log(&"Starting Vacuum db!".to_string());
        self.transaction_flush();
        self.transaction_close();
        self.execute("VACUUM;".to_string());
        self.transaction_start();
        logging::info_log(&"Finishing Vacuum db!".to_string());
    }

    /// Sets up first database interaction. Makes tables and does first time setup.
    pub fn first_db(&mut self) {
        // Making Relationship Table
        let mut name = "Relationship".to_string();
        let mut keys = vec_of_strings!["fileid", "tagid"];
        let mut vals = vec_of_strings!["INTEGER", "INTEGER"];
        self.table_create(&name, &keys, &vals);

        // Making Tags Table
        name = "Tags".to_string();
        keys = vec_of_strings!["id", "name", "namespace"];
        vals = vec_of_strings!["INTEGER", "TEXT", "INTEGER"];
        self.table_create(&name, &keys, &vals);

        // Making Parents Table. Relates tags to tag parents.
        name = "Parents".to_string();
        keys = vec_of_strings!["tag_id", "relate_tag_id", "limit_to"];
        vals = vec_of_strings!["INTEGER", "INTEGER", "INTEGER"];
        self.table_create(&name, &keys, &vals);

        // Making Namespace Table
        name = "Namespace".to_string();
        keys = vec_of_strings!["id", "name", "description"];
        vals = vec_of_strings!["INTEGER", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Settings Table
        name = "Settings".to_string();
        keys = vec_of_strings!["name", "pretty", "num", "param"];
        vals = vec_of_strings!["TEXT PRIMARY KEY", "TEXT", "INTEGER", "TEXT"];
        self.table_create(&name, &keys, &vals);

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
            "INTEGER PRIMARY KEY",
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

        self.table_create(&name, &keys, &vals);

        self.enclave_create_database_v5();

        // Making dead urls Table
        name = "dead_source_urls".to_string();
        keys = vec_of_strings!["id", "dead_url"];
        vals = vec_of_strings!["INTEGER PRIMARY KEY", "TEXT NOT NULL"];
        self.table_create(&name, &keys, &vals);

        self.transaction_flush();
    }

    pub fn updatedb(&mut self) {
        self.setting_add(
            "DBCOMMITNUM".to_string(),
            Some("Number of transactional items before pushing to db.".to_string()),
            Some(3000),
            None,
            true,
        );
        self.setting_add(
            "VERSION".to_string(),
            Some("Version that the database is currently on.".to_string()),
            Some(self._vers.try_into().unwrap()),
            None,
            true,
        );
        self.setting_add("DEFAULTRATELIMIT".to_string(), None, Some(5), None, true);
        self.setting_add(
            "FilesLoc".to_string(),
            None,
            None,
            Some("Files".to_string()),
            true,
        );
        self.setting_add(
            "DEFAULTUSERAGENT".to_string(),
            None,
            None,
            Some("DIYHydrus/1.0".to_string()),
            true,
        );
        self.setting_add(
            "pluginloadloc".to_string(),
            Some("Where plugins get loaded into.".to_string()),
            None,
            Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
            true,
        );

        self.setting_add(
            "scraperloadloc".to_string(),
            Some("Where scrapers get loaded into.".to_string()),
            None,
            Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
            true,
        );

        self.enclave_create_default_file_download(self.location_get());

        self.transaction_flush();
    }

    ///
    /// Gets a scraper folder. If it doesn't exist then please create it in db
    ///
    pub fn loaded_scraper_folder(&mut self) -> PathBuf {
        match self.settings_get_name(&"scraperloadloc".to_string()) {
            Some(setting) => {
                if let Some(param) = &setting.param {
                    Path::new(param).to_path_buf()
                } else {
                    self.setting_add(
                        "scraperloadloc".to_string(),
                        Some("Where scrapers get loaded into.".to_string()),
                        None,
                        Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
                        true,
                    );
                    Path::new(crate::DEFAULT_LOC_SCRAPER).to_path_buf()
                }
            }
            None => {
                self.setting_add(
                    "scraperloadloc".to_string(),
                    Some("Where scrapers get loaded into.".to_string()),
                    None,
                    Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
                    true,
                );
                Path::new(crate::DEFAULT_LOC_SCRAPER).to_path_buf()
            }
        }
    }

    ///
    /// Gets a plugin folder. If it doesn't exist then please create it in db
    ///
    pub fn loaded_plugin_folder(&mut self) -> PathBuf {
        match self.settings_get_name(&"pluginloadloc".to_string()) {
            Some(setting) => {
                if let Some(param) = &setting.param {
                    Path::new(param).to_path_buf()
                } else {
                    self.setting_add(
                        "pluginloadloc".to_string(),
                        Some("Where plugins get loaded into.".to_string()),
                        None,
                        Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
                        true,
                    );
                    Path::new(crate::DEFAULT_LOC_PLUGIN).to_path_buf()
                }
            }
            None => {
                self.setting_add(
                    "pluginloadloc".to_string(),
                    Some("Where plugins get loaded into.".to_string()),
                    None,
                    Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
                    true,
                );
                Path::new(crate::DEFAULT_LOC_PLUGIN).to_path_buf()
            }
        }
    }

    ///
    /// Gets a default namespace id if it doesn't exist
    ///
    pub fn create_default_source_url_ns_id(&mut self) -> usize {
        match self.namespace_get(&"source_url".to_string()).cloned() {
            None => self.namespace_add(
                "source_url".to_string(),
                Some("Source URL for a file.".to_string()),
                true,
            ),
            Some(id) => id,
        }
    }

    /// Creates a table name: The table name key: List of Collumn lables. dtype: List
    /// of Collumn types. NOTE Passed into SQLITE DIRECTLY THIS IS BAD :C
    pub fn table_create(&mut self, name: &String, key: &Vec<String>, dtype: &Vec<String>) {
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
        info!("Creating table as: {}", endresult);
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute_batch(&endresult);
        //stocat = endresult;
        //self.execute(stocat);
    }

    /// Alters a tables name
    fn alter_table(&mut self, original_table: &String, new_table: &String) {
        self.execute(format!(
            "ALTER TABLE {} RENAME TO {};",
            original_table, new_table
        ));
    }

    /// Checks if table exists in DB if it do then delete.
    fn check_table_exists(&mut self, table: String) -> bool {
        let mut out = false;
        let query_string = format!(
            "SELECT * FROM sqlite_master WHERE type='table' AND name='{}';",
            table
        );
        let binding = self._conn.lock().unwrap();
        let mut toexec = binding.prepare(&query_string).unwrap();
        let mut rows = toexec.query(params![]).unwrap();
        if let Some(_each) = rows.next().unwrap() {
            out = true;
        }
        out
    }

    /// Gets the names of a collumn in a table
    fn db_table_collumn_getnames(&mut self, table: &String) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        {
            let conn = self._conn.lock().unwrap();
            let stmt = conn
                .prepare(&format!("SELECT * FROM {} LIMIT 1", table))
                .unwrap();
            for collumn in stmt.column_names() {
                out.push(collumn.to_owned());
            }
        }
        out
    }

    /// Sets DB Version
    fn db_version_set(&mut self, version: usize) {
        logging::log(&format!("Setting DB Version to: {}", &version));
        self._active_vers = version;
        self.setting_add(
            "VERSION".to_string(),
            Some("Version that the database is currently on.".to_string()),
            Some(version),
            None,
            true,
        );
    }

    fn db_drop_table(&mut self, table: &String) {
        let query_string = format!("DROP TABLE IF EXISTS {};", table);
        let binding = self._conn.lock().unwrap();
        let mut toexec = binding.prepare(&query_string).unwrap();
        toexec.execute(params![]).unwrap();
    }

    /// Checks if db version is consistent. If this function returns false signifies
    /// that we shouldn't run.
    pub fn check_version(&mut self) -> bool {
        let mut query_string = "SELECT num FROM Settings WHERE name='VERSION';";
        let query_string_manual = "SELECT num FROM Settings_Old WHERE name='VERSION';";
        let mut g1 = self.quer_int(query_string.to_string()).unwrap();
        if g1.len() != 1 {
            error!(
                "Could not check_version due to length of recieved version being less then one. Trying manually!!!"
            );

            // let out = self.execute("SELECT num from Settings WHERE
            // name='VERSION';".to_string());
            let binding = self._conn.lock().unwrap();
            let mut toexec = binding.prepare(query_string).unwrap();
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
            let binding = self._conn.lock().unwrap();
            let mut toexec = binding.prepare(query_string).unwrap();
            let mut rows = toexec.query(params![]).unwrap();
            g1.clear();
            while let Some(each) = rows.next().unwrap() {
                let ver: String = each.get(0).unwrap();

                // let vers = ver.try_into().unwrap();
                let izce = ver.parse().unwrap();
                g1.push(izce)
            }
            logging::panic_log(
                &"check_version: Could not load DB properly PANICING!!!".to_string(),
            );
        }
        let mut db_vers = g1[0] as usize;
        self._active_vers = db_vers;
        logging::info_log(&format!("check_version: Loaded version {}", db_vers));
        if self._active_vers != self._vers {
            logging::info_log(&format!(
                "Starting upgrade from V{} to V{}",
                db_vers,
                db_vers + 1
            ));

            // Resets the DB internal Cache if their is any.
            self.clear_cache();
            self.load_table(&sharedtypes::LoadDBTable::Settings);

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
            }
            logging::info_log(&format!("Finished upgrade to V{}.", db_vers));
            self.transaction_flush();
            if db_vers == self._vers {
                logging::info_log(&format!(
                    "Successfully updated db to version {}",
                    self._vers
                ));
                return true;
            }
        } else {
            info!("Database Version is: {}", g1[0]);
            return true;
        }
        false
    }

    /// Checks if table is loaded in mem and if not then loads it.
    pub fn load_table(&mut self, table: &sharedtypes::LoadDBTable) {
        // Blocks the thread until another thread has finished loading the table.
        while self._tables_loading.contains(table) {
            let dur = std::time::Duration::from_secs(1);
            std::thread::sleep(dur);
        }
        if !self._tables_loaded.contains(table) {
            self._tables_loading.push(*table);
            match &table {
                sharedtypes::LoadDBTable::Files => {
                    self.load_files();
                }
                sharedtypes::LoadDBTable::Jobs => {
                    self.load_jobs();
                }
                sharedtypes::LoadDBTable::Namespace => {
                    self.load_namespace();
                }
                sharedtypes::LoadDBTable::Parents => {
                    self.load_parents();
                }
                sharedtypes::LoadDBTable::Relationship => {
                    self.load_relationships();
                }
                sharedtypes::LoadDBTable::Settings => {
                    self.load_settings();
                }
                sharedtypes::LoadDBTable::Tags => {
                    self.load_tags();
                }
                sharedtypes::LoadDBTable::DeadSourceUrls => {
                    self.load_dead_urls();
                }
                sharedtypes::LoadDBTable::All => {
                    self.load_table(&sharedtypes::LoadDBTable::Tags);
                    self.load_table(&sharedtypes::LoadDBTable::Files);
                    self.load_table(&sharedtypes::LoadDBTable::Jobs);
                    self.load_table(&sharedtypes::LoadDBTable::Namespace);
                    self.load_table(&sharedtypes::LoadDBTable::Parents);
                    self.load_table(&sharedtypes::LoadDBTable::Relationship);
                    self.load_table(&sharedtypes::LoadDBTable::Settings);
                }
            }
            self._tables_loaded.push(*table);
            self._tables_loading.retain(|&x| x != *table);
        }
    }

    /// Adds file into Memdb instance.
    pub fn file_add_db(&mut self, file: sharedtypes::DbFileStorage) -> usize {
        self._inmemdb.file_put(file)
    }

    /// NOTE USES PASSED CONNECTION FROM FUNCTION NOT THE DB CONNECTION GETS ARROUND
    /// MEMROY SAFETY ISSUES WITH CLASSES IN RUST
    fn load_files(&mut self) {
        self.load_extensions();

        logging::info_log(&"Database is Loading: Files".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM File");
        if let Ok(mut con) = temp {
            let files = con
                .query_map([], |row| {
                    let id: Option<usize> = row.get(0).unwrap();
                    let hash: Option<String> = row.get(1).unwrap();
                    let ext: Option<usize> = row.get(2).unwrap();
                    let location: Option<usize> = row.get(3).unwrap();
                    if id.is_some() && hash.is_some() && ext.is_some() && location.is_some() {
                        Ok(sharedtypes::DbFileStorage::Exist(sharedtypes::DbFileObj {
                            id: row.get(0).unwrap(),
                            hash: row.get(1).unwrap(),
                            ext_id: row.get(2).unwrap(),
                            storage_id: row.get(3).unwrap(),
                        }))
                    } else if id.is_some() && hash.is_none() && ext.is_none() && location.is_none()
                    {
                        Ok(sharedtypes::DbFileStorage::NoExist(id.unwrap()))
                    } else {
                        panic!("Error on: {:?} {:?} {:?} {:?}", id, hash, ext, location);
                    }
                })
                .unwrap();
            for each in files {
                if let Ok(res) = each {
                    self.file_add_db(res);
                } else {
                    error!("Bad File cant load {:?}", each);
                }
            }
        }
        // fiex = conn.prepare("SELECT * FROM File").unwrap(); files = fiex .query_map([],
        // |row| { Ok(sharedtypes::DbFileObj { id: row.get(0).unwrap(), hash:
        // row.get(1).unwrap(), ext: row.get(2).unwrap(), location: row.get(3).unwrap(),
        // }) }) .unwrap();
    }

    ///
    /// Gets an ID if a extension string exists
    ///
    pub fn extension_get_id(&self, ext: &String) -> Option<&usize> {
        self._inmemdb.extension_get_id(ext)
    }
    ///
    /// Gets an ID if a extension string exists
    ///
    pub fn extension_get_string(&self, ext_id: &usize) -> Option<&String> {
        self._inmemdb.extension_get_string(ext_id)
    }

    ///
    /// Gets an extension id and creates it if it does not exist
    ///
    pub fn extension_put_string(&mut self, ext: &String) -> usize {
        match self.extension_get_id(ext) {
            Some(id) => *id,
            None => {
                let conn = self._conn.lock().unwrap();
                conn.execute(
                    "insert or ignore into FileExtensions(extension) VALUES (?)",
                    params![ext],
                )
                .unwrap();
                let out: usize = conn
                    .query_row(
                        "select id from FileExtensions where extension = ?",
                        params![ext],
                        |row| row.get(0),
                    )
                    .unwrap();
                out
            }
        }
    }

    /// Puts extension into mem cache
    pub fn extension_load(&mut self, id: usize, extension: String) {
        self._inmemdb.extension_load(id, extension);
    }

    /// Loads extensions into db
    fn load_extensions(&mut self) {
        logging::info_log(&"Database is Loading: File Extensions".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM FileExtensions");

        if let Ok(mut con) = temp {
            let quer = con.query([]);
            if let Ok(mut rows) = quer {
                while let Ok(Some(row)) = rows.next() {
                    let id: Option<usize> = row.get(0).unwrap();
                    let extension: Option<String> = row.get(1).unwrap();

                    if let Some(ext) = extension {
                        if let Some(id) = id {
                            self.extension_load(id, ext);
                        }
                    }
                }
            }
        }
    }

    /// Same as above
    fn load_namespace(&mut self) {
        logging::info_log(&"Database is Loading: Namespace".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM Namespace");
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
                    self.namespace_add(res.name, res.description, false);
                } else {
                    error!("Bad Namespace cant load {:?}", each);
                }
            }
        }
    }

    /// Loads jobs in from DB Connection
    fn load_jobs(&mut self) {
        logging::info_log(&"Database is Loading: Jobs".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM Jobs");
        if let Ok(mut con) = temp {
            let jobs = con
                .query_map([], |row| {
                    let id = row.get(0).unwrap();
                    let time = row.get(1).unwrap();
                    let reptime = row.get(2).unwrap();
                    let priority = row.get(3).unwrap_or(sharedtypes::DEFAULT_PRIORITY);
                    let cachetime = row.get(4).unwrap_or_default();
                    let cachechecktype: String = row.get(5).unwrap();
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
                        cachechecktype: serde_json::from_str(&cachechecktype).unwrap(),
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

    /// Wrapper
    pub fn file_get_hash(&self, hash: &String) -> Option<&usize> {
        self._inmemdb.file_get_hash(hash)
    }

    /// Wrapper
    pub fn tag_get_name(&self, tag: String, namespace: usize) -> Option<&usize> {
        let tagobj = &sharedtypes::DbTagNNS {
            name: tag,
            namespace,
        };
        self._inmemdb.tags_get_id(tagobj)
    }

    /// Loads _dbcommitnum from DB Used for determining when to flush to DB.
    /// If we can't load a value for db_commitnum then we don't  flush intermittently
    fn db_commit_man(&mut self) {
        self._dbcommitnum += 1;

        if let Some(static_commit) = self._dbcommitnum_static {
            if self._dbcommitnum >= static_commit {
                logging::info_log(&format!("Flushing {} Records into DB.", static_commit));

                self.transaction_flush();
            }
        }
    }

    /// db get namespace wrapper
    pub fn namespace_get(&self, inp: &String) -> Option<&usize> {
        self._inmemdb.namespace_get(inp)
    }

    /// Returns namespace as a string from an ID returns None if it doesn't exist.
    pub fn namespace_get_string(&self, inp: &usize) -> Option<&sharedtypes::DbNamespaceObj> {
        self._inmemdb.namespace_id_get(inp)
    }

    ///
    /// loads the commit number into memory
    ///
    pub fn db_commit_man_set(&mut self) {
        if let Some(num) = self.settings_get_name(&"DBCOMMITNUM".to_string()) {
            self._dbcommitnum_static = num.num;
        }
    }

    /// Adds a file into the db sqlite. Do this first.
    pub fn file_add(&mut self, file: sharedtypes::DbFileStorage, addtodb: bool) -> usize {
        match file {
            sharedtypes::DbFileStorage::Exist(ref file_obj) => {
                if self.file_get_hash(&file_obj.hash).is_none() {
                    let id = self.file_add_db(file.clone());
                    if addtodb {
                        self.file_add_sql(
                            &Some(file_obj.hash.clone()),
                            &Some(file_obj.ext_id),
                            &Some(file_obj.storage_id),
                            &file_obj.id,
                        );
                    }
                    id
                } else {
                    file_obj.id
                }
            }
            sharedtypes::DbFileStorage::NoIdExist(ref noid_obj) => {
                if self.file_get_hash(&noid_obj.hash).is_none() {
                    let id = self.file_add_db(file.clone());
                    if addtodb {
                        self.file_add_sql(
                            &Some(noid_obj.hash.clone()),
                            &Some(noid_obj.ext_id),
                            &Some(noid_obj.storage_id),
                            &id,
                        );
                    }
                    id
                } else {
                    *self.file_get_hash(&noid_obj.hash).unwrap()
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

    /// Wrapper for inmemdb function: file_get_id Returns info for file in Option
    // DO NOT USE UNLESS NECISSARY. LOG(n2) * 3
    pub fn file_get_id(&self, fileid: &usize) -> Option<&sharedtypes::DbFileStorage> {
        self._inmemdb.file_get_id(fileid)
    }

    /// Wrapper for inmemdb adding
    fn namespace_add_db(&mut self, namespace_obj: sharedtypes::DbNamespaceObj) -> usize {
        self._inmemdb.namespace_put(namespace_obj)
    }

    /// Adds namespace into DB. Returns the ID of the namespace.
    pub fn namespace_add(
        &mut self,
        name: String,
        description: Option<String>,
        addtodb: bool,
    ) -> usize {
        let namespace_grab = self._inmemdb.namespace_get(&name);
        match namespace_grab {
            None => {}
            Some(id) => return id.to_owned(),
        }
        let ns_id = self._inmemdb.namespace_get_max();
        let ns = sharedtypes::DbNamespaceObj {
            id: ns_id,
            name,
            description,
        };
        if addtodb && namespace_grab.is_none() {
            self.namespace_add_sql(&ns.name, &ns.description, &ns_id);
        }
        self.namespace_add_db(ns)
    }

    /// Wrapper that handles inserting parents info into DB.
    fn parents_add_sql(&mut self, parent: &sharedtypes::DbParentsObj) {
        let inp = "INSERT INTO Parents VALUES(?, ?, ?)";
        let limit_to = match parent.limit_to {
            None => &Null as &dyn ToSql,
            Some(out) => &out.to_string(),
        };
        let _out = self._conn.borrow_mut().lock().unwrap().execute(
            inp,
            params![
                parent.tag_id.to_string(),
                parent.relate_tag_id.to_string(),
                limit_to
            ],
        );
        self.db_commit_man();
    }

    /// Wrapper for inmemdb adding
    fn parents_add_db(&mut self, parent: sharedtypes::DbParentsObj) -> usize {
        self._inmemdb.parents_put(parent)
    }

    /// Wrapper for inmemdb and parents_add_db
    pub fn parents_add(
        &mut self,
        tag_id: usize,
        relate_tag_id: usize,
        limit_to: Option<usize>,
        addtodb: bool,
    ) -> usize {
        let par = sharedtypes::DbParentsObj {
            tag_id,
            relate_tag_id,
            limit_to,
        };
        let parent = self._inmemdb.parents_get(&par);
        if addtodb & &parent.is_none() {
            self.parents_add_sql(&par);
        }
        self.parents_add_db(par)
    }

    /// Relates the list of relationships assoicated with tag
    pub fn parents_rel_get(&self, relid: &usize) -> Option<HashSet<usize>> {
        self._inmemdb.parents_rel_get(relid, None)
    }

    /// Relates the list of tags assoicated with relations
    pub fn parents_tag_get(&self, tagid: &usize) -> Option<HashSet<usize>> {
        self._inmemdb.parents_tag_get(tagid, None)
    }

    /// Adds tag into inmemdb
    fn tag_add_db(&mut self, tag: &String, namespace: &usize, id: Option<usize>) -> usize {
        let selected_id = match id {
            None => {
                match self._inmemdb.tags_get_id(&sharedtypes::DbTagNNS {
                    name: tag.to_string(),
                    namespace: namespace.to_owned(),
                }) {
                    None => {
                        let tag_info = sharedtypes::DbTagNNS {
                            name: tag.to_string(),
                            namespace: *namespace,
                        };
                        self._inmemdb.tags_put(&tag_info, id)
                    }
                    Some(tag_id_max) => *tag_id_max,
                }
            }
            Some(out) => {
                let tag_info = sharedtypes::DbTagNNS {
                    name: tag.to_string(),
                    namespace: *namespace,
                };
                self._inmemdb.tags_put(&tag_info, Some(out))
            }
        };

        selected_id
    }

    /// Prints db info
    pub fn debugdb(&self) {
        self._inmemdb.dumpe_data();
    }

    pub fn setup_globalload(&mut self, globalload: Arc<RwLock<GlobalLoad>>) {
        self.globalload = Some(globalload);
    }

    pub fn setup_localref(&mut self, localref: Arc<RwLock<Main>>) {
        self.localref = Some(localref);
    }

    pub fn namespace_add_namespaceobject(
        &mut self,
        namespace_obj: sharedtypes::GenericNamespaceObj,
    ) -> usize {
        self.namespace_add(namespace_obj.name, namespace_obj.description, true)
    }

    pub fn tag_add_tagobject(&mut self, tag: &sharedtypes::TagObject) -> usize {
        let nsid = self.namespace_add_namespaceobject(tag.namespace.clone());
        self.tag_add(&tag.tag, nsid, true, None)
    }

    /// Adds tag into DB if it doesn't exist in the memdb.
    pub fn tag_add(
        &mut self,
        tags: &String,
        namespace: usize,
        addtodb: bool,
        id: Option<usize>,
    ) -> usize {
        //testing only please remove once the direct download plugin finishes

        match id {
            None => {
                //if let Some(globalload) = &self.globalload {
                //let globalload = globalload.clone();
                /*globalload::plugin_on_tag(
                    &mut globalload.write().unwrap(),
                    self,
                    tags,
                    &namespace,
                );*/
                //}

                // Do we have an ID coming in to add manually?
                let tagnns = sharedtypes::DbTagNNS {
                    name: tags.to_string(),
                    namespace,
                };
                let tags_grab = self._inmemdb.tags_get_id(&tagnns).copied();
                match tags_grab {
                    None => {
                        let tag_id = self.tag_add_db(tags, &namespace, None);
                        if addtodb {
                            self.tag_add_sql(&tag_id, tags, &namespace);
                        }
                        tag_id
                    }
                    Some(tag_id) => tag_id,
                }
            }
            Some(_) => {
                // We've got an ID coming in will check if it exists.
                let tag_id = self.tag_add_db(tags, &namespace, id);
                if addtodb {
                    self.tag_add_sql(&tag_id, tags, &namespace);
                }
                tag_id
            }
        }
    }

    /// Wrapper for inmemdb relationship adding
    fn relationship_add_db(&mut self, file: usize, tag: usize) {
        self._inmemdb.relationship_add(file, tag);
    }

    /// Adds relationship to SQL db.
    fn relationship_add_sql(&mut self, file: usize, tag: usize) {
        let inp = "INSERT INTO Relationship VALUES(?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![&file.to_string(), &tag.to_string()]);
        self.db_commit_man();
    }

    /// Adds relationship into DB. Inherently trusts user user to not duplicate stuff.
    pub fn relationship_add(&mut self, file: usize, tag: usize, addtodb: bool) {
        let existcheck = self._inmemdb.relationship_get(&file, &tag);
        if addtodb && !existcheck {
            // println!("relationship a ");
            self.relationship_add_sql(file, tag);
        }
        if !existcheck {
            // println!("relationship b ");
            self.relationship_add_db(file, tag);
            self.db_commit_man();
        }
        // println!("relationship complete : {} {}", file, tag);
    }

    /// Updates the database for inmemdb and sql
    pub fn jobs_update_db(&mut self, jobs_obj: sharedtypes::DbJobsObj) {
        if self._inmemdb.jobref_new(jobs_obj.clone()) {
            self.jobs_update_by_id(&jobs_obj);
        } else {
            self.jobs_add_sql(&jobs_obj)
        }
    }

    /// Adds a job to sql
    fn jobs_add_sql(&mut self, data: &sharedtypes::DbJobsObj) {
        dbg!(data);
        let inp = "INSERT INTO Jobs VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(
                inp,
                params![
                    data.id.unwrap().to_string(),
                    data.time.to_string(),
                    data.reptime.unwrap().to_string(),
                    data.priority.to_string(),
                    serde_json::to_string(&data.cachetime).unwrap(),
                    serde_json::to_string(&data.cachechecktype).unwrap(),
                    serde_json::to_string(&data.jobmanager).unwrap(),
                    data.site,
                    serde_json::to_string(&data.param).unwrap(),
                    serde_json::to_string(&data.system_data).unwrap(),
                    serde_json::to_string(&data.user_data).unwrap(),
                ],
            )
            .unwrap();
        self.db_commit_man();
    }

    /// Updates job by id
    fn jobs_update_by_id(&mut self, data: &sharedtypes::DbJobsObj) {
        dbg!(data);
        let inp =
            "UPDATE Jobs SET id=?, time=?, reptime=?, Manager=?, priority=?,cachetime=?,cachechecktype=?, site=?, param=?, SystemData=?, UserData=? WHERE id = ?";
        let _out = self._conn.borrow_mut().lock().unwrap().execute(
            inp,
            params![
                data.id.unwrap().to_string(),
                data.time.to_string(),
                data.reptime.unwrap().to_string(),
                serde_json::to_string(&data.jobmanager).unwrap(),
                data.priority.to_string(),
                serde_json::to_string(&data.cachetime).unwrap(),
                serde_json::to_string(&data.cachechecktype).unwrap(),
                data.site,
                serde_json::to_string(&data.param).unwrap(),
                serde_json::to_string(&data.system_data).unwrap(),
                serde_json::to_string(&data.user_data).unwrap(),
                data.id.unwrap().to_string()
            ],
        );
        self.db_commit_man();
    }

    ///
    /// Adds a job into a DB.
    /// Going to make a better job adding function
    ///
    pub fn jobs_add(
        &mut self,
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
            None => *self.jobs_get_max(),
            Some(id) => id,
        };

        for each in self.jobs_get_all() {
            if time == each.1.time
                && Some(reptime) == each.1.reptime
                && site == each.1.site
                && param == each.1.param
            {
                return id;
            }
        }

        let jobs_obj: sharedtypes::DbJobsObj = sharedtypes::DbJobsObj {
            id: Some(id),
            time,
            reptime: Some(reptime),
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

    pub fn jobs_add_new(&mut self, dbjobsobj: sharedtypes::DbJobsObj) -> usize {
        let mut dbjobsobj = dbjobsobj.clone();
        let id = match dbjobsobj.id {
            None => *self.jobs_get_max(),
            Some(id) => id,
        };

        for each in self.jobs_get_all() {
            if dbjobsobj.time == each.1.time
                && dbjobsobj.reptime == each.1.reptime
                && dbjobsobj.site == each.1.site
                && dbjobsobj.param == each.1.param
            {
                return id;
            }
        }

        dbjobsobj.id = Some(id);

        self.jobs_update_db(dbjobsobj);

        id
    }

    /// Wrapper for inmemdb insert.
    fn setting_add_db(
        &mut self,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        self._inmemdb.settings_add(name, pretty, num, param);
    }

    fn setting_add_sql(
        &mut self,
        name: String,
        pretty: &Option<String>,
        num: Option<usize>,
        param: &Option<String>,
    ) {
        let _ex =
            self
                ._conn
                .borrow_mut()
                .lock()
                .unwrap()
                .execute(
                    "INSERT INTO Settings(name, pretty, num, param) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(name) DO UPDATE SET pretty=?2, num=?3, param=?4 ;",
                    params![
                        &name,
                        // Hella jank workaround. can only pass 1 type into a function without doing
                        // workaround. This makes it work should be fine for one offs.
                        if pretty.is_none() {
                            &Null as &dyn ToSql
                        } else {
                            &pretty
                        },
                        if num.is_none() {
                            &Null as &dyn ToSql
                        } else {
                            &num
                        },
                        if param.is_none() {
                            &Null as &dyn ToSql
                        } else {
                            &param
                        }
                    ],
                );
        match _ex {
            Err(_ex) => {
                println!(
                    "setting_add: Their was an error with inserting {} into db. {}",
                    &name, &_ex
                );
                error!(
                    "setting_add: Their was an error with inserting {} into db. {}",
                    &name, &_ex
                );
            }
            Ok(_ex) => self.db_commit_man(),
        }
    }

    /// Adds a setting to the Settings Table. name: str   , Setting name pretty: str ,
    /// Fancy Flavor text optional num: u64    , unsigned u64 largest int is
    /// 18446744073709551615 smallest is 0 param: str  , Parameter to allow (value)
    pub fn setting_add(
        &mut self,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
        addtodb: bool,
    ) {
        if addtodb {
            self.setting_add_sql(name.to_string(), &pretty, num, &param);
        }

        // Adds setting into memdbb
        self.setting_add_db(name, pretty, num, param);
        self.transaction_flush();
    }

    /// Starts a transaction for bulk inserts.
    pub fn transaction_start(&mut self) {
        self.execute("BEGIN".to_string());
    }

    ///
    /// Determines if the DB has pending actions.
    /// Was having a weird edge case where I was flushing over 6k tags at once and the db vomited
    /// on itself. Hopefully this fixes it
    ///
    fn determine_if_busy(&self) -> bool {
        let conn = self._conn.lock().unwrap();
        conn.is_busy()
    }

    /// Flushes to disk.
    pub fn transaction_flush(&mut self) {
        self._dbcommitnum = 0;

        // If were busy doing a transaction then do nothing
        while self.determine_if_busy() {
            std::thread::sleep(Duration::from_millis(100));
        }
        self.execute("COMMIT".to_string());
        self.execute("BEGIN".to_string());
    }

    // Closes a transaction for bulk inserts.
    pub fn transaction_close(&mut self) {
        self.execute("COMMIT".to_string());
        self._dbcommitnum = 0;
    }

    /// Returns db location as String refernce.
    pub fn get_db_loc(&self) -> Option<String> {
        self._dbpath.clone()
    }

    /// database searching advanced.
    pub fn db_search_adv(&self, db_search: sharedtypes::DbSearchQuery) {
        let namespaceone_unwrap = self.search_for_namespace(&db_search.tag_one);

        // let namespacetwo_unwrap = self.search_for_namespace(db_search.tag_two);
        if namespaceone_unwrap.is_none() {
            logging::info_log(&format!(
                "Couldn't find namespace from search: {:?} {:?}",
                db_search.tag_one, namespaceone_unwrap
            ));
            return;
        }
        let _namespaceone = namespaceone_unwrap.unwrap();
        // let tagtwo = namespacetwo_unwrap.unwrap();
    }

    /// Parses data from search query to return an id
    fn search_for_namespace(&self, search: &sharedtypes::DbSearchObject) -> Option<usize> {
        match &search.namespace {
            None => search.namespace_id,
            Some(id_string) => self.namespace_get(id_string).copied(),
        }
    }

    /// Querys the db use this for select statements. NOTE USE THIS ONY FOR RESULTS
    /// THAT RETURN STRINGS
    pub fn quer_str(&mut self, inp: String) -> Result<Vec<String>> {
        let conmut = self._conn.borrow_mut();
        let binding = conmut.lock().unwrap();
        let mut toexec = binding.prepare(&inp).unwrap();
        let rows = toexec.query_map([], |row| row.get(0)).unwrap();
        let mut out = Vec::new();
        for each in rows {
            out.push(each.unwrap());
        }
        Ok(out)
    }

    /// Querys the db use this for select statements. NOTE USE THIS ONY FOR RESULTS
    /// THAT RETURN INTS
    pub fn quer_int(&mut self, inp: String) -> Result<Vec<isize>> {
        let conmut = self._conn.borrow_mut();
        let binding = conmut.lock().unwrap();
        let mut toexec = binding.prepare(&inp).unwrap();
        let rows = toexec.query_map([], |row| row.get(0)).unwrap();
        let mut out: Vec<isize> = Vec::new();
        for each in rows {
            match each {
                Ok(temp) => {
                    out.push(temp);
                }
                Err(errer) => {
                    error!("Could not load {} Due to error: {:?}", &inp, errer);
                }
            }
        }
        Ok(out)
    }

    /// Raw Call to database. Try to only use internally to this file only. Doesn't
    /// support params nativly. Will not write changes to DB. Have to call write().
    /// Panics to help issues.
    fn execute(&mut self, inp: String) -> usize {
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(&inp, params![]);
        match _out {
            Err(_out) => {
                println!("SQLITE STRING:: {}", inp);
                println!("BAD CALL {}", _out);
                error!("BAD CALL {}", _out);
                panic!("BAD CALL {}", _out);
            }
            Ok(_out) => _out,
        }
    }

    /*/// Deletes an item from jobs table. critera is the searchterm and collumn is the
    /// collumn to target. Doesn't remove from inmemory database
    pub fn del_from_jobs_table(&mut self, job: &sharedtypes::JobScraper) {
        self.del_from_jobs_table_sql(&job.site, &job.param);
    }*/

    /// Removes a job from the database by id. Removes from both memdb and sql.
    pub fn del_from_jobs_byid(&mut self, id: Option<&usize>) {
        if let Some(id) = id {
            self.del_from_jobs_inmemdb(id);
            self.del_from_jobs_table_sql_better(id);
        }
    }

    /// Removes job from inmemdb Removes by id
    fn del_from_jobs_inmemdb(&mut self, id: &usize) {
        self._inmemdb.jobref_remove(id)
    }

    /// Removes a job from sql table by id
    fn del_from_jobs_table_sql_better(&mut self, id: &usize) {
        let inp = "DELETE FROM Jobs WHERE id = ?";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![id.to_string()])
            .unwrap();
    }

    /// Removes a tag from sql table by name and namespace
    fn del_from_tags_by_name_and_namespace(&mut self, name: &String, namespace: &String) {
        dbg!("Deleting", name, namespace);
        let inp = "DELETE FROM Tags WHERE name = ? AND namespace = ?";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![name, namespace])
            .unwrap();
    }

    /*/// Removes a job from the sql table
    fn del_from_jobs_table_sql(&mut self, site: &String, param: &String) {
        let mut delcommand = "DELETE FROM Jobs".to_string();

        // This is horribly shit code. Opens us up to SQL injection. I should change this
        // later WARNING
        delcommand += &format!(" WHERE site LIKE {} AND", site);
        delcommand += &format!(" WHERE param LIKE {:?};", param.replace("\"", "\'"));
        logging::info_log(&format!("Deleting job via: {}", &delcommand));
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM Jobs WHERE site LIKE ?1 AND param LIKE ?2",
                params![site, param],
            )
            .unwrap();
    }*/

    /// Handles transactional pushes.
    pub fn transaction_execute(trans: Transaction, inp: String) {
        trans.execute(&inp, params![]).unwrap();
    }

    /// Sqlite wrapper for deleteing a relationship from table.
    fn delete_relationship_sql(&mut self, file_id: &usize, tag_id: &usize) {
        logging::info_log(&format!(
            "Removing Relationship where fileid = {} and tagid = {}",
            file_id, tag_id
        ));

        let inp = "DELETE FROM Relationship WHERE fileid = ? AND tagid = ?";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![file_id.to_string(), tag_id.to_string()])
            .unwrap();
        self.db_commit_man();
    }

    /// Sqlite wrapper for deleteing a parent from table.
    fn delete_parent_sql(&mut self, tag_id: &usize, relate_tag_id: &usize) {
        let inp = "DELETE FROM Parents WHERE tag_id = ? AND relate_tag_id = ?";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![tag_id.to_string(), relate_tag_id.to_string()]);
        self.db_commit_man();
    }

    /// Sqlite wrapper for deleteing a tag from table.
    fn delete_tag_sql(&mut self, tag_id: &usize) {
        let inp = "DELETE FROM Tags WHERE id = ?";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![tag_id.to_string()]);
        self.db_commit_man();
    }

    /// Sqlite wrapper for deleteing a tag from table.
    pub fn delete_namespace_sql(&mut self, namespace_id: &usize) {
        logging::info_log(&format!(
            "Deleting namespace with id : {} from db",
            namespace_id
        ));
        let inp = "DELETE FROM Namespace WHERE id = ?";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![namespace_id.to_string()]);
        self.db_commit_man();
    }

    /// Removes tag & relationship from db.
    pub fn delete_tag_relationship(&mut self, tag_id: &usize) {
        // self.transaction_flush();
        let relationships = self.relationship_get_fileid(tag_id);

        // Gets list of fileids from internal db.

        for _fileids in relationships.iter() {
            logging::log(&format!(
                "Found {} relationships's effected for tagid: {}.",
                relationships.len(),
                tag_id
            ));

            // let mut sql = String::new();
            for file_id in relationships.clone() {
                logging::log(&format!(
                    "Removing file: {} tagid: {} from db.",
                    file_id, tag_id
                ));

                logging::log(&"Removing relationship sql".to_string());

                self.relationship_remove(&file_id, tag_id);
            }

            // self._conn.lock().unwrap().execute_batch(&sql).unwrap();
            logging::log(&"Relationship Loop".to_string());
            // self.transaction_flush();
            self.db_commit_man();
        }
    }

    ///
    /// Removes a relationship based on fileid and tagid
    ///
    pub fn relationship_remove(&mut self, file_id: &usize, tag_id: &usize) {
        self._inmemdb.relationship_remove(file_id, tag_id);
        self.delete_relationship_sql(file_id, tag_id);
    }

    /// Removes tag from inmemdb and sql database.
    pub fn tag_remove(&mut self, id: &usize) {
        self._inmemdb.tag_remove(id);
        self.delete_tag_sql(id);
        let rel = &self._inmemdb.parents_remove(id);
        for each in rel {
            println!("Removing Parent: {} {}", each.0, each.1);
            self.delete_parent_sql(&each.0, &each.1);
        }
    }

    ///
    /// Removes parent from db
    ///
    pub fn parents_tagid_remove(&mut self, tag_id: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        self.parents_delete_tag_id_sql(tag_id);
        self._inmemdb.parents_tagid_remove(tag_id)
    }

    ///
    /// Wrapper for inmemdb
    ///
    pub fn parents_reltagid_remove(
        &mut self,
        reltag: &usize,
    ) -> HashSet<sharedtypes::DbParentsObj> {
        self.parents_delete_relate_tag_id_sql(reltag);
        self._inmemdb.parents_reltagid_remove(reltag)
    }

    pub fn parents_limitto_remove(
        &mut self,
        limit_to: Option<usize>,
    ) -> HashSet<sharedtypes::DbParentsObj> {
        if let Some(limit_to) = limit_to {
            self.parents_delete_limit_to_sql(&limit_to);
            self._inmemdb.parents_limitto_remove(&limit_to)
        } else {
            HashSet::new()
        }
    }

    /// Removes a parent selectivly
    pub fn parents_selective_remove(&mut self, parentobj: &sharedtypes::DbParentsObj) {
        self._inmemdb.parents_selective_remove(parentobj);
    }

    /// Deletes namespace by id Removes tags & relationships assocated.
    pub fn namespace_delete_id(&mut self, id: &usize) {
        logging::info_log(&format!("Starting deletion work on namespace id: {}", id));

        // self.vacuum(); self._conn.lock().unwrap().execute("create index ffid on
        // Relationship(fileid);", []);
        self.transaction_flush();
        if self.namespace_get_string(id).is_none() {
            logging::info_log(&"Stopping because I cannot get ns string.".to_string());
            return;
        }
        let tagids = self.namespace_get_tagids(id);
        for each in tagids.clone().iter() {
            self.tag_remove(each);

            // tag_sql += &format!("DELETE FROM Tags WHERE id = {}; ", each);
            self.delete_tag_relationship(each);
        }

        // elf._conn.lock().unwrap().execute_batch(&tag_sql).unwrap();
        // self.transaction_flush();
        self._inmemdb.namespace_delete(id);
        self.delete_namespace_sql(id);
    }

    /// Retuns namespace id's
    pub fn namespace_keys(&self) -> Vec<usize> {
        self._inmemdb.namespace_keys()
    }

    ///
    /// Condenses and corrects file locations
    ///
    pub fn condense_file_locations(&mut self) {
        self.load_table(&sharedtypes::LoadDBTable::Files);

        /*let mut location_list = HashSet::new();
        let mut bad_fids = HashSet::new();

        info_log(&format!("Starting to correct bad fids"));
        for fid in self.file_get_list_id().iter() {
            if let Some(_) = self.get_file(fid) {
                let location = self.file_get_id(fid).unwrap().location;
                location_list.insert(location);
            } else {
                bad_fids.insert(fid);
            }
        }

        for fid in bad_fids {
            let file = self.file_get_id(fid);
        }
        info_log(&format!("Finished correcting bad fids"));*/

        let mut file_id_list: Vec<usize> = self.file_get_list_all().clone().into_keys().collect();

        file_id_list.par_sort_unstable();

        let mut cnt = 0;
        for key in file_id_list {
            if cnt != key {
                dbg!("mismatch", &key, &cnt);
            }
            cnt += 1;
        }
    }

    ///
    /// Condenses tag ids into a solid column
    ///
    pub fn condense_tags(&mut self) {
        self.load_table(&sharedtypes::LoadDBTable::Tags);

        // Stopping automagically updating the db
        let commit_storage = self._dbcommitnum_static;
        self._dbcommitnum_static = None;

        let tag_max = self.tags_max_id();

        let mut flag = false;

        logging::info_log(&"Starting preliminary tags scanning".to_string());
        for id in 0..tag_max - 1 {
            if self.tag_id_get(&id).is_none() {
                logging::info_log(&format!(
                    "Disjointed tags detected. initting fixing badid: {}",
                    &id
                ));
                flag = true;
                break;
            }
        }

        if flag {
            logging::info_log(&"Started loading files, relationship and parents table".to_string());
            self.load_table(&sharedtypes::LoadDBTable::Files);
            self.load_table(&sharedtypes::LoadDBTable::Relationship);
            self.load_table(&sharedtypes::LoadDBTable::Parents);
            flag = false;
        }

        let mut cnt: usize = 0;
        let mut eta = Eta::new(tag_max, TimeAcc::MILLI);
        let count_percent = tag_max.div_ceil(100 / 2);
        for id in 0..tag_max + 1 {
            /*if id == 719 {
                self.transaction_flush();
                break;
            }*/
            let tagnns = self.tag_id_get(&id);
            match tagnns {
                None => {
                    flag = true;
                }
                Some(tag) => {
                    if flag {
                        logging::log(&format!("Moving tagid: {} to {}", &id, &cnt));
                        self.tag_add(&tag.name.clone(), tag.namespace, true, Some(cnt));
                        let fileids = self.relationship_get_fileid(&id);
                        for file_id in fileids {
                            self.relationship_add(file_id, cnt, true);
                            self.relationship_remove(&file_id, &id);
                        }

                        // Removes parent by ID and readds it with the new id
                        for parent in self.parents_tagid_remove(&id) {
                            logging::log(&format!(
                                "T Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                                parent.tag_id,
                                parent.relate_tag_id,
                                parent.limit_to,
                                cnt,
                                parent.relate_tag_id,
                                parent.limit_to
                            ));
                            self.parents_add(cnt, parent.relate_tag_id, parent.limit_to, true);
                        }

                        // Removes parent by ID and readds it with the new id
                        for parent in self.parents_reltagid_remove(&id) {
                            logging::log(&format!(
                                "R Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                                parent.tag_id,
                                parent.relate_tag_id,
                                parent.limit_to,
                                parent.tag_id,
                                cnt,
                                parent.limit_to
                            ));
                            self.parents_add(parent.tag_id, cnt, parent.limit_to, true);
                        }
                        // Kinda hacky but nothing bad will happen if we have nothing in the limit
                        // slot
                        for parent in self.parents_limitto_remove(Some(id)) {
                            logging::log(&format!(
                                "L Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                                parent.tag_id,
                                parent.relate_tag_id,
                                parent.limit_to,
                                parent.tag_id,
                                parent.relate_tag_id,
                                cnt
                            ));
                            self.parents_add(parent.tag_id, parent.relate_tag_id, Some(cnt), true);
                        }

                        self.tag_remove(&id);
                    }
                    cnt += 1;
                }
            }
            eta.step();
            if (cnt % count_percent) == 0 || cnt == 10 || cnt == tag_max {
                logging::info_log(&format!("{}", &eta));
            }
        }

        self.transaction_flush();
        self._dbcommitnum_static = commit_storage;
    }

    pub fn condense_namespace(&mut self) {
        self.load_table(&sharedtypes::LoadDBTable::Namespace);
        self.load_table(&sharedtypes::LoadDBTable::Tags);

        //self.namespace_delete_id(&2);

        for namespace_id in self.namespace_keys() {
            if self.namespace_get_tagids(&namespace_id).is_empty() {
                dbg!(&namespace_id);
            }
        }
    }

    /// Condesnes relationships between tags & files. Changes tag id's removes spaces
    /// inbetween tag id's and their relationships.
    /// TODO FIX THIS FUNCTION TEMPORARILY DEPRECATING
    pub fn condense_db_all(&mut self) {
        self.condense_namespace();
        //self.condense_tags();
        //self.condense_file_locations();
        self.vacuum();
    }

    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(&self, id: &usize) -> HashSet<usize> {
        self._inmemdb.namespace_get_tagids(id)
    }

    /// Checks if a tag exists in a namespace
    //pub fn namespace_contains_id(&self, namespace_id: &usize) -> bool {
    pub fn namespace_contains_id(&self, namespace_id: &usize, tag_id: &usize) -> bool {
        self.namespace_get_tagids(namespace_id).contains(tag_id)
    }

    /// Recreates the db with only one ns in it.
    pub fn drop_recreate_ns(&mut self, id: &usize) {
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
    }

    /// Returns all file id's loaded in db
    pub fn file_get_list_id(&self) -> HashSet<usize> {
        self._inmemdb.file_get_list_id()
    }

    /// Returns all file objects in db.
    pub fn file_get_list_all(&self) -> &HashMap<usize, sharedtypes::DbFileStorage> {
        let out = self._inmemdb.file_get_list_all();
        out
    }

    /// Returns the location of the DB. Helper function
    pub fn location_get(&self) -> String {
        self.settings_get_name(&"FilesLoc".to_string())
            .unwrap()
            .param
            .as_ref()
            .unwrap()
            .to_owned()
    }
}

#[cfg(test)]
pub(crate) mod test_database {
    use super::*;
    use crate::VERS;

    pub fn setup_default_db() -> Main {
        let mut db = Main::new(None, VERS);
        db.parents_add(1, 2, Some(3), true);
        db.parents_add(2, 3, Some(4), true);
        db.parents_add(3, 4, Some(5), true);
        db.tag_add(&"test".to_string(), 1, false, None);
        db.tag_add(&"test1".to_string(), 1, false, None);
        db.tag_add(&"test2".to_string(), 1, false, None);
        db
    }

    #[test]
    fn db_parents_tagid_remove() {
        let mut main = setup_default_db();
        main.parents_tagid_remove(&1);
        assert_eq!(main.parents_rel_get(&1), None);
        assert_eq!(main.parents_tag_get(&1), None);
    }

    #[test]
    fn db_test_load_tags() {
        let mut main = setup_default_db();
        assert_eq!(main._inmemdb.tags_max_return(), 3);
        let id = main.tag_add(&"test3".to_string(), 3, false, None);
        assert_eq!(id, 3);

        assert!(main.tag_id_get(&2).is_some());
    }

    ///
    /// This test does not deduplicate data. We rely on the jobs obj to deduplicate
    ///
    #[test]
    fn db_jobs_check_id() {
        let mut main = setup_default_db();
        dbg!(main.jobs_get_max());
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
        assert_eq!(main.jobs_get_max(), &1);
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
        assert_eq!(main.jobs_get_max(), &2);
        assert!(main.jobs_get(&1).is_some());
    }

    #[test]
    fn db_dead_jobs() {
        let mut main = Main::new(Some("test1.db".to_string()), VERS);
        main.add_dead_url(&"test".to_string());

        assert!(main.check_dead_url(&"test".to_string()));
        assert!(!main.check_dead_url(&"Null".to_string()));

        std::fs::remove_file("test1.db").unwrap();
    }
}
