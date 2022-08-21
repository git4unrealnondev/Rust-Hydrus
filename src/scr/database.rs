#![forbid(unsafe_code)]

use ahash::AHashMap;
use log::{error, info};
//use rusqlite::ToSql;
use crate::vec_of_strings;
pub use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
//use rusqlite::OptionalExtension;
use crate::scr::time;
pub use rusqlite::{params, types::Null, Connection, Result, Transaction};
use std::panic;
use std::path::Path;

extern crate urlparse;
use urlparse::urlparse;

/// Returns an open connection to use.
pub fn dbinit(dbpath: &String) -> Connection {
    //Engaging Transaction Handling
    let res = Connection::open(&dbpath).unwrap();
    return res;
}

/// Holder of database self variables
pub struct Main {
    _dbpath: String,
    _conn: Connection,
    _vers: isize,
    // inmem db with ahash low lookup/insert time. Alernative to hashmap
    _inmemdb: Memdb,
}

/// Holds internal in memory hashmap stuff
#[allow(dead_code)]
struct Memdb {
    _table_names: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ),

    _file_max_id: u128,
    _file_hash: AHashMap<u128, String>,
    _file_filename: AHashMap<u128, String>,
    _file_size: AHashMap<u128, u64>,

    _jobs_max_id: u128,
    _jobs_time: AHashMap<u128, u128>,
    _jobs_rep: AHashMap<u128, u128>,
    _jobs_site: AHashMap<u128, String>,
    _jobs_param: AHashMap<u128, String>,

    _namespace_max_id: u128,
    _namesace_name: AHashMap<u128, String>,
    _namespace_description: AHashMap<u128, String>,

    _parents_max_id: u128,
    _parents_name: AHashMap<u128, String>,
    _parents_children: AHashMap<u128, String>,
    _parents_namespace: AHashMap<u128, u128>,

    _relationship_max_id: u128,
    _relationship_fileid: AHashMap<u128, u128>,
    _relationship_tagid: AHashMap<u128, u128>,

    _settings_max_id: u128,
    _settings_name: AHashMap<String, u128>,
    _settings_pretty: AHashMap<u128, String>,
    _settings_num: AHashMap<u128, u128>,
    _settings_param: AHashMap<u128, String>,

    _tags_max_id: u128,
    _tags_name: AHashMap<u128, String>,
    _tags_parents: AHashMap<u128, u128>,
    _tags_namespace: AHashMap<u128, u128>,
}

/// Functions for working with memorory db.
/// Uses AHash for maximum speed.
#[allow(dead_code)]
impl Memdb {
    pub fn new() -> Self {
        Memdb {
            _table_names: ("File", "Jobs", "Namespace", "Parents", "Settings", "Tags"),
            //_table_names: ,
            //_file_id: AHashMap::new(),
            _file_hash: AHashMap::new(),
            _file_filename: AHashMap::new(),
            _file_size: AHashMap::new(),
            _jobs_time: AHashMap::new(),
            _jobs_rep: AHashMap::new(),
            _jobs_site: AHashMap::new(),
            _jobs_param: AHashMap::new(),
            //_namespace_id: AHashMap::new(),
            _namesace_name: AHashMap::new(),
            _namespace_description: AHashMap::new(),
            //_parents_id: AHashMap::new(),
            _parents_name: AHashMap::new(),
            _parents_children: AHashMap::new(),
            _parents_namespace: AHashMap::new(),
            _relationship_fileid: AHashMap::new(),
            _relationship_tagid: AHashMap::new(),
            _settings_name: AHashMap::new(),
            _settings_pretty: AHashMap::new(),
            _settings_num: AHashMap::new(),
            _settings_param: AHashMap::new(),
            //_tags_id: AHashMap::new(),
            _tags_name: AHashMap::new(),
            _tags_parents: AHashMap::new(),
            _tags_namespace: AHashMap::new(),
            _file_max_id: 0,
            _jobs_max_id: 0,
            _namespace_max_id: 0,
            _parents_max_id: 0,
            _relationship_max_id: 0,
            _settings_max_id: 0,
            _tags_max_id: 0,
        }
    }

