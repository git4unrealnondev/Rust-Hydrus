#![forbid(unsafe_code)]
use crate::scr::jobs;
use crate::scr::scraper;
use crate::scr::sharedtypes::CommitType;
use crate::scr::time;
use crate::vec_of_strings;
use ahash::AHashMap;
use log::{error, info};
//use nohash_hasher::NoHashHasher;
use super::jobs::JobsRef;
use super::sharedtypes;
use nohash_hasher::BuildNoHashHasher;
use nohash_hasher::IntMap;
use nohash_hasher::NoHashHasher;
pub use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
pub use rusqlite::{params, types::Null, Connection, Result, Transaction};
use smallest_uint::SmallestUIntFor;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::panic;
use std::path::Path;
use std::{collections::HashMap, hash::BuildHasherDefault};

pub enum tag_relate_conjoin {
    Tag,
    Error,
    Relate,
    Conjoin,
    Tag_and_Relate,
    None,
}

/// Returns an open connection to use.
pub fn dbinit(dbpath: &String) -> Connection {
    //Engaging Transaction Handling
    Connection::open(&dbpath).unwrap()
}

///
/// Pulls db into memdb.
/// main: &mut Main, conn: &Connection
///
pub fn load_mem(tempmem: &mut Main, conn: &Connection) {
    //let mut brr =  tempmem._conn.borrow();

    //drop(brr);
    //let conn = dbinit(&self._dbpath);
    //let mutborrow = &*self._conn.borrow_mut();
    // Loads data from db into memory. CAN BE SLOW SHOULD OPTIMIZE WITH HASHMAP MAYBE??
    let mut fiex = conn.prepare("SELECT * FROM File").unwrap();
    let mut files = fiex.query(params![]).unwrap();

    //let mut files = self._conn.prepare("SELECT * FROM File").unwrap().query(params![]).unwrap();
    let mut jobex = conn.prepare("SELECT * FROM Jobs").unwrap();
    let mut jobs = jobex.query(params![]).unwrap();
    let mut naex = conn.prepare("SELECT * FROM Namespace").unwrap();
    let mut names = naex.query(params![]).unwrap();
    let mut paex = conn.prepare("SELECT * FROM Parents").unwrap();
    let mut paes = paex.query(params![]).unwrap();
    let mut relx = conn.prepare("SELECT * FROM Relationship").unwrap();
    let mut rels = relx.query(params![]).unwrap();
    let mut taex = conn.prepare("SELECT * FROM Tags").unwrap();
    let mut tags = taex.query(params![]).unwrap();
    let mut setex = conn.prepare("SELECT * FROM Settings").unwrap();
    let mut sets = setex.query(params![]).unwrap();

    //drop(fiex);
    //Preserves mutability of database while we have an active connection to database.
    let mut file_vec: Vec<(usize, String, String, String)> = Vec::new();
    let mut job_vec: Vec<(usize, usize, String, String, CommitType)> = Vec::new();
    let mut parents_vec: Vec<(usize, String, String, usize)> = Vec::new();
    let mut namespace_vec: Vec<(usize, String, String)> = Vec::new();
    let mut relationship_vec: Vec<(usize, usize)> = Vec::new();
    let mut tag_vec: Vec<(usize, String, String, usize)> = Vec::new();
    let mut setting_vec: Vec<(String, String, isize, String)> = Vec::new();

    dbg!("Loading DB.");
    //drop(fiex);
    while let Some(file) = files.next().unwrap() {
        let a: String = file.get(0).unwrap();
        let a1: usize = a.parse::<usize>().unwrap();
        tempmem.file_add(
            file.get(1).unwrap(),
            file.get(2).unwrap(),
            file.get(3).unwrap(),
            false,
        );
    }
    while let Some(job) = jobs.next().unwrap() {
        let a1: String = job.get(0).unwrap();
        let b1: String = job.get(1).unwrap();
        let d1: String = job.get(2).unwrap();
        let e1: String = job.get(3).unwrap();
        let c1: String = job.get(4).unwrap();
        let c: CommitType = sharedtypes::stringto_commit_type(&c1);
        let a: usize = a1.parse::<usize>().unwrap();
        let b: usize = b1.parse::<usize>().unwrap();
        tempmem.jobs_add_new_todb(&d1, &e1, b, a, &c);
    }

    while let Some(name) = names.next().unwrap() {
        let a1: String = name.get(0).unwrap();
        let b1: String = name.get(2).unwrap();
        let a: usize = a1.parse::<usize>().unwrap();
        let b: String = b1.parse::<String>().unwrap();
        //namespace_vec.push((a, name.get(1).unwrap(), b.to_string()));
        tempmem.namespace_add(&name.get(1).unwrap(), &b.to_string(), false);
    }

    while let Some(tag) = rels.next().unwrap() {
        let a1: String = tag.get(0).unwrap();
        let b1: String = tag.get(1).unwrap();
        let a: usize = a1.parse::<usize>().unwrap();
        let b: usize = b1.parse::<usize>().unwrap();
        //relationship_vec.push((a, b));
        tempmem.relationship_add(a, b, false);
    }

    while let Some(tag) = tags.next().unwrap() {
        let a1: String = tag.get(2).unwrap();
        let b1: String = tag.get(3).unwrap();
        let c1: String = tag.get(0).unwrap();
        let a: String = a1.parse::<String>().unwrap();
        let b: usize = b1.parse::<usize>().unwrap();
        let c: usize = c1.parse::<usize>().unwrap();
        //tag_vec.push((c, tag.get(1).unwrap(), a, b));
        tempmem.tag_add(tag.get(1).unwrap(), a, b, false);
        //self.tag_add(each.1, each.2, each.3, false);
    }

    while let Some(tag) = paes.next().unwrap() {
        let a1: String = tag.get(0).unwrap();
        let a2: String = tag.get(1).unwrap();
        let a3: String = tag.get(2).unwrap();
        let a4: String = tag.get(3).unwrap();

        let b1 = a1.parse::<usize>().unwrap();
        let b2 = a2.parse::<usize>().unwrap();
        let b3 = a3.parse::<usize>().unwrap();
        let b4 = a4.parse::<usize>().unwrap();
        tempmem.parents_add(b1, b2, b3, b4, false);
    }

    while let Some(name) = paes.next().unwrap() {
        let a1: String = name.get(0).unwrap();
        let b1: String = name.get(3).unwrap();
        let a: usize = a1.parse::<usize>().unwrap();
        let b: usize = b1.parse::<usize>().unwrap();
        parents_vec.push((a, name.get(1).unwrap(), name.get(2).unwrap(), b));
    }

    while let Some(set) = sets.next().unwrap() {
        let b1: String = set.get(2).unwrap(); // FIXME
        let b: usize = b1.parse::<usize>().unwrap();
        let re1: String = match set.get(1) {
            Ok(re1) => re1,
            Err(error) => "".to_string(),
        };
        let re3: String = match set.get(3) {
            Ok(re3) => re3,
            Err(error) => "".to_string(),
        };

        setting_vec.push((set.get(0).unwrap(), re1, b.try_into().unwrap(), re3));
    }

    for each in setting_vec {
        tempmem.setting_add(each.0, each.1, each.2, each.3, false);
    }

    //self._conn = conn;
}

