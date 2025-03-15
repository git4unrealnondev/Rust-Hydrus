#![forbid(unsafe_code)]
use crate::logging;
use crate::logging::info_log;
use crate::plugins;
use crate::plugins::PluginManager;
use crate::scraper::ScraperManager;
use crate::sharedtypes;
use log::{error, info};
use rayon::prelude::*;
pub use rusqlite::types::ToSql;
use rusqlite::OptionalExtension;
pub use rusqlite::{params, types::Null, Connection, Result, Transaction};
use std::borrow::BorrowMut;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::panic;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

mod db;

use self::db::helpers;
use crate::database::db::inmemdbnew::NewinMemDB;

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
    _dbpath: String,
    pub _conn: Arc<Mutex<Connection>>,
    _vers: usize,
    _active_vers: usize,
    // inmem db with ahash low lookup/insert time. Alernative to hashmap
    _inmemdb: NewinMemDB,
    _dbcommitnum: usize,
    _dbcommitnum_static: usize,
    _tables_loaded: Vec<sharedtypes::LoadDBTable>,
    _tables_loading: Vec<sharedtypes::LoadDBTable>,
    _cache: CacheType,
    _pluginmanager: Option<Arc<Mutex<PluginManager>>>,
}

/// Contains DB functions.
impl Main {
    /// Sets up new db instance.
    pub fn new(path: String, vers: usize) -> Self {
        // Initiates two connections to the DB. Cheap workaround to avoid loading errors.
        let dbexist = Path::new(&path).exists();
        let connection = dbinit(&path);

        // let conn = connection;
        let memdb = NewinMemDB::new();

        // let path = String::from("./main.db");
        let memdbmain = Main {
            _dbpath: path.to_owned(),
            _conn: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            _vers: vers,
            _active_vers: 0,
            _inmemdb: memdb,
            _dbcommitnum: 0,
            _dbcommitnum_static: 3000,
            _tables_loaded: vec![],
            _tables_loading: vec![],
            _cache: CacheType::Bare(path.clone()),
            _pluginmanager: None,
        };
        let mut main = Main {
            _dbpath: path,
            _conn: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            _vers: vers,
            _active_vers: 0,
            _inmemdb: memdbmain._inmemdb,
            _dbcommitnum: 0,
            _dbcommitnum_static: 3000,
            _tables_loaded: vec![],
            _tables_loading: vec![],
            _cache: CacheType::InMemdb,
            _pluginmanager: None,
        };
        main._conn = Arc::new(Mutex::new(connection));

        // Sets default settings for db settings.
        main.db_open();
        if !dbexist {
            // Database Doesn't exist
            main.transaction_start();
            main.first_db();
            main.updatedb();
            main.db_commit_man_set();
        } else {
            // Database does exist.
            main.transaction_start();
            println!("Database Exists: {} : Skipping creation.", dbexist);
            info!("Database Exists: {} : Skipping creation.", dbexist);
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

        let current_date = Utc::now();
        let year = current_date.year();
        let month = current_date.month();
        let day = current_date.day();
        let dbbackuploc = self.settings_get_name(&"db_backup_location".to_string());

        // Default location for the DB
        let defaultloc = String::from("dbbackup");

        // Gets the DB file location for copying.
        let dbloc = self.get_db_loc();
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
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Option<HashSet<usize>> {
        let mut stor: Vec<sharedtypes::SearchHolder> = Vec::with_capacity(search.searches.len());
        let mut fin: HashSet<usize> = HashSet::new();
        let mut fin_temp: HashMap<usize, HashSet<usize>> = HashMap::new();
        let mut searched: Vec<(usize, usize)> = Vec::with_capacity(search.searches.len());
        if search.search_relate.is_none() {
            if search.searches.len() == 1 {
                stor.push(sharedtypes::SearchHolder::AND((0, 0)));
            } else {
                // Assume AND search
                for each in 0..search.searches.len() {
                    stor.push(sharedtypes::SearchHolder::AND((each, each + 1)));
                }
            }
        } else {
            stor = search.search_relate.unwrap();
        }
        let mut cnt = 0;
        for un in search.searches {
            match un {
                sharedtypes::SearchHolder::NOT((a, b)) => {
                    let fa = self.relationship_get_fileid(&a);
                    let fb = self.relationship_get_fileid(&b);
                    if let Some(fa) = fa {
                        if let Some(fb) = fb {
                            fin_temp.insert(cnt, fa.difference(&fb).cloned().collect());
                        }
                    }
                    searched.push((cnt, a));
                    searched.push((cnt, b));
                }
                sharedtypes::SearchHolder::AND((a, b)) => {
                    if let Some(fa) = self.relationship_get_fileid(&a) {
                        if let Some(fb) = self.relationship_get_fileid(&b) {
                            fin_temp.insert(cnt, fa.intersection(&fb).cloned().collect());
                        }
                    }
                    searched.push((cnt, a));
                    searched.push((cnt, b));
                }
                sharedtypes::SearchHolder::OR((a, b)) => {
                    let fa = self.relationship_get_fileid(&a);
                    let fb = self.relationship_get_fileid(&b);
                    if let Some(fa) = &fa {
                        if let Some(fb) = &fb {
                            fin_temp.insert(cnt, fa.union(fb).cloned().collect());
                        }
                    }
                    searched.push((cnt, a));
                    searched.push((cnt, b));
                }
            }
            cnt += 1
        }
        for each in stor {
            match each {
                sharedtypes::SearchHolder::OR((_a, _b)) => {}
                sharedtypes::SearchHolder::AND((a, b)) => {
                    let fa = fin_temp.get(&a).unwrap();
                    let fb = fin_temp.get(&b).unwrap();
                    let tem = fa.intersection(fb);
                    for each in tem {
                        fin.insert(*each);
                    }
                }
                sharedtypes::SearchHolder::NOT((_a, _b)) => {}
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

    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(&self, tag: &usize) -> Option<HashSet<usize>> {
        self._inmemdb.relationship_get_fileid(tag)
    }

    pub fn relationship_get_one_fileid(&self, tag: &usize) -> Option<&usize> {
        self._inmemdb.relationship_get_one_fileid(tag)
    }

    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(&self, tag: &usize) -> Option<HashSet<usize>> {
        self._inmemdb.relationship_get_tagid(tag)
    }

    pub fn settings_get_name(&self, name: &String) -> Option<&sharedtypes::DbSettingObj> {
        self._inmemdb.settings_get_name(name)
    }

    /// Returns total jobs from _inmemdb
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

    /// Pulls data of table into form. Parses Data
    ///
    /// Pulls collums info -> (Vec`<String>`, Vec`<String>` SELECT sql FROM
    /// sqlite_master WHERE tbl_name='File' AND type = 'table';
    pub fn table_collumns(&mut self, table: String) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut t1: Vec<String> = Vec::new();
        let mut t2: Vec<String> = Vec::new();
        let parsedstring = format!(
            "SELECT sql FROM sqlite_master WHERE tbl_name='{}' AND type = 'table';",
            &table.as_str()
        );
        let conmut = self._conn.borrow_mut();
        let binding = conmut.lock().unwrap();
        let mut toexec = binding.prepare(&parsedstring).unwrap();
        let mut outvec = Vec::new();
        let mut outs = toexec.query(params![]).unwrap();
        while let Some(out) = outs.next().unwrap() {
            let g1: String = out.get(0).unwrap();
            let g2: Vec<&str> = g1.split('(').collect();
            let g3: Vec<&str> = g2[1].split(')').collect();
            let g4: Vec<&str> = g3[0].split(", ").collect();
            for e in &g4 {
                let e1: Vec<&str> = e.split(' ').collect();
                t1.push(e1[0].to_string());
                t2.push(e1[1].to_string());
            }
            outvec.push(g1);
        }
        (outvec, t1, t2)
    }

    /// Get table names Returns: Vec of strings
    pub fn table_names(&mut self) -> Vec<String> {
        let conmut = self._conn.borrow_mut();
        let binding = conmut.lock().unwrap();
        let mut toexec = binding
            .prepare("SELECT name FROM sqlite_master WHERE type='table';")
            .unwrap();
        let mut outvec = Vec::new();
        let mut outs = toexec.query(params![]).unwrap();

        // println!("{:?}", out);
        while let Some(out) = outs.next().unwrap() {
            let vecpop = out.get(0).unwrap();
            outvec.push(vecpop);
        }

        // println!("{:?}", outvec);
        outvec
    }

    /// Sets up first database interaction. Makes tables and does first time setup.
    pub fn first_db(&mut self) {
        // Checking if file exists. If doesn't then no write perms.
        let dbexists = Path::new(&self.get_db_loc()).exists();
        if !dbexists {
            panic!("No database write perms or file not created");
        }

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
        keys = vec_of_strings![
            "id",
            "time",
            "reptime",
            "Manager",
            "site",
            "param",
            "CommitType",
            "SystemData",
            "UserData"
        ];
        vals = vec_of_strings![
            "INTEGER", "INTEGER", "INTEGER", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT"
        ];
        self.table_create(&name, &keys, &vals);

        self.enclave_create_database_v5();

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
        info!("Set VERSION to 1.");
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
            Some("./Plugins/".to_string()),
            true,
        );

        self.enclave_create_default_file_download(self.location_get());

        self.transaction_flush();
    }

    ///
    /// Gets a default namespace id if it doesn't exist
    ///
    pub fn create_default_source_url_ns_id(&mut self) -> usize {
        match self.namespace_get(&"source_url".to_string()).cloned() {
            None => self
                .namespace_add(
                    "source_url".to_string(),
                    Some("Source URL for a file.".to_string()),
                    true,
                )
                .clone(),
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
    pub fn check_version(&mut self, scraper_manager: &mut ScraperManager) -> bool {
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
            logging::panic_log(&format!(
                "check_version: Could not load DB properly PANICING!!!"
            ));
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

            if db_vers == 1 && self._vers == 2 {
                panic!("How did you get here vers is 1 did you do something dumb??")
            } else if db_vers + 1 == 3 {
                dbg!(self._vers, self._active_vers);
                self.db_update_two_to_three();
                db_vers += 1;
            } else if db_vers + 1 == 4 {
                self.db_update_three_to_four();
            } else if db_vers + 1 == 5 {
                self.db_update_four_to_five();
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

    ///
    /// Gets a file storage location id
    ///
    pub fn storage_get_id(&self, location: &String) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from FileStorageLocations where location = ?",
                params![location],
                |row| row.get(0),
            )
            .optional()
        {
            out
        } else {
            None
        }
    }

    ///
    /// Gets a string from the ID of the storage location
    ///
    pub fn storage_get_string(&self, id: &usize) -> Option<String> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT location from FileStorageLocations where id = ?",
                params![id],
                |row| row.get(0),
            )
            .optional()
        {
            out
        } else {
            None
        }
    }

    ///
    /// Inserts into storage the location
    ///
    pub fn storage_put(&mut self, location: &String) {
        if self.storage_get_id(location).is_some() {
            return;
        }
        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO FileStorageLocations (location) VALUES (?)")
            .unwrap();
        let _ = prep.insert(params![location]);
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
            Some(id) => id.clone(),
            None => {
                let conn = self._conn.lock().unwrap();
                conn.execute(
                    "insert or ignore into FileExtensions(extension) VALUES (?)",
                    params![ext],
                );
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
                    if id.is_some() && extension.is_some() {
                        self.extension_load(id.unwrap(), extension.unwrap());
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
                    self.namespace_add_db(res);
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
                    let manager: String = row.get(3).unwrap();
                    let man = serde_json::from_str(&manager).unwrap();
                    let system_data_string: String = row.get(7).unwrap();
                    let user_data_string: String = row.get(8).unwrap();
                    let system_data = serde_json::from_str(&system_data_string).unwrap();
                    let user_data = serde_json::from_str(&user_data_string).unwrap();
                    Ok(sharedtypes::DbJobsObj {
                        id: row.get(0).unwrap(),
                        time: row.get(1).unwrap(),
                        reptime: row.get(2).unwrap(),
                        site: row.get(4).unwrap(),
                        param: row.get(5).unwrap(),
                        jobmanager: man,
                        committype: Some(sharedtypes::stringto_commit_type(&row.get(6).unwrap())),
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

    /// Loads Parents in from DB Connection
    fn load_parents(&mut self) {
        logging::info_log(&"Database is Loading: Parents".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM Parents");
        if let Ok(mut con) = temp {
            let parents = con
                .query_map([], |row| {
                    Ok(sharedtypes::DbParentsObj {
                        tag_id: row.get(0).unwrap(),
                        relate_tag_id: row.get(1).unwrap(),
                        limit_to: row.get(2).unwrap(),
                    })
                })
                .unwrap();
            for each in parents {
                if let Ok(res) = each {
                    self.parents_add_db(res);
                } else {
                    error!("Bad Parent cant load {:?}", each);
                }
            }
        }
    }

    /// Loads Relationships in from DB connection
    fn load_relationships(&mut self) {
        logging::info_log(&"Database is Loading: Relationships".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM Relationship");
        if let Ok(mut con) = temp {
            let relationship = con
                .query_map([], |row| {
                    Ok(sharedtypes::DbRelationshipObj {
                        fileid: row.get(0).unwrap(),
                        tagid: row.get(1).unwrap(),
                    })
                })
                .unwrap();
            for each in relationship {
                match each {
                    Ok(res) => {
                        self.relationship_add_db(res.fileid, res.tagid);
                    }
                    Err(err) => {
                        error!("Bad relationship cant load");
                        err.to_string().contains("database disk image is malformed");
                        error!("DATABASE IMAGE IS MALFORMED PANICING rel {:?}", &err);
                        panic!("DATABASE IMAGE IS MALFORMED PANICING rel {:?}", &err);
                    }
                }
            }
        }
    }

    /// Loads settings into db
    fn load_settings(&mut self) {
        logging::info_log(&"Database is Loading: Settings".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM Settings");
        match temp {
            Ok(mut con) => {
                let settings = con
                    .query_map([], |row| {
                        Ok(sharedtypes::DbSettingObj {
                            name: row.get(0)?,
                            pretty: row.get(1)?,
                            num: row.get(2)?,
                            param: row.get(3)?,
                        })
                    })
                    .unwrap();
                for each in settings {
                    if let Ok(res) = each {
                        self.setting_add_db(res.name, res.pretty, res.num, res.param);
                    } else {
                        error!("Bad Setting cant load {:?}", each);
                    }
                }
            }
            Err(_) => return,
        };
        self.db_commit_man_set();
    }

    /// Loads tags into db
    fn load_tags(&mut self) {
        logging::info_log(&"Database is Loading: Tags".to_string());

        // let mut delete_tags = HashSet::new();
        {
            let binding = self._conn.clone();
            let temp_test = binding.lock().unwrap();
            let temp = temp_test.prepare("SELECT * FROM Tags");
            if let Ok(mut con) = temp {
                let tag = con.query_map([], |row| {
                    Ok(sharedtypes::DbTagObjCompatability {
                        id: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        namespace: row.get(2).unwrap(),
                    })
                });
                match tag {
                    Ok(tags) => {
                        for each in tags {
                            if let Ok(res) = each {
                                // if let Some(id) = self._inmemdb.tags_get_data(&res.id) {
                                // logging::info_log(&format!( "Already have tag {:?} adding {} {} {}", id,
                                // res.name, res.namespace, res.id )); continue;
                                // delete_tags.insert((res.name.clone(), res.namespace.clone())); }
                                self.tag_add(&res.name, res.namespace, false, Some(res.id));
                            } else {
                                error!("Bad Tag cant load {:?}", each);
                            }
                        }
                    }
                    Err(errer) => {
                        error!("WARNING COULD NOT LOAD TAG: {:?} DUE TO ERROR", errer);
                    }
                }
            }
        }
        // if self.check_table_exists("Tags_New".to_string()) {
        // self.db_drop_table(&"Tags_New".to_string()); self.transaction_flush(); }
        // self.table_create( &"Tags_New".to_string(), &[ "id".to_string(),
        // "name".to_string(), "namespace".to_string(), ] .to_vec(), &[
        // "INTEGER".to_string(), "TEXT".to_string(), "INTEGER".to_string(), ] .to_vec(),
        // ); { let conn = self._conn.lock().unwrap(); let mut stmt = conn
        // .prepare("INSERT INTO Tags_New (id,name,namespace) VALUES (?1,?2,?3)")
        // .unwrap();
        //
        // for i in 0..self._inmemdb.tags_max_return() { if let Some(taginfo) =
        // self._inmemdb.tags_get_data(&i) { stmt.execute((i, taginfo.name.clone(),
        // taginfo.namespace)) .unwrap(); } } } self.db_drop_table(&"Tags".to_string());
        // if !self.check_table_exists("Tags".to_string()) {
        // self.alter_table(&"Tags_New".to_string(), &"Tags".to_string()); }
        // self.transaction_flush();
    }

    /// Sets advanced settings for journaling. NOTE Experimental badness
    pub fn db_open(&mut self) {
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(&"PRAGMA secure_delete = 0;", params![]);
        // self.execute("PRAGMA journal_mode = MEMORY".to_string()); self.execute("PRAGMA
        // synchronous = OFF".to_string()); info!("Setting synchronous = OFF");
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
    fn db_commit_man(&mut self) {
        self._dbcommitnum += 1;

        // let result_general = self.settings_get_name(&"DBCOMMITNUM".to_string());
        if self._dbcommitnum >= self._dbcommitnum_static {
            logging::info_log(&format!(
                "Flushinbg {} Records into DB.",
                self._dbcommitnum_static
            ));

            // dbg!(self._dbcommitnum, general);
            self.transaction_flush();
            // dbg!(self._dbcommitnum, general);
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
        self._dbcommitnum_static = self
            .settings_get_name(&"DBCOMMITNUM".to_string())
            .unwrap()
            .num
            .unwrap();
    }

    /// Adds file via SQL
    fn file_add_sql(
        &mut self,
        hash: &Option<String>,
        extension: &Option<usize>,
        storage_id: &Option<usize>,
        file_id: &usize,
    ) {
        let inp = "INSERT INTO File VALUES(?, ?, ?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![file_id, hash, extension, storage_id]);
        self.db_commit_man();
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
                            &Some(file_obj.ext_id.clone()),
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
                            &Some(noid_obj.ext_id.clone()),
                            &Some(noid_obj.storage_id),
                            &id,
                        );
                    }
                    id
                } else {
                    self.file_get_hash(&noid_obj.hash).unwrap().clone()
                }
            }
            sharedtypes::DbFileStorage::NoExist(_) => {
                panic!();
                let id = self.file_add_db(file.clone());
                if addtodb {
                    self.file_add_sql(&None, &None, &None, &id);
                }
                id
            }
            sharedtypes::DbFileStorage::NoExistUnknown => {
                panic!();
                let id = self.file_add_db(file.clone());
                if addtodb {
                    self.file_add_sql(&None, &None, &None, &id);
                }
                id
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

    /// Adds namespace to the SQL database
    fn namespace_add_sql(&mut self, name: &String, description: &Option<String>, name_id: &usize) {
        let inp = "INSERT INTO Namespace VALUES(?, ?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![name_id, name, description]);
        self.db_commit_man();
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

    /// Adds tags into sql database
    fn tag_add_sql(&mut self, tag_id: &usize, tags: &String, namespace: &usize) {
        let inp = "INSERT INTO Tags VALUES(?, ?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![tag_id, tags, namespace]);
        self.db_commit_man();
    }

    /// Prints db info
    pub fn debugdb(&self) {
        self._inmemdb.dumpe_data();
    }

    pub fn setup_pluginmanager(&mut self, pluginmanager: Arc<Mutex<PluginManager>>) {
        self._pluginmanager = Some(pluginmanager);
    }

    /// Adds tag into DB if it doesn't exist in the memdb.
    pub fn tag_add(
        &mut self,
        tags: &String,
        namespace: usize,
        addtodb: bool,
        id: Option<usize>,
    ) -> usize {
        match id {
            None => {
                if let Some(pluginmanager) = &self._pluginmanager {
                    plugins::plugin_on_tag(pluginmanager.clone(), self, &tags, &namespace);
                }

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
            Some(id_some) => {
                // We've got an ID coming in will check if it exists.
                let tag_id = self.tag_add_db(tags, &namespace, id);

                // println!( "test: {} {} {:?} {}", &tag_id, tagnns.name.clone(), id.clone(),
                // &tagnns.namespace );
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
        let inp = "INSERT INTO Jobs VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)";
        let _out = self._conn.borrow_mut().lock().unwrap().execute(
            inp,
            params![
                data.id.to_string(),
                data.time.unwrap().to_string(),
                data.reptime.unwrap().to_string(),
                serde_json::to_string(&data.jobmanager).unwrap(),
                data.site,
                data.param,
                data.committype.unwrap().to_string(),
                serde_json::to_string(&data.system_data).unwrap(),
                serde_json::to_string(&data.user_data).unwrap()
            ],
        );
        self.db_commit_man();
    }

    /// Updates job by id
    fn jobs_update_by_id(&mut self, data: &sharedtypes::DbJobsObj) {
        let inp =
            "UPDATE Jobs SET id=?, time=?, reptime=?, Manager=?, site=?, param=?, CommitType=?, SystemData=?, UserData=? WHERE id = ?";
        let _out = self._conn.borrow_mut().lock().unwrap().execute(
            inp,
            params![
                data.id.to_string(),
                data.time.unwrap().to_string(),
                data.reptime.unwrap().to_string(),
                serde_json::to_string(&data.jobmanager).unwrap(),
                data.site,
                data.param,
                data.committype.unwrap().to_string(),
                serde_json::to_string(&data.system_data).unwrap(),
                serde_json::to_string(&data.user_data).unwrap(),
                data.id.to_string()
            ],
        );
        self.db_commit_man();
    }

    pub fn jobs_add(
        &mut self,
        id: Option<usize>,
        time: usize,
        reptime: usize,
        site: String,
        param: String,
        addtodb: bool,
        committype: sharedtypes::CommitType,
        dbjobtype: &sharedtypes::DbJobType,
        system_data: BTreeMap<String, String>,
        user_data: BTreeMap<String, String>,
        jobsmanager: sharedtypes::DbJobsManager,
    ) -> usize {
        let id = match id {
            None => self._inmemdb.jobs_get_max().clone(),
            Some(id) => id,
        };
        let params = if param.is_empty() {
            None
        } else {
            Some(param.clone())
        };
        let jobs_obj: sharedtypes::DbJobsObj = sharedtypes::DbJobsObj {
            id,
            time: Some(time),
            reptime: Some(reptime),
            site: site.clone(),
            param: params.clone(),
            jobmanager: jobsmanager.clone(),
            committype: Some(committype),
            isrunning: false,
            system_data: system_data.clone(),
            user_data: user_data.clone(),
        };
        self.jobs_update_db(jobs_obj.clone());

        // let jobsmanager = sharedtypes::DbJobsManager { jobtype: *dbjobtype, recreation:
        // None, additionaldata: None, };
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
    pub fn get_db_loc(&self) -> String {
        self._dbpath.to_string()
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

    /// Deletes an item from jobs table. critera is the searchterm and collumn is the
    /// collumn to target. Doesn't remove from inmemory database
    pub fn del_from_jobs_table(&mut self, job: &sharedtypes::JobScraper) {
        self.del_from_jobs_table_sql(&job.site, &job.original_param);
    }

    /// Removes a job from the database by id. Removes from both memdb and sql.
    pub fn del_from_jobs_byid(&mut self, id: &usize) {
        self.del_from_jobs_inmemdb(id);
        self.del_from_jobs_table_sql_better(id);
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

    /// Removes a job from the sql table
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
    }

    /// Handles transactional pushes.
    pub fn transaction_execute(trans: Transaction, inp: String) {
        trans.execute(&inp, params![]).unwrap();
    }

    /// Sqlite wrapper for deleteing a relationship from table.
    fn delete_relationship_sql(&mut self, file_id: &usize, tag_id: &usize) {
        let inp = "DELETE FROM Relationship WHERE fileid = ? AND tagid = ?";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![file_id.to_string(), tag_id.to_string()])
            .unwrap();
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
    }

    /// Removes tag & relationship from db.
    pub fn delete_tag_relationship(&mut self, tag_id: &usize) {
        // self.transaction_flush();
        let relationships = self.relationship_get_fileid(tag_id);

        // Gets list of fileids from internal db.
        match relationships {
            None => (),
            Some(fileids) => {
                logging::log(&format!(
                    "Found {} relationships's effected for tagid: {}.",
                    fileids.len(),
                    tag_id
                ));

                // let mut sql = String::new();
                for file_id in fileids.clone() {
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
            }
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

    /// Removes a parent item from system.
    pub fn parents_remove_id(&mut self, parent_id: &usize) {
        self._inmemdb.parents_remove_id(parent_id)
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
        let tagida = self.namespace_get_tagids(id);
        if let Some(tagids) = tagida {
            for each in tagids.clone().iter() {
                logging::log(&format!("Removing tagid: {}.", each));
                self.tag_remove(each);

                // tag_sql += &format!("DELETE FROM Tags WHERE id = {}; ", each);
                self.delete_tag_relationship(each);
            }

            // elf._conn.lock().unwrap().execute_batch(&tag_sql).unwrap();
            // self.transaction_flush();
            self._inmemdb.namespace_delete(id);
            self.delete_namespace_sql(id);

            // self.vacuum(); Condenses the database. (removes gaps in id's)
            self.condese_relationships_tags();
        }
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
    }

    /// Condesnes relationships between tags & files. Changes tag id's removes spaces
    /// inbetween tag id's and their relationships.
    /// TODO FIX THIS FUNCTION TEMPORARILY DEPRECATING
    pub fn condese_relationships_tags(&mut self) {
        self.load_table(&sharedtypes::LoadDBTable::Relationship);
        self.load_table(&sharedtypes::LoadDBTable::Parents);
        self.load_table(&sharedtypes::LoadDBTable::Tags);
        //logging::info_log(&"Starting compression of tags & relationships.".to_string());
        //logging::info_log(&"Backing up db this could be messy.".to_string());
        self.backup_db();
        let tag_max = self._inmemdb.tags_max_return();
        dbg!(&tag_max);
        /*self._inmemdb.tags_max_reset();
        let mut lastvalid: usize = 0;
        for tag_id in 0..tag_max + 1 {
            let exst = self._inmemdb.tags_get_data(&tag_id).cloned();
            match exst {
                None => {}
                Some(nns) => {
                    self.transaction_flush();
                    let mut relat_str = String::new();
                    let file_listop = self.relationship_get_fileid(&tag_id);
                    let mut file_to_add: HashSet<usize> = HashSet::new();
                    match file_listop {
                        None => {
                            self.tag_remove(&tag_id);
                            self.tag_add(&nns.name, nns.namespace, true, Some(lastvalid));
                            lastvalid += 1;
                            continue;
                        }
                        Some(file_list) => {
                            for file_id in file_list.clone() {
                                self.relationship_remove(&file_id, &tag_id);

                                file_to_add.insert(file_id);
                            }
                            self.transaction_flush();
                        }
                    }
                    self.tag_remove(&tag_id);
                    self.tag_add(&nns.name, nns.namespace, true, Some(lastvalid));
                    for fid in file_to_add {
                        self.relationship_add(fid, lastvalid, true);
                        // self._inmemdb.relationship_add(fid, tid);
                    }
                    lastvalid += 1;
                }
            }
        }
        self.vacuum();*/
    }

    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(&self, id: &usize) -> Option<&HashSet<usize>> {
        self._inmemdb.namespace_get_tagids(id)
    }

    /// Checks if a tag exists in a namespace
    pub fn namespace_contains_id(&self, namespace_id: &usize, tag_id: &usize) -> bool {
        if let Some(ns_data) = self.namespace_get_tagids(namespace_id) {
            return ns_data.contains(tag_id);
        }
        false
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
        if let Some(tids) = tida {
            let mut cnt: usize = 0;
            for tid in tids.clone() {
                let file_listop = self._inmemdb.relationship_get_fileid(&tid).unwrap().clone();
                let tag = self._inmemdb.tags_get_data(&tid).unwrap().clone();
                self.tag_add_sql(&cnt, &tag.name, &tag.namespace);
                for file in file_listop {
                    self.relationship_add_sql(file, cnt);
                }
                cnt += 1;
            }
            self._inmemdb.tags_clear();
            self._inmemdb.parents_clear();
            self._inmemdb.relationships_clear();
            self.vacuum();
            self.transaction_flush();

            // self.condese_relationships_tags();
            self.transaction_flush();
        }
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