    ///
    /// Displays all stuff in the memory db.
    ///
    pub fn dbg_show_internals(&self) {
        for each in 0..self._file_max_id {
            println!(
                "FIL DB DBG: {} {} {} {}",
                each, self._file_hash[&each], self._file_filename[&each], self._file_size[&each]
            );
        }
        for e in 0..self._jobs_max_id {
            println!(
                "JOB DB DBG: {} {} {} {} {}",
                e,
                self._jobs_time[&e],
                self._jobs_rep[&e],
                self._jobs_site[&e],
                self._jobs_param[&e]
            );
        }
    }

    ///
    /// Increments All counters.
    ///
    fn max_increment(&mut self) {
        self._file_max_id += 1;
        self._namespace_max_id += 1;
        self._parents_max_id += 1;
        self._relationship_max_id += 1;
        self._settings_max_id += 1;
        self._tags_max_id += 1;
        self._jobs_max_id += 1;
    }

    ///
    /// Increments the file counter.
    ///
    fn max_file_increment(&mut self) {
        self._file_max_id += 1;
    }

    ///
    /// Increments the jobs counter.
    ///
    fn max_jobs_increment(&mut self) {
        self._jobs_max_id += 1;
    }
    ///
    /// Increments the jobs counter.
    ///
    fn max_settings_increment(&mut self) {
        self._settings_max_id += 1;
    }

    ///
    /// Adds Setting to memdb.
    ///
    fn settings_add(&mut self, name: String, pretty: String, num: u128, param: String) {
        self._settings_name.insert(name, self._settings_max_id);
        self._settings_pretty.insert(self._settings_max_id, pretty);
        self._settings_num.insert(self._settings_max_id, num);
        self._settings_param.insert(self._settings_max_id, param);
        self.max_settings_increment();
    }
    ///
    /// Gets Setting from memdb.
    /// Returns the num & param from memdb.
    ///
    fn settings_get_name(&self, name: &String) -> Result<(u128, String), &str> {
        if !self._settings_name.contains_key(name) {
            return Err("None");
        }
        let val = self._settings_name[name];
        return Ok((val, self._settings_param[&val].to_string()));
    }

    ///
    /// Adds job to memdb.
    ///
    fn jobs_add(
        &mut self,
        jobs_time: u128,
        jobs_rep: u128,
        jobs_site: String,
        jobs_param: String,
    ) -> u128 {
        self._jobs_time.insert(self._jobs_max_id, jobs_time);
        self._jobs_site.insert(self._jobs_max_id, jobs_site);
        self._jobs_rep.insert(self._jobs_max_id, jobs_rep);
        self._jobs_param.insert(self._jobs_max_id, jobs_param);
        self.max_jobs_increment();
        return self._jobs_max_id - 1;
    }

    ///
    /// Pulls jobs from memdb
    ///
    fn jobs_get(&self, id: u128) -> (String, String, String, String) {
        if self._jobs_time.contains_key(&id) {
            let a = self._jobs_time[&id].to_string();
            let d = self._jobs_rep[&id].to_string();
            let b = self._jobs_site[&id].to_string();
            let c = self._jobs_param[&id].to_string();

            return (a, b, d, c);
        } else {
            let error = "job_get cannot find job id in hashmap!";
            println!("{}, {}", error, id);
            error!("{}, {}", error, id);
            panic!("{}, {}", error, id)
        }
    }

    ///
    /// Checks if job exists in current memdb.
    ///
    pub fn jobs_exist(&self, id: u128) -> bool {
        self._jobs_time.contains_key(&id)
    }

    ///
    /// Returns total jobs in db.
    ///
    pub fn jobs_total(&self) -> u128 {
        self._jobs_max_id
    }

    /// Gets max_id from every max_id field.
    /// let (fid, nid, pid, rid, sid, tid, jid) = self.max_id_return();
    ///
    fn max_id_return(&mut self) -> (u128, u128, u128, u128, u128, u128, u128) {
        return (
            self._file_max_id,
            self._namespace_max_id,
            self._parents_max_id,
            self._relationship_max_id,
            self._settings_max_id,
            self._tags_max_id,
            self._jobs_max_id,
        );
    }