/// Holder of database self variables
pub struct Main {
    _dbpath: String,
    pub _conn: RefCell<Connection>,
    _vers: isize,
    // inmem db with ahash low lookup/insert time. Alernative to hashmap
    _inmemdb: Memdb,
    _dbcommitnum: usize,
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

    _file_max_id: usize,
    _file_hash: AHashMap<String, usize>,
    _file: AHashMap<(String, String, String), usize>,
    _file_url_to_id: AHashMap<String, usize>,

    _jobs_max_id: usize,
    _jobs_ref: IntMap<usize, JobsRef>,
    _jobs_time: IntMap<usize, usize>,
    _jobs_rep: IntMap<usize, usize>,
    _jobs_site: IntMap<usize, String>,
    _jobs_param: IntMap<usize, String>,
    _jobs_commitunfinished: IntMap<usize, bool>,

    _namespace_max_id: usize,
    _namespace_name: AHashMap<String, usize>,
    _namespace_description: AHashMap<usize, String>,

    //_parents_relate: IntMap<(usize, usize, usize, usize), usize>,
    _parents_tag: AHashMap<(usize, usize), usize>,
    _parents_tag_max_id: usize,
    _parents_relate: AHashMap<(usize, usize), usize>,
    _parents_relate_max_id: usize,
    _parents_conjoin: AHashMap<(usize, usize), usize>,
    _parents_max_id: usize,

    _relationship_max_id: usize,
    _relationship_fileid: IntMap<usize, usize>,
    _relationship_tagid: IntMap<usize, usize>,
    _relationship_relate: AHashMap<(usize, usize), usize>,

    _settings_max_id: usize,
    _settings_name: AHashMap<String, usize>,
    _settings_pretty: AHashMap<usize, String>,
    _settings_num: AHashMap<usize, usize>,
    _settings_param: AHashMap<usize, String>,

    _tags_max_id: usize,
    _tags_name: AHashMap<String, usize>,
    _tags_parents: IntMap<usize, usize>,
    _tags_namespace: IntMap<usize, usize>,
    _tags_relate: AHashMap<(String, usize), usize>,
}

/// Functions for working with memorory db.
/// Uses AHash for maximum speed.
#[allow(dead_code)]

impl Memdb {
    pub fn new() -> Self {
        Memdb {
            _table_names: ("File", "Jobs", "Namespace", "Parents", "Settings", "Tags"),
            //_table_names: ,
            //_file_id:AHashMap::new(),
            _file_hash: AHashMap::new(),
            _file: AHashMap::new(),
            _file_url_to_id: AHashMap::new(),
            _jobs_time: HashMap::with_hasher(BuildNoHashHasher::default()),
            _jobs_rep: HashMap::with_hasher(BuildNoHashHasher::default()),
            _jobs_ref: HashMap::with_hasher(BuildNoHashHasher::default()),
            _jobs_site: HashMap::with_hasher(BuildNoHashHasher::default()),
            _jobs_param: HashMap::with_hasher(BuildNoHashHasher::default()),
            _jobs_commitunfinished: HashMap::with_hasher(BuildNoHashHasher::default()),
            //_namespace_id:AHashMap::new(),
            _namespace_name: AHashMap::new(),
            _namespace_description: AHashMap::new(),
            //_parents_id:AHashMap::new(),
            _parents_relate: AHashMap::new(),
            _parents_tag: AHashMap::new(),
            _parents_conjoin: AHashMap::new(),

            _relationship_fileid: HashMap::with_hasher(BuildNoHashHasher::default()),
            _relationship_tagid: HashMap::with_hasher(BuildNoHashHasher::default()),
            _relationship_relate: AHashMap::new(),
            _settings_name: AHashMap::new(),
            _settings_pretty: AHashMap::new(),
            _settings_num: AHashMap::new(),
            _settings_param: AHashMap::new(),
            //_tags_id:AHashMap::new(),
            _tags_name: AHashMap::new(),
            _tags_parents: HashMap::with_hasher(BuildNoHashHasher::default()),
            _tags_namespace: HashMap::with_hasher(BuildNoHashHasher::default()),
            _tags_relate: AHashMap::new(),
            _file_max_id: 0,
            _jobs_max_id: 0,
            _namespace_max_id: 0,
            _parents_max_id: 0,
            _parents_tag_max_id: 0,
            _parents_relate_max_id: 0,
            _relationship_max_id: 0,
            _settings_max_id: 0,
            _tags_max_id: 0,
        }
    }

