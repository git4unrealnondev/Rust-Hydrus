use std::collections::{HashMap, HashSet};

///
/// New in memory database.
/// Should be cleaner
///
use crate::sharedtypes::{self, DbSettingObj, DbFileObj};


use ::std::collections::BTreeSet;

// Stolen from: https://stackoverflow.com/questions/41035869/how-to-use-a-structs-member-as-its-own-key-when-inserting-the-struct-into-a-map


pub enum RelationshipFort {
    File(usize),
    Tag(usize),
}

pub struct newinmemdb {
    _tag_id_data: HashMap<usize, sharedtypes::DbTagObj>,
    _tag_name_id: HashMap<String, usize>,
    _settings_id_data: HashMap<usize, sharedtypes::DbSettingObj>,
    _settings_name_id: HashMap<String, usize>,
    _parents_id_data: HashMap<usize, sharedtypes::DbParentsObj>,
    _parents_name_id: HashMap<String, usize>,
    _jobs_id_data: HashMap<usize, sharedtypes::DbJobsObj>,
    _jobs_name_id: HashMap<String, usize>,
    _namespace_id_data: HashMap<usize, sharedtypes::DbNamespaceObj>,
    _namespace_name_id: HashMap<String, usize>,
    _file_id_data: HashMap<usize, sharedtypes::DbFileObj>,
    _file_name_id: HashMap<String, usize>,
    _relationship_file_tag: HashMap<usize, HashSet<usize>>,
    _relationship_tag_file: HashMap<usize, HashSet<usize>>,
    _tag_max: usize,
    _relationship_max: usize,
    _settings_max: usize,
    _parents_max: usize,
    _jobs_max: usize,
    _namespace_max: usize,
    _file_max: usize,
}

impl newinmemdb {
    pub fn new() -> newinmemdb {
        let inst = newinmemdb {
            _tag_name_id: HashMap::new(),
            _tag_id_data: HashMap::new(),
            _jobs_id_data: HashMap::new(),
            _file_id_data: HashMap::new(),
            _jobs_name_id: HashMap::new(),
            _file_name_id: HashMap::new(),
            _parents_id_data: HashMap::new(),
            _parents_name_id: HashMap::new(),
            _settings_name_id: HashMap::new(),
            _settings_id_data: HashMap::new(),
            _namespace_name_id: HashMap::new(),
            _namespace_id_data: HashMap::new(),
            _relationship_tag_file: HashMap::new(),
            _relationship_file_tag: HashMap::new(),
            _tag_max: 0,
            _settings_max: 0,
            _parents_max: 0,
            _jobs_max: 0,
            _namespace_max: 0,
            _file_max: 0,
            _relationship_max: 0,
        };

        return inst;
    }

    ///
    /// Shows Internals of the database
    ///
    pub fn dbg_show_internals(self) {
        dbg!(
            self._tag_name_id,
            self._tag_id_data,
            self._jobs_id_data,
            self._jobs_name_id,
            self._file_id_data,
            self._file_name_id,
            self._parents_id_data,
            self._parents_name_id,
            self._settings_name_id,
            self._settings_id_data,
            self._namespace_name_id,
            self._namespace_id_data
        );
    }

    // File
    

    ///
    /// Adds file into Memdb instance.
    ///
    pub fn file_put(
        &mut self,
        id: Option<usize>,
        hash: Option<String>,
        extension: Option<String>,
        location: Option<String>,
    ) -> usize { 
        
        let file = DbFileObj{
            id: id,
            hash: hash,
            ext: extension,
            location: location
        };
        
        self._file_id_data[&self._file_max] = file;
        let temp_max = self._file_max;
        self._file_max +=1;
        temp_max
    }
    
    
    // Settings

    ///
    /// Returns the setting based on string
    ///
    pub fn settings_get_name(self, id: &String) -> Option<DbSettingObj> {
        if self._settings_name_id.contains_key(id) {
            Some(self._settings_id_data[&self._settings_name_id[id]])
        } else {
            None
        }
    }

    ///
    /// Adds a setting into the DB
    ///
    pub fn settings_add(&mut self, name: String, pretty: String, num: usize, param: String) {
        let setting = DbSettingObj {
            name: name,
            pretty: Some(pretty),
            num: Some(num),
            param: Some(param),
        };
        self._settings_id_data[&self._settings_max] = setting;
        self._settings_max += 1;
    }

    // Relationships

    ///
    /// Returns a list of fileid's associated with tagid
    ///
    pub fn relationship_get_tagid(self, id: &usize) -> HashSet<usize> {
        self._relationship_tag_file[id]
    }

    ///
    /// Returns a list of tagid's associated with fileid
    ///
    pub fn relationship_get_fileid(self, id: &usize) -> HashSet<usize> {
        self._relationship_file_tag[id]
    }

    // Jobs

    ///
    /// Returns the max id that can be used.
    ///
    pub fn jobs_max(self) -> usize {
        self._jobs_max
    }

    ///
    /// Returns if an object exists from the in memory db
    ///
    pub fn jobs_get(self, id: &usize) -> Option<sharedtypes::DbJobsObj> {
        if self._jobs_id_data.contains_key(id) {
            return Some(self._jobs_id_data[id]);
        } else {
            None
        }
    }

    ///
    /// Inserts job into memdb
    ///
    pub fn jobref_new(
        &mut self,
        sites: String,
        params: String,
        jobstime: usize,
        jobsref: usize,
        committype: sharedtypes::CommitType,
    ) {
        let job = sharedtypes::DbJobsObj {
            time: Some(jobstime),
            reptime: Some(jobsref),
            site: Some(sites),
            param: Some(params),
            committype: Some(committype),
        };

        self._jobs_id_data[&self._jobs_max] = job;
        self._jobs_max += 1;
    }

    // Tags

    ///
    /// Returns the tag data object if it exists.
    ///
    pub fn tag_id_get(&mut self, id: &usize) -> Option<&sharedtypes::DbTagObj> {
        self._tag_id_data.get(id)
    }
}