    ///
    /// Adds a file to memdb.Jobs
    ///
    pub fn file_put(&mut self, hash: String, filename: String, size: u64) {
        let (fid, _nid, _pid, _rid, _sid, _tid, _jid) = self.max_id_return();

        self._file_hash.insert(fid, hash);
        self._file_filename.insert(fid, filename);
        self._file_size.insert(fid, size);
        self.max_file_increment();
    }

    ///
    /// Gets a file from memdb.
    ///
    pub fn file_get(&mut self, id: u128) -> (&String, &String, &u64) {
        return (
            &self._file_hash[&id],
            &self._file_filename[&id],
            &self._file_size[&id],
        );
    }
}

/// Contains DB functions.
impl Main {
    /// Sets up new db instance.
    pub fn new(connection: Connection, path: String, vers: isize) -> Self {
        //let path = String::from("./main.db");

        Main {
            _dbpath: path,
            _conn: connection,
            _vers: vers,
            _inmemdb: Memdb::new(),
        }
    }
    ///
    /// Shows internals in db
    ///
    pub fn dbg_show_internals(&self) {
        self._inmemdb.dbg_show_internals();
    }
    ///
    /// Pulls db into memdb.
    ///
    pub fn load_mem(&mut self) {
        //TODO ADD SUPPORT FOR LOADING TAGS & FILES & RELATIONSHIPS.

        // Loads data from db into memory. CAN BE SLOW SHOULD OPTIMIZE WITH HASHMAP MAYBE??
        let mut fiex = self._conn.prepare("SELECT * FROM File").unwrap();
        let mut files = fiex.query(params![]).unwrap();
        let mut jobex = self._conn.prepare("SELECT * FROM Jobs").unwrap();
        let mut jobs = jobex.query(params![]).unwrap();
        let mut naex = self._conn.prepare("SELECT * FROM Namespace").unwrap();
        let mut names = naex.query(params![]).unwrap();
        let mut paex = self._conn.prepare("SELECT * FROM Parents").unwrap();
        let mut paes = paex.query(params![]).unwrap();
        let mut relx = self._conn.prepare("SELECT * FROM Relationship").unwrap();
        let mut rels = relx.query(params![]).unwrap();
        let mut taex = self._conn.prepare("SELECT * FROM Tags").unwrap();
        let mut tags = taex.query(params![]).unwrap();
        let mut setex = self._conn.prepare("SELECT * FROM Settings").unwrap();
        let mut sets = setex.query(params![]).unwrap();

        //Preserves mutability of database while we have an active connection to database.
        let mut file_vec: Vec<(u128, String, u128)> = Vec::new();
        let mut job_vec: Vec<(u128, u128, String, String)> = Vec::new();
        let mut parents_vec: Vec<(u128, String, String, u128)> = Vec::new();
        let mut namespace_vec: Vec<(u128, String, String)> = Vec::new();
        let mut relationship_vec: Vec<(u128, u128)> = Vec::new();
        let mut tag_vec: Vec<(u128, String, u128, u128)> = Vec::new();
        let mut setting_vec: Vec<(String, String, isize, String)> = Vec::new();

        while let Some(file) = files.next().unwrap() {
            let a: String = file.get(0).unwrap();
            let a1: u128 = a.parse::<u128>().unwrap();
            let b: String = file.get(2).unwrap();
            let b1: u128 = a.parse::<u128>().unwrap();
            file_vec.push((a1, file.get(1).unwrap(), b1));
        }

        while let Some(job) = jobs.next().unwrap() {
            let a1: String = job.get(0).unwrap();
            let b1: String = job.get(1).unwrap();
            let a: u128 = a1.parse::<u128>().unwrap();
            let b: u128 = b1.parse::<u128>().unwrap();
            job_vec.push((a, b, job.get(2).unwrap(), job.get(3).unwrap()));
            //self.jobs_add(a, b, job.get(2).unwrap(), job.get(3).unwrap(),true);
        }

        while let Some(name) = names.next().unwrap() {
            let a1: String = name.get(0).unwrap();
            let b1: String = name.get(2).unwrap();
            let a: u128 = a1.parse::<u128>().unwrap();
            let b: u128 = b1.parse::<u128>().unwrap();
            namespace_vec.push((a, name.get(1).unwrap(), b.to_string()));
        }

        while let Some(name) = paes.next().unwrap() {
            let a1: String = name.get(0).unwrap();
            let b1: String = name.get(3).unwrap();
            let a: u128 = a1.parse::<u128>().unwrap();
            let b: u128 = b1.parse::<u128>().unwrap();
            parents_vec.push((a, name.get(1).unwrap(), name.get(2).unwrap(), b));
        }

        while let Some(tag) = rels.next().unwrap() {
            let a1: String = tag.get(0).unwrap();
            let b1: String = tag.get(1).unwrap();
            let a: u128 = a1.parse::<u128>().unwrap();
            let b: u128 = b1.parse::<u128>().unwrap();
            relationship_vec.push((a, b));
        }

        while let Some(tag) = tags.next().unwrap() {
            let a1: String = tag.get(2).unwrap();
            let b1: String = tag.get(3).unwrap();
            let c1: String = tag.get(0).unwrap();
            let a: u128 = a1.parse::<u128>().unwrap();
            let b: u128 = b1.parse::<u128>().unwrap();
            let c: u128 = c1.parse::<u128>().unwrap();
            tag_vec.push((c, tag.get(1).unwrap(), a, b));
        }

        while let Some(set) = sets.next().unwrap() {

            let b1: isize = 0; //set.get(2).unwrap() FIXME
            let b: isize = b1.try_into().unwrap();
            let re1: String = match set.get(1) {
                Ok(re1) => re1,
                Err(error) => "".to_string(),
            };
            let re3: String = match set.get(3) {
                Ok(re3) => re3,
                Err(error) => "".to_string(),
            };

            setting_vec.push((set.get(0).unwrap(), re1, b, re3));
        }

        // Drops database connections.
        // Theirs probably a betterway to do this.
        // query makes things act weird...
        drop(files);
        drop(jobs);
        drop(names);
        drop(paes);
        drop(rels);
        drop(tags);
        drop(sets);
        drop(fiex);
        drop(jobex);
        drop(naex);
        drop(paex);
        drop(relx);
        drop(taex);
        drop(setex);

        // This adds the data gathered into memdb.
        for each in file_vec {
            self.file_add(each.0, each.1, each.2, false);
        }
        for each in job_vec {
            self.jobs_add(each.0, each.1, each.2, each.3, false);
        }
        for each in parents_vec {
            self.parents_add(each.0, each.1, each.2, each.3, false);
        }
        for each in namespace_vec {
            self.namespace_add(each.0, each.1, each.2, false);
        }
        for each in relationship_vec{
            self.relationship_add(each.0, each.1, false);}
        for each in tag_vec{
            self.tag_add(each.0, each.1, each.2, each.3, false);}
        for each in setting_vec{
        self.setting_add(each.0, each.1, each.2, each.3, false);}

    }