    ///
    /// Displays all stuff in the memory db.
    ///
    pub fn dbg_show_internals(&self) {
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
    /// Increments the file counter.
    ///
    fn max_tags_increment(&mut self) {
        self._tags_max_id += 1;
    }

    ///
    /// Increments the file counter.
    ///
    fn max_namespace_increment(&mut self) {
        self._namespace_max_id += 1;
    }

    ///
    /// Increments the jobs counter.
    ///
    fn max_jobs_increment(&mut self) {
        self._jobs_max_id += 1;
    }
    ///
    /// Increments the settings counter.
    ///
    fn max_settings_increment(&mut self) {
        self._settings_max_id += 1;
    }
    ///
    /// Increments the relationship counter.
    ///
    fn max_relationship_increment(&mut self) {
        self._relationship_max_id += 1;
    }

    ///
    /// Adds Setting to memdb.
    ///
    fn settings_add(&mut self, name: String, pretty: String, num: usize, param: String) {
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
    fn settings_get_name(&self, name: &String) -> Result<(usize, String), &str> {
        if !self._settings_name.contains_key(name) {
            return Err("None");
        }
        let val = self._settings_name[name];
        Ok((
            self._settings_num[&val],
            self._settings_param[&val].to_string(),
        ))
    }

    fn jobs_add_new(&mut self, job: jobs::JobsRef) {
        self._jobs_ref.insert(job._idindb, job);
        self.max_jobs_increment();
    }

    ///
    /// Adds job to memdb.
    ///
    fn jobs_add(
        &mut self,
        jobs_time: usize,
        jobs_rep: usize,
        jobs_site: String,
        jobs_param: String,
        commit: bool,
    ) -> usize {
        self._jobs_time.insert(self._jobs_max_id, jobs_time);
        self._jobs_site.insert(self._jobs_max_id, jobs_site);
        self._jobs_rep.insert(self._jobs_max_id, jobs_rep);
        self._jobs_param.insert(self._jobs_max_id, jobs_param);
        self._jobs_commitunfinished
            .insert(self._jobs_max_id, commit);
        self.max_jobs_increment();
        self._jobs_max_id - 1
    }

    ///
    /// Made job ref and adds job into db.
    ///
    fn jobref_new(
        &mut self,
        sites: String,
        params: Vec<String>,
        jobstime: usize,
        jobsref: usize,
        committype: CommitType,
    ) {
        let job = JobsRef {
            _idindb: self._jobs_max_id,
            _sites: sites,
            _params: params,
            _jobsref: jobsref,
            _jobstime: jobstime,
            _committype: committype,
        };
        self.jobs_add_new(job);
    }

    ///
    /// Checks if a parent exists.
    ///
    fn parents_get(
        &self,
        tag_namespace_id: usize,
        tag_id: usize,
        relate_namespace_id: usize,
        relate_tag_id: usize,
    ) -> (tag_relate_conjoin, &usize) {
        // Checks if keys exists in each hashmap.
        // Twohashmaps and one relational hashmap. Should be decently good.
        let tag_usize = self._parents_tag.get(&(tag_namespace_id, tag_id));
        let relate_usize = self
            ._parents_relate
            .get(&(relate_namespace_id, relate_tag_id));

        match tag_usize {
            None => match relate_usize {
                None => (tag_relate_conjoin::Tag_and_Relate, &0),
                Some(_) => (tag_relate_conjoin::Tag, &0),
            },
            Some(_) => match relate_usize {
                None => (tag_relate_conjoin::Relate, tag_usize.unwrap()),
                Some(_) => {
                    let tag_conjoin = self
                        ._parents_conjoin
                        .get(&(*tag_usize.unwrap(), *relate_usize.unwrap()));

                    match tag_conjoin {
                        None => (tag_relate_conjoin::Conjoin, &0),
                        Some(_) => (tag_relate_conjoin::None, tag_conjoin.unwrap()),
                    }
                }
            },
        }
        //self._parents_relate.contains_key(&(tag_namespace_id, tag_id, relate_namespace_id, relate_tag_id));
    }

    ///
    /// Increase parents tag increase
    ///
    fn parents_tag_increment(&mut self) {
        self._parents_tag_max_id += 1;
    }

    ///
    /// Increases relates tag increase
    ///
    fn parents_relate_increment(&mut self) {
        self._parents_relate_max_id += 1;
    }

    ///
    /// parents conjoin relates tag increase
    ///
    fn parents_conjoin_increment(&mut self) {
        self._parents_max_id += 1;
    }

    ///
    /// Creates a parent inside memdb.
    ///
    fn parents_put(
        &mut self,
        tag_namespace_id: usize,
        tag_id: usize,
        relate_namespace_id: usize,
        relate_tag_id: usize,
    ) -> tag_relate_conjoin {
        let (tag_enum, fin_uint) =
            self.parents_get(tag_namespace_id, tag_id, relate_namespace_id, relate_tag_id);

        match tag_enum {
            tag_relate_conjoin::Error => {
                // Error happened. :D
                error!("WARNING: PARENTS_GET GOT ERROR FOR UNKNOWN POSSIBLE DB CORRUPTION: {} {} : {} {} :", tag_namespace_id, tag_id, relate_namespace_id, relate_tag_id);
                error!("PANICING DUE TO PARENTS_GET FAIL");
                panic!("Check Log for details");
            }
            tag_relate_conjoin::Tag => {
                // Missing tag and namespace
                let namespace = tag_namespace_id;
                let fint = *fin_uint;
                self._parents_tag
                    .insert((namespace, tag_id), self._parents_tag_max_id);
                self._parents_conjoin
                    .insert((self._parents_tag_max_id, fint), self._parents_max_id);
                self.parents_tag_increment();
                self.parents_conjoin_increment();
                tag_relate_conjoin::Tag
            }
            tag_relate_conjoin::Relate => {
                // Missing tag_relate and namespace_relate
                let fint = *fin_uint;
                self._parents_relate.insert(
                    (relate_namespace_id, relate_tag_id),
                    self._parents_relate_max_id,
                );
                self._parents_conjoin
                    .insert((fint, self._parents_relate_max_id), self._parents_max_id);
                self.parents_relate_increment();
                self.parents_conjoin_increment();
                tag_relate_conjoin::Relate
            }
            tag_relate_conjoin::Conjoin => {
                // Missing Conjoin linkage between two hashmaps
                let tid = self._parents_tag.get(&(tag_namespace_id, tag_id)).unwrap();
                let rid = self
                    ._parents_relate
                    .get(&(relate_namespace_id, relate_tag_id))
                    .unwrap();
                self._parents_conjoin
                    .insert((*tid, *rid), self._parents_max_id);
                self.parents_conjoin_increment();
                tag_relate_conjoin::Conjoin
            }
            tag_relate_conjoin::Tag_and_Relate => {
                // Missing tag&namespace and relate&namespace
                self._parents_tag
                    .insert((tag_namespace_id, tag_id), self._parents_tag_max_id);
                self._parents_relate.insert(
                    (relate_namespace_id, relate_tag_id),
                    self._parents_relate_max_id,
                );
                self._parents_conjoin.insert(
                    (self._parents_tag_max_id, self._parents_relate_max_id),
                    self._parents_max_id,
                );
                self.parents_tag_increment();
                self.parents_relate_increment();
                self.parents_conjoin_increment();
                tag_relate_conjoin::Tag_and_Relate
            }
            tag_relate_conjoin::None => {
                // Missing Nothing
                tag_relate_conjoin::None
            }
        }
    }

    /*pub enum tag_relate_conjoin {
        Tag,
        Error,
        Relate,
        Conjoin,
        Tag_and_Relate,
        Tag_and_Relatead_Conjoin,
        None,
    }*/

    ///
    /// Checks if relationship exists in db.
    ///
    fn relationship_get(&self, file: usize, tag: usize) -> bool {
        if self._relationship_relate.contains_key(&(file, tag)) {
            return true;
        }

        false
    }

    ///
    /// Returns a list of fileid's associated with tagid
    ///
    fn relationship_get_fileid(&self, tag: &usize) -> Vec<usize> {
        let mut comp: Vec<usize> = Vec::new();
        for each in self._relationship_relate.keys() {
            if &each.1 == tag {
                comp.push(each.0);
            }
        }
        comp
    }

    ///
    /// relationship gets only one fileid
    ///
    fn relationship_get_one_fileid(&self, tag: &usize) -> Option<usize> {
        for each in self._relationship_relate.keys() {
            if &each.1 == tag {
                return Some(each.0);
            }
        }
        None
    }

    ///
    /// Returns a list of tag's associated with fileid
    ///
    fn relationship_get_tagid(&self, file: &usize) -> Vec<usize> {
        let mut comp: Vec<usize> = Vec::new();
        for each in self._relationship_relate.keys() {
            if &each.0 == file {
                comp.push(each.1);
            }
        }
        comp
    }

    ///
    /// Adds relationship to db.
    ///
    fn relationship_add(&mut self, file: usize, tag: usize) {
        //self._relationship_fileid.insert(file, file);
        //self._relationship_tagid.insert(tag, tag);
        self._relationship_relate
            .insert((file, tag), self._relationship_max_id);
    }

    ///
    /// returns a immutable reference to the database's job table
    ///
    fn jobs_get_all(&self) -> &HashMap<usize, JobsRef, BuildHasherDefault<NoHashHasher<usize>>> {
        &self._jobs_ref
    }

    ///
    /// Returns job by refid inmemdb
    ///
    fn jobs_get_new(&self, id: &usize) -> &JobsRef {
        &self._jobs_ref[id]
    }

    ///
    /// Pulls jobs from memdb
    ///
    fn jobs_get(&self, id: usize) -> (String, String, String, String, bool) {
        if self._jobs_time.contains_key(&id) {
            let a = self._jobs_time[&id].to_string();
            let d = self._jobs_rep[&id].to_string();
            let b = self._jobs_site[&id].to_string();
            let c = self._jobs_param[&id].to_string();
            let e = self._jobs_commitunfinished[&id];

            (a, b, d, c, e)
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
    pub fn jobs_exist(&self, id: usize) -> bool {
        self._jobs_time.contains_key(&id)
    }

    ///
    /// Returns total jobs in db.
    ///
    pub fn jobs_total(&self) -> usize {
        self._jobs_max_id
    }

    /// Gets max_id from every max_id field.
    /// let (fid, nid, pid, rid, sid, tid, jid) = self.max_id_return();
    ///
    fn max_id_return(&mut self) -> (usize, usize, usize, usize, usize, usize, usize) {
        (
            self._file_max_id,
            self._namespace_max_id,
            self._parents_max_id,
            self._relationship_max_id,
            self._settings_max_id,
            self._tags_max_id,
            self._jobs_max_id,
        )
    }

    ///
    /// Adds a file to memdb.Jobs
    ///
    pub fn file_put(&mut self, hash: &String, extension: &String, location: &String) -> usize {
        if self._file_hash.contains_key(&hash.to_string()) {
            return self._file_hash[&hash.to_string()];
        }
        let ret_name: usize = self._file_max_id;
        let (fid, _nid, _pid, _rid, _sid, _tid, _jid) = self.max_id_return();

        self._file.insert(
            (
                hash.to_string(),
                extension.to_string(),
                location.to_string(),
            ),
            fid,
        );

        self._file_hash.insert(hash.to_string(), fid);
        self.max_file_increment();

        ret_name
    }

    ///
    /// Gets a file from memdb via hash.
    ///
    pub fn file_get_hash(&self, hash: &String) -> (usize, bool) {
        if self._file_hash.contains_key(hash) {
            (self._file_hash[hash], true)
        } else {
            (0, false)
        }
    }

    ///
    /// Returns a files info based on id
    ///
    pub fn file_get_id(&self, id: &usize) -> Option<(String, String, String)> {
        let hash = self
            ._file_hash
            .iter()
            .find_map(|(key, &val)| if &val == id { Some(key) } else { None });

        let structsearch = self
            ._file
            .iter()
            .find_map(|(key, &val)| if &val == id { Some(key) } else { None });

        if hash.is_some() && structsearch.is_some() {
            Some((
                hash.unwrap().to_string(),
                structsearch.unwrap().1.to_string(),
                structsearch.unwrap().2.to_string(),
            ))
        } else {
            None
        }
    }

    ///
    /// Does namespace exist? If so return number.
    ///
    pub fn namespace_put(&mut self, name: &String) -> usize {
        if self._namespace_name.contains_key(name) {
            return self._namespace_name[name];
        }
        let ret_name: usize = self._namespace_max_id;
        self._namespace_name.insert(name.to_string(), ret_name);
        self.max_namespace_increment();
        ret_name
    }

    ///
    /// Does namespace contain key?
    ///
    pub fn namespace_get(&self, name: &String) -> (usize, bool) {
        if self._namespace_name.contains_key(name) {
            (self._namespace_name[name], true)
        } else {
            (0, false)
        }
    }

    ///
    /// Namespace get name from id
    ///
    pub fn namespace_id_get(&self, uid: &usize) -> String {
        for (key, val) in self._namespace_name.iter() {
            if val == uid {
                return key.to_string();
            }
        }
        "".to_string()
    }

    ///
    /// Adds a file to memdb.Tags
    ///
    pub fn tags_name_put_test(&mut self, tag: String, namespace: &String) -> usize {
        if self._tags_name.contains_key(&tag) {
            return self._tags_name[&tag];
        }
        let ret_name: usize = self._tags_max_id;

        self._tags_name.insert(tag, ret_name);

        self.namespace_put(namespace);

        let ret_tag: usize = self._tags_max_id;
        //self._tags_relate.insert((tag.to_string(), *namespace), ret_name);

        self.max_tags_increment();

        ret_tag
    }

    ///
    /// Adds a file to memdb.Tags
    ///
    pub fn tags_put(&mut self, tag: &String, namespace: &usize) -> usize {
        //if self._tags_name.contains_key(tag) {
        //    return self._tags_name[tag];
        //}
        if self
            ._tags_relate
            .contains_key(&(tag.to_string(), *namespace))
        {
            //let tagid = self._tags_name[&(tags.to_string(), namespace)];

            let urin: usize = self._tags_relate[&(tag.to_string(), *namespace)];
            return urin;
        }
        //println!("{} {}", tag, namespace);
        let ret_name: usize = self._tags_max_id;

        self._tags_name.insert(tag.to_string(), ret_name);
        self._tags_namespace.insert(*namespace, 0);

        self._tags_relate
            .insert((tag.to_string(), *namespace), ret_name);

        self.max_tags_increment();

        ret_name
    }

    ///
    /// Does tags contain key?
    ///
    pub fn tags_get(&self, tags: String, namespace: usize) -> (usize, bool) {
        if self
            ._tags_relate
            .contains_key(&(tags.to_string(), namespace))
        {
            //let tagid = self._tags_name[&(tags.to_string(), namespace)];

            let urin: usize = self._tags_relate[&(tags, namespace)];

            (urin, true)
        } else {
            (namespace, false)
        }
    }

    pub fn dbg(&mut self) {
        for each in &self._tags_relate {
            println!("{:?}", each);
        }
    }

    ///
    /// Gets tag name by id
    ///
    pub fn tag_id_get(&mut self, uid: &usize) -> (String, String) {
        for (key, val) in self._tags_relate.iter() {
            if val == uid {
                return (key.0.to_string(), self.namespace_id_get(&key.1));
            }
        }
        ("".to_string(), "".to_string())
    }
}

/// Contains DB functions.
impl Main {
    /// Sets up new db instance.
    pub fn new(path: String, vers: isize) -> Self {
        // Initiates two connections to the DB.
        // Cheap workaround to avoid loading errors.
        let dbexist = Path::new(&path).exists();
        let connection = dbinit(&path);
        let connection2 = dbinit(&path);
        //let conn = connection;
        let memdb = Memdb::new();
        //let path = String::from("./main.db");

        let mut memdbmain = Main {
            _dbpath: path.to_owned(),
            _conn: RefCell::new(Connection::open_in_memory().unwrap()),
            _vers: vers,
            _inmemdb: memdb,
            _dbcommitnum: 0,
        };

        let mut main = Main {
            _dbpath: path,
            _conn: RefCell::new(Connection::open_in_memory().unwrap()),
            _vers: vers,
            _inmemdb: memdbmain._inmemdb,
            _dbcommitnum: 0,
        };
        main._conn = RefCell::new(connection);

        main.db_open(); // Sets default settings for db settings.

        if !dbexist {
            // Database Doesn't exist
            main.first_db();
            main.updatedb();
            main.db_commit_man_set();
        } else {
            // Database does exist.
            main.transaction_start();
            println!("Database Exists: {} : Skipping creation.", dbexist);
            info!("Database Exists: {} : Skipping creation.", dbexist);
        }
        load_mem(&mut main, &connection2); // loads db into memory from alternate db connection.

        main
    }

    ///
    /// Run this code after creating
    ///
    pub fn after_creation(&mut self) {}

    ///
    /// Shows internals in db
    ///
    pub fn dbg_show_internals(&self) {
        self._inmemdb.dbg_show_internals();
    }

    pub fn jobs_add_new_todb(
        &mut self,
        site: &String,
        query: &str,
        time_offset: usize,
        current_time: usize,
        committype: &CommitType,
    ) {
        let querya = query.split(' ').map(|s| s.to_string()).collect();
        self._inmemdb.jobref_new(
            site.to_string(),
            querya,
            current_time,
            time_offset,
            committype.clone(),
        );
    }

    ///
    /// New jobs adding management.
    /// Will not add job to db is time is now.
    ///
    pub fn jobs_add_new(
        &mut self,
        site: &String,
        query: &String,
        time: &String,
        committype: &CommitType,
        addtodb: bool,
    ) {
        //let a1: String = time.to_string();
        dbg!(&time);
        let current_time: usize = time::time_secs();
        let time_offset: usize = time::time_conv(time);

        self.jobs_add_new_todb(site, query, time_offset, current_time, committype);
        if addtodb {
            let inp = "INSERT INTO Jobs VALUES(?, ?, ?, ?, ?)";
            let _out = self._conn.borrow_mut().execute(
                inp,
                params![
                    current_time.to_string(),
                    time_offset.to_string(),
                    site,
                    query,
                    committype.to_string()
                ],
            );
            self.db_commit_man();
        }
    }

    ///
    /// Adds job to system.
    /// Will not add job to system if time is now.
    ///
    pub fn jobs_add_maian(
        &mut self,
        jobs_time: String,
        jobs_rep: &str,
        jobs_site: String,
        jobs_param: String,
        does_loop: bool,
        jobs_commit: String,
        jobs_todo: sharedtypes::CommitType,
    ) {
        //let time_offset: usize = time::time_conv(jobs_rep);

        /*self._inmemdb.jobs_add(
            jobs_time.parse::<usize>().unwrap(),
            time_offset,
            jobs_site.to_string(),
            jobs_param.to_string(),
        );*/
        let a1: String = jobs_time.to_string();
        let a: usize = a1.parse::<usize>().unwrap();
        let com: bool = jobs_commit.parse::<bool>().unwrap();
        if &jobs_time != "now" && does_loop {
            let b: usize = jobs_rep.parse::<usize>().unwrap();
            self.jobs_add(a, b, &jobs_site, &jobs_param, com, true, &jobs_todo);
        } else {
            self.jobs_add(a, 0, &jobs_site, &jobs_param, com, false, &jobs_todo);
        }
    }

    ///
    /// Wrapper
    ///
    pub fn jobs_get_all(
        &self,
    ) -> &HashMap<usize, JobsRef, BuildHasherDefault<NoHashHasher<usize>>> {
        self._inmemdb.jobs_get_all()
    }

    ///
    /// Wrapper
    ///
    pub fn jobs_get_new(&self, id: &usize) -> &JobsRef {
        self._inmemdb.jobs_get_new(id)
    }

    ///
    /// Pull job by id
    /// TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    ///
    pub fn jobs_get(&self, id: usize) -> (String, String, String, String, bool) {
        if self._inmemdb.jobs_exist(id) {
            self._inmemdb.jobs_get(id)
        } else {
            (
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                false,
            )
        }
    }

    pub fn tag_id_get(&mut self, uid: &usize) -> (String, String) {
        self._inmemdb.tag_id_get(uid)
    }

    /*pub fn relationship_get_fileid(&self, tag: &usize) -> Vec<usize> {
        self._inmemdb.relationship_get_fileid(tag)
    }*/

    pub fn relationship_get_one_fileid(&self, tag: &usize) -> Option<usize> {
        self._inmemdb.relationship_get_one_fileid(tag)
    }

    pub fn relationship_get_tagid(&self, tag: &usize) -> Vec<usize> {
        self._inmemdb.relationship_get_tagid(tag)
    }

    pub fn settings_get_name(&self, name: &String) -> Result<(usize, String), &str> {
        self._inmemdb.settings_get_name(name)
    }

    ///
    /// Returns total jobs from _inmemdb
    ///
    pub fn jobs_get_max(&self) -> usize {
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
        let conmut = self._conn.borrow_mut();
        let mut toexec = conmut.prepare(&parsedstring).unwrap();
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

    ///
    /// Get table names
    /// Returns: Vec of strings
    ///
    pub fn table_names(&mut self, table: String) -> Vec<String> {
        let conmut = self._conn.borrow_mut();
        let mut toexec = conmut
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
        outvec
    }

    pub fn pull_data<'a>(
        &mut self,
        table: String,
        collumn: String,
        search_term: String,
    ) -> Vec<&'a str> {
        let name = "Tags".to_string();
        let list = vec!["a", "b"];

        //println!("PRAGMA table_info({});", &table);

        let a = self.table_names(table);

        for each in a {
            //println!("{}", each);
            self.table_collumns(each);
        }

        list
    }

    ///Sets up first database interaction.
    ///Makes tables and does first time setup.
    pub fn first_db(&mut self) {
        //Checking if file exists. If doesn't then no write perms.
        let dbexists = Path::new(&self.get_db_loc()).exists();

        if !dbexists {
            panic!("No database write perms or file not created");
        }
        self.transaction_start();

        // Making File Table
        let mut name = "File".to_string();
        let mut keys = vec_of_strings!["id", "hash", "extension", "location"];
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

        // Making Parents Table. Relates tags to tag parents.
        name = "Parents".to_string();
        keys = vec_of_strings![
            "tag_namespace_id",
            "tag_id",
            "relate_namespace_id",
            "relate_tag_id"
        ];
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
        keys = vec_of_strings!["time", "reptime", "site", "param", "CommitType"];
        vals = vec_of_strings!["TEXT", "TEXT", "TEXT", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        self.transaction_flush();
    }
    pub fn updatedb(&mut self) {
        self._inmemdb.settings_add(
            "DBCOMMITNUM".to_string(),
            "Number of transactional items before pushing to db.".to_string(),
            3000,
            "None".to_string(),
        );

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
            "DIYHydrus/1.0".to_string(),
            true,
        );
        self.setting_add(
            "pluginloadloc".to_string(),
            "Where plugins get loaded into.".to_string(),
            0,
            "./plugins".to_string(),
            true,
        );
        self.setting_add(
            "DBCOMMITNUM".to_string(),
            "Number of transactional items before pushing to db.".to_string(),
            3000,
            "None".to_string(),
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

    ///
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
    }

    ///
    /// Wrapper
    ///
    pub fn file_get_hash(&self, hash: &String) -> (usize, bool) {
        self._inmemdb.file_get_hash(hash)
    }

    ///
    /// Wrapper
    ///
    pub fn tag_get_name(&self, tag: String, namespace: usize) -> (usize, bool) {
        self._inmemdb.tags_get(tag, namespace)
    }

    ///
    ///
    ///
    ///
    fn db_commit_man(&mut self) {
        self._dbcommitnum += 1;
        let general = self
            .settings_get_name(&"DBCOMMITNUM".to_string())
            .unwrap()
            .0;
        if self._dbcommitnum >= general {
            dbg!(self._dbcommitnum, general);
            self.transaction_flush();
            self._dbcommitnum = 0;
            dbg!(self._dbcommitnum, general);
        }
    }

    ///
    /// db get namespace wrapper
    /// Returns 0, false if namespace doesn't exist.
    ///
    pub fn namespace_get(&mut self, inp: &String) -> (usize, bool) {
        self._inmemdb.namespace_get(inp)
    }

    pub fn db_commit_man_set(&mut self) {
        self._dbcommitnum = self
            .settings_get_name(&"DBCOMMITNUM".to_string())
            .unwrap()
            .0;
        dbg!(
            self._dbcommitnum,
            self.settings_get_name(&"DBCOMMITNUM".to_string())
                .unwrap()
                .0
        );
    }

    ///
    /// Adds a file into the db.
    /// Do this first.
    ///
    pub fn file_add(
        &mut self,
        hash: String,
        extension: String,
        location: String,
        addtodb: bool,
    ) -> usize {
        let file_grab: (usize, bool) = self._inmemdb.file_get_hash(&hash);

        let file_id = self._inmemdb.file_put(&hash, &extension, &location);

        if addtodb && !file_grab.1 {
            let inp = "INSERT INTO File VALUES(?, ?, ?, ?)";
            let _out = self._conn.borrow_mut().execute(
                inp,
                params![
                    &file_id.to_string(),
                    &hash.to_string(),
                    &extension.to_string(),
                    &location.to_string()
                ],
            );
            self.db_commit_man();
        }
        file_id
    }

    ///
    /// Wrapper for inmemdb function: file_get_id
    /// Returns info for file in Option
    // DO NOT USE UNLESS NECISSARY. LOG(n2) * 3
    ///
    pub fn file_get_id(&self, fileid: &usize) -> Option<(String, String, String)> {
        self._inmemdb.file_get_id(fileid)
    }

    ///
    /// Adds namespace into DB.
    /// Returns the ID of the namespace.
    ///
    pub fn namespace_add(&mut self, name: &String, description: &String, addtodb: bool) -> usize {
        let namespace_grab: (usize, bool) = self._inmemdb.namespace_get(name);

        let name_id = self._inmemdb.namespace_put(name);
        if addtodb && !namespace_grab.1 {
            let inp = "INSERT INTO Namespace VALUES(?, ?, ?)";
            let _out = self._conn.borrow_mut().execute(
                inp,
                params![&name_id.to_string(), &name.to_string(), &description],
            );
            self.db_commit_man();
        }
        name_id
    }

    ///
    /// Wrapper that handles inserting parents info into DB.
    ///
    fn parents_add_db(
        &mut self,
        tag_namespace_id: &usize,
        tag_id: &usize,
        relate_namespace_id: &usize,
        relate_tag_id: &usize,
    ) {
        let inp = "INSERT INTO Parents VALUES(?, ?, ?, ?)";
        let _out = self._conn.borrow_mut().execute(
            inp,
            params![
                tag_namespace_id.to_string(),
                tag_id.to_string(),
                relate_namespace_id.to_string(),
                relate_tag_id.to_string()
            ],
        );
        self.db_commit_man();
    }

    ///
    /// Wrapper for inmemdb and parents_add_db
    ///
    pub fn parents_add(
        &mut self,
        tag_namespace_id: usize,
        tag_id: usize,
        relate_namespace_id: usize,
        relate_tag_id: usize,
        addtodb: bool,
    ) {
        let todo =
            self._inmemdb
                .parents_put(tag_namespace_id, tag_id, relate_namespace_id, relate_tag_id);

        match todo {
            tag_relate_conjoin::Tag => {
                if addtodb {
                    self.parents_add_db(
                        &tag_namespace_id,
                        &tag_id,
                        &relate_namespace_id,
                        &relate_tag_id,
                    );
                }
            }
            tag_relate_conjoin::Error => {}
            tag_relate_conjoin::Relate => {
                if addtodb {
                    self.parents_add_db(
                        &tag_namespace_id,
                        &tag_id,
                        &relate_namespace_id,
                        &relate_tag_id,
                    );
                }
            }
            tag_relate_conjoin::Conjoin => {
                if addtodb {
                    self.parents_add_db(
                        &tag_namespace_id,
                        &tag_id,
                        &relate_namespace_id,
                        &relate_tag_id,
                    );
                }
            }
            tag_relate_conjoin::Tag_and_Relate => {
                if addtodb {
                    self.parents_add_db(
                        &tag_namespace_id,
                        &tag_id,
                        &relate_namespace_id,
                        &relate_tag_id,
                    );
                }
            }
            tag_relate_conjoin::None => {}
        }
    }

    ///
    /// Adds tag into DB if it doesn't exist in the memdb.
    ///
    pub fn tag_add(
        &mut self,
        tags: String,
        parents: String,
        namespace: usize,
        addtodb: bool,
    ) -> usize {
        let tags_grab: (usize, bool) = self._inmemdb.tags_get(tags.to_string(), namespace);
        let tag_id = self._inmemdb.tags_put(&tags, &namespace);
        //println!("{} {} {} {:?} {}", tags, namespace, addtodb, tags_grab, tag_id);
        if addtodb && !tags_grab.1 {
            let inp = "INSERT INTO Tags VALUES(?, ?, ?, ?)";
            let _out = self._conn.borrow_mut().execute(
                inp,
                params![
                    &tag_id.to_string(),
                    &tags.to_string(),
                    &parents,
                    &namespace.to_string()
                ],
            );
            self.db_commit_man();
        }
        tag_id
    }

    ///
    /// Adds relationship into DB.
    /// Inherently trusts user user to not duplicate stuff.
    ///
    pub fn relationship_add(&mut self, file: usize, tag: usize, addtodb: bool) {
        let existcheck = self._inmemdb.relationship_get(file, tag);

        if addtodb && !existcheck {
            let inp = "INSERT INTO Relationship VALUES(?, ?)";
            let _out = self
                ._conn
                .borrow_mut()
                .execute(inp, params![&file.to_string(), &tag.to_string()]);
            self.db_commit_man();
        }

        self._inmemdb.relationship_add(file, tag);
    }

    pub fn jobs_add(
        &mut self,
        time: usize,
        reptime: usize,
        site: &String,
        param: &String,
        filler: bool,
        addtodb: bool,
        committype: &sharedtypes::CommitType,
    ) {
        if addtodb {
            let inp = "INSERT INTO Jobs VALUES(?, ?, ?, ?, ?)";
            let _out = self._conn.borrow_mut().execute(
                inp,
                params![
                    time.to_string(),
                    reptime.to_string(),
                    site,
                    param,
                    committype.to_string()
                ],
            );
            self.db_commit_man();
        }
        dbg!(&filler, &addtodb);
        self._inmemdb
            .jobs_add(time, reptime, site.to_string(), param.to_string(), filler);
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
            let _ex = self._conn.borrow_mut().execute(
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
                    if num == temp {
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
                Ok(_ex) => self.db_commit_man(),
            }
        }
        // Adds setting into memdbb
        if num >= 0 {
            self._inmemdb
                .settings_add(name, pretty, num.try_into().unwrap(), param);
        } else {
            self._inmemdb.settings_add(name, pretty, 0, param);
        }
        self.transaction_flush();
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
        self._dbcommitnum = 0;
    }

    /// Returns db location as String refernce.
    pub fn get_db_loc(&self) -> String {
        self._dbpath.to_string()
    }

    ///
    /// Querys the db use this for select statements.
    /// NOTE USE THIS ONY FOR RESULTS THAT RETURN STRINGS
    ///
    pub fn quer_str(&mut self, inp: String) -> Result<Vec<String>> {
        let conmut = self._conn.borrow_mut();

        let mut toexec = conmut.prepare(&inp).unwrap();
        let rows = toexec.query_map([], |row| row.get(0)).unwrap();
        let mut out = Vec::new();

        for each in rows {
            out.push(each.unwrap());
        }
        Ok(out)
    }

    ///
    /// Querys the db use this for select statements.
    /// NOTE USE THIS ONY FOR RESULTS THAT RETURN INTS
    ///
    pub fn quer_int(&mut self, inp: String) -> Result<Vec<isize>> {
        dbg!(&inp);
        let conmut = self._conn.borrow_mut();
        let mut toexec = conmut.prepare(&inp).unwrap();
        let rows = toexec.query_map([], |row| row.get(0)).unwrap();
        let mut out: Vec<isize> = Vec::new();

        for each in rows {
            let temp: String = each.unwrap();
            out.push(temp.parse::<isize>().unwrap());
        }
        dbg!(&out);
        Ok(out)
    }

    /// Raw Call to database. Try to only use internally to this file only.
    /// Doesn't support params nativly.
    /// Will not write changes to DB. Have to call write().
    /// Panics to help issues.
    pub fn execute(&mut self, inp: String) -> usize {
        let _out = self._conn.borrow_mut().execute(&inp, params![]);

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

    ///
    /// Deletes an item from jobs table.
    /// critera is the searchterm and collumn is the collumn to target.
    /// Doesn't remove from inmemory database
    ///
    pub fn del_from_jobs_table(&mut self, collumn: &String, critera: &String) {
        let delcommand = format!("DELETE FROM Jobs WHERE {} LIKE '{}'", collumn, critera);
        self.execute(delcommand);
    }

    /// Handles transactional pushes.
    pub fn transaction_execute(trans: Transaction, inp: String) {
        trans.execute(&inp, params![]).unwrap();
    }
}