    ///
    /// Adds job to system.
    /// Will not add job to system if time is now.
    ///
    pub fn jobs_add_main(
        &mut self,
        jobs_time: String,
        jobs_rep: String,
        jobs_site: String,
        jobs_param: String,
        does_loop: bool,
    ) {
        let time_offset: u128 = time::time_conv(jobs_rep.to_string());

        /*self._inmemdb.jobs_add(
            jobs_time.parse::<u128>().unwrap(),
            time_offset,
            jobs_site.to_string(),
            jobs_param.to_string(),
        );*/
        let a1: String = jobs_time.to_string();
        let a: u128 = a1.parse::<u128>().unwrap();
        if &jobs_time != "now" && does_loop {

        let b1: String = jobs_rep.to_string();
        dbg!(&b1);

        let b: u128 = b1.parse::<u128>().unwrap();
            self.jobs_add(a, b, jobs_site, jobs_param, true);
        } else {
            self.jobs_add(a, 0, jobs_site, jobs_param, false);
        }



    }
    ///
    /// Pull job by id
    /// TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    ///
    pub fn jobs_get(&self, id: u128) -> (String, String, String, String) {
        if self._inmemdb.jobs_exist(id) {
            return self._inmemdb.jobs_get(id);
        } else {
            return (
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            );
        }
    }

    pub fn settings_get_name(&self, name: &String) -> Result<(u128, String), &str> {
        self._inmemdb.settings_get_name(&name)
    }

    ///
    /// Returns total jobs from _inmemdb
    ///
    pub fn jobs_get_max(&self) -> u128 {
        self._inmemdb.jobs_total()
    }

    /// Vacuums database. cleans everything.
    pub fn vacuum(&mut self) {
        info!("Starting Vacuum db!");
        self.execute("VACUUM".to_string());
        info!("Finishing Vacuum db!");
    }

    ///
    /// Handles the namespace data insertion into the DB
    /// ONLY ADDS NEW x IF NOT PRESENT IN NAMESPACE.
    /// TODO
    ///
    pub fn namespace_manager(&mut self, key: String) {
        let _name = self.pull_data(
            "Namespace".to_string(),
            "name".to_string(),
            urlparse::quote(key, b"").unwrap(),
        );

        if !_name.len() >= 1 {
            println!("NO NAMESPACE FOUND");
        }
    }

    ///
    /// Pulls data of table into form.
    /// Parses Data
    ///

    ///
    /// Pulls collums info
    ///  -> (Vec<String>, Vec<String>
    ///  SELECT sql FROM sqlite_master WHERE tbl_name='File' AND type = 'table';
    ///
    pub fn table_collumns(&mut self, table: String) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut t1: Vec<String> = Vec::new();
        let mut t2: Vec<String> = Vec::new();
        let parsedstring = format!(
            "SELECT sql FROM sqlite_master WHERE tbl_name='{}' AND type = 'table';",
            &table.as_str()
        );

        let mut toexec = self._conn.prepare(&parsedstring).unwrap();
        let mut outvec = Vec::new();

        let mut outs = toexec.query(params![]).unwrap();
        while let Some(out) = outs.next().unwrap() {
            let g1: String = out.get(0).unwrap();
            let g2: Vec<&str> = g1.split("(").collect();
            let g3: Vec<&str> = g2[1].split(")").collect();
            let g4: Vec<&str> = g3[0].split(", ").collect();

            for e in &g4 {
                let e1: Vec<&str> = e.split(" ").collect();
                t1.push(e1[0].to_string());
                t2.push(e1[1].to_string());
            }

            outvec.push(g1);
        }

        return (outvec, t1, t2);
    }

    ///
    /// Get table names
    /// Returns: Vec of strings
    ///
    pub fn table_names(&mut self, table: String) -> Vec<String> {
        let mut toexec = self
            ._conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table';")
            .unwrap();

        let mut outvec = Vec::new();

        let mut outs = toexec.query(params![]).unwrap();

        //println!("{:?}", out);

        while let Some(out) = outs.next().unwrap() {
            let vecpop = out.get(0).unwrap();
            outvec.push(vecpop);
        }

        //println!("{:?}", outvec);
        return outvec;
    }

    pub fn pull_data<'a>(
        &mut self,
        table: String,
        collumn: String,
        search_term: String,
    ) -> Vec<&'a str> {
        let name = "Tags".to_string();
        let mut list = Vec::new();
        list.push("a");
        list.push("b");

        //println!("PRAGMA table_info({});", &table);

        let a = self.table_names(table);

        for each in a {
            //println!("{}", each);
            self.table_collumns(each);
        }

        return list;
    }

    ///Sets up first database interaction.
    ///Makes tables and does first time setup.
    pub fn first_db(&mut self) {
        //Checking if file exists. If doesn't then no write perms.
        let dbexists = Path::new(&self.get_db_loc()).exists();

        if !dbexists {
            panic!("No database write perms or file not created");
        }

        // Making File Table
        let mut name = "File".to_string();
        let mut keys = vec_of_strings!["id", "hash", "filename", "size"];
        let mut vals = vec_of_strings!["TEXT", "TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Relationship Table
        name = "Relationship".to_string();
        keys = vec_of_strings!["fileid", "tagid"];
        vals = vec_of_strings!["TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Tags Table
        name = "Tags".to_string();
        keys = vec_of_strings!["id", "name", "parents", "namespace"];
        vals = vec_of_strings!["TEXT", "TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Parents Table
        name = "Parents".to_string();
        keys = vec_of_strings!["id", "name", "children", "namespace"];
        vals = vec_of_strings!["TEXT", "TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Namespace Table
        name = "Namespace".to_string();
        keys = vec_of_strings!["id", "name", "description"];

        vals = vec_of_strings!["TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Settings Table
        name = "Settings".to_string();
        keys = vec_of_strings!["name", "pretty", "num", "param"];
        vals = vec_of_strings!["TEXT", "TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Jobs Table
        name = "Jobs".to_string();
        keys = vec_of_strings!["time", "reptime", "site", "param"];
        vals = vec_of_strings!["TEXT", "TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        self.transaction_flush();
    }
    pub fn updatedb(&mut self) {
        self.setting_add(
            "VERSION".to_string(),
            "Version that the database is currently on.".to_string(),
            self._vers,
            "None".to_string(),
            true,
        );
        info!("Set VERSION to 1.");
        self.setting_add(
            "DEFAULTRATELIMIT".to_string(),
            "None".to_string(),
            5,
            "None".to_string(),
            true,
        );
        self.setting_add(
            "FilesLoc".to_string(),
            "None".to_string(),
            0,
            "./Files/".to_string(),
            true,
        );
        self.setting_add(
            "DEFAULTUSERAGENT".to_string(),
            "None".to_string(),
            0,
            "DIYHydrus/5.0 (Windows NT x.y; rv:10.0) Gecko/20100101 DIYHydrus/10.0".to_string(),
            true,
        );
        self.setting_add(
            "pluginloadloc".to_string(),
            "Where plugins get loaded into.".to_string(),
            0,
            "./plugins".to_string(),
            true,
        );
        self.transaction_flush();
    }

    ///
    /// Creates a table
    /// name: The table name
    /// key: List of Collumn lables.
    /// dtype: List of Collumn types. NOTE Passed into SQLITE DIRECTLY THIS IS BAD :C
    ///
    pub fn table_create(&mut self, name: &String, key: &Vec<String>, dtype: &Vec<String>) {
        //Sanity checking...
        assert_eq!(
            key.len(),
            dtype.len(),
            "Warning table create was 2 Vecs weren't balanced. Lengths: {} {}",
            key.len(),
            dtype.len()
        );

        //Not sure if theirs a better way to dynamically allocate a string based on two vec strings at run time.
        //Let me know if im doing something stupid.
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
            ")".to_string(),
        ]
        .concat();

        info!("Creating table as: {}", endresult);
        stocat = endresult;

        self.execute(stocat);
    }

    ///u128
    /// Checks if db version is consistent.
    ///

    pub fn check_version(&mut self) {
        let g1 = self
            .quer_int("SELECT num FROM Settings WHERE name='VERSION';".to_string())
            .unwrap();

        if self._vers != g1[0] {
            println!("DB UPDATE NOT IMPLEMENTED YEET.");
        } else {
            info!("Database Version is: {}", g1[0]);
        }
    }

    ///
    /// Sets advanced settings for journaling.
    /// NOTE Experimental badness
    ///
    pub fn db_open(&mut self) {
        //self.execute("PRAGMA journal_mode = MEMORY".to_string());
        self.execute("PRAGMA synchronous = OFF".to_string());
        info!("Setting synchronous = OFF");
        //println!("db_open");
    }

    ///
    /// Adds a file into the db.
    /// Do this first.
    ///
    pub fn file_add(&mut self, id: u128, hash: String, size: u128, addtodb: bool) {}

    pub fn namespace_add(&mut self, id: u128, name: String, description: String, addtodb: bool) {}

    pub fn parents_add(
        &mut self,
        id: u128,
        name: String,
        children: String,
        namespace: u128,
        addtodb: bool,
    ) {
    }

    pub fn tag_add(
        &mut self,
        id: u128,
        name: String,
        parents: u128,
        namespace: u128,
        addtodb: bool,
    ) {
    }

    pub fn relationship_add(&mut self, file: u128, tag: u128, addtodb: bool) {}

    pub fn jobs_add(
        &mut self,
        time: u128,
        reptime: u128,
        site: String,
        param: String,
        addtodb: bool,
    ) {

    if addtodb {
        let inp = "INSERT INTO Jobs VALUES(?, ?, ?, ?)";
            let _out = self._conn.execute(
                &inp,
                params![
                    &time.to_string(),
                    &reptime.to_string(),
                    &site.to_string(),
                    &param.to_string()
                ],
            );
    }
    self._inmemdb.jobs_add(
            time,
            reptime,
            site,
            param,
        );

    }

    ///
    /// Adds a setting to the Settings Table.
    /// name: str   , Setting name
    /// pretty: str , Fancy Flavor text optional
    /// num: u64    , unsigned u64 largest int is 18446744073709551615 smallest is 0
    /// param: str  , Parameter to allow (value)
    ///
    pub fn setting_add(
        &mut self,
        name: String,
        pretty: String,
        num: isize,
        param: String,
        addtodb: bool,
    ) {
        let temp: isize = -9999;

        if addtodb {
            let _ex = self._conn.execute(
                "INSERT INTO Settings(name, pretty, num, param) VALUES (?1, ?2, ?3, ?4)",
                params![
                    &name,
                    //Hella jank workaround. can only pass 1 type into a function without doing workaround.
                    //This makes it work should be fine for one offs.
                    if &pretty == "None" {
                        &Null as &dyn ToSql
                    } else {
                        &pretty
                    },
                    if &num == &temp {
                        &Null as &dyn ToSql
                    } else {
                        &num
                    },
                    if &param == "None" {
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
                Ok(_ex) => (),
            }
        }
        // Adds setting into memdbb
        if num >= 0 {
            self._inmemdb
                .settings_add(name, pretty, num.try_into().unwrap(), param);
        } else {
            self._inmemdb.settings_add(name, pretty, 0, param);
        }
    }

    /// Starts a transaction for bulk inserts.
    pub fn transaction_start(&mut self) {
        self.execute("BEGIN".to_string());
    }

    /// Flushes to disk.
    pub fn transaction_flush(&mut self) {
        self.execute("COMMIT".to_string());
        self.execute("BEGIN".to_string());
    }

    // Closes a transaction for bulk inserts.
    pub fn transaction_close(&mut self) {
        self.execute("COMMIT".to_string());
    }

    /// Returns db location as String refernce.
    pub fn get_db_loc(&self) -> String {
        return self._dbpath.to_string();
    }

    ///
    /// Querys the db use this for select statements.
    /// NOTE USE THIS ONY FOR RESULTS THAT RETURN STRINGS
    ///
    pub fn quer_str(&mut self, inp: String) -> Result<Vec<String>> {
        let mut toexec = self._conn.prepare(&inp).unwrap();
        let rows = toexec.query_map([], |row| row.get(0)).unwrap();
        let mut out = Vec::new();

        for each in rows {
            out.push(each.unwrap());
        }
        return Ok(out);
    }

    ///
    /// Querys the db use this for select statements.
    /// NOTE USE THIS ONY FOR RESULTS THAT RETURN INTS
    ///
    pub fn quer_int(&mut self, inp: String) -> Result<Vec<isize>> {

        dbg!(&inp);

        let mut toexec = self._conn.prepare(&inp).unwrap();
        let rows = toexec.query_map([], |row| row.get(0)).unwrap();
        let mut out: Vec<isize> = Vec::new();

        for each in rows {
            let temp: String = each.unwrap();
            out.push(temp.parse::<isize>().unwrap());
        }
        return Ok(out);
    }

    /// Raw Call to database. Try to only use internally to this file only.
    /// Doesn't support params nativly.
    /// Will not write changes to DB. Have to call write().
    /// Panics to help issues.
    pub fn execute(&mut self, inp: String) -> usize {
        let _out = self._conn.execute(&inp, params![]);

        match _out {
            Err(_out) => {
                println!("SQLITE STRING:: {}", inp);
                println!("BAD CALL {}", _out);
                error!("BAD CALL {}", _out);
                panic!("BAD CALL {}", _out);
            }
            Ok(_out) => (_out),
        }
    }

    /// Handles transactional pushes.
    pub fn transaction_execute(trans: Transaction, inp: String) {
        trans.execute(&inp, params![]).unwrap();
    }
}
