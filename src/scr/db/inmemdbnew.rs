use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::BuildHasherDefault;

use crate::jobs;
use crate::logging;
///
/// New in memory database.
/// Should be cleaner
///
use crate::sharedtypes;
use crate::sharedtypes::DbFileObj;
use crate::sharedtypes::DbJobsObj;
use crate::sharedtypes::DbTagObjCompatability;
use nohash_hasher::BuildNoHashHasher;
use nohash_hasher::IntMap;
use nohash_hasher::IntSet;
use nohash_hasher::NoHashHasher;

pub enum TagRelateConjoin {
    Tag,
    Error,
    Relate,
    Conjoin,
    TagAndRelate,
    None,
}

pub struct NewinMemDB {
    _tag_id_data: IntMap<usize, sharedtypes::DbTagObjNNS>,
    //_tag_name_id: HashMap<String, usize>,
    _tag_nns_id_data: IntMap<usize, sharedtypes::DbTagNNS>,
    _tag_nns_data_id: HashMap<sharedtypes::DbTagNNS, usize>,

    _settings_id_data: IntMap<usize, sharedtypes::DbSettingObj>,
    _settings_name_id: HashMap<String, usize>,

    _parents_id_data: IntMap<usize, sharedtypes::DbParentsObj>,
    _parents_name_id: HashMap<sharedtypes::DbParentsObj, usize>,

    _jobs_id_data: IntMap<usize, sharedtypes::DbJobsObj>,
    //_jobs_name_id: HashMap<String, usize>,
    _namespace_id_data: IntMap<usize, sharedtypes::DbNamespaceObj>,
    _namespace_name_id: HashMap<String, usize>,

    _file_id_data: IntMap<usize, sharedtypes::DbFileObj>,
    _file_name_id: HashMap<String, usize>,

    _relationship_file_tag: IntMap<usize, IntSet<usize>>,
    _relationship_tag_file: IntMap<usize, IntSet<usize>>,
    _relationship_dual: IntMap<usize, usize>,
    //_relationship_dual_f: IntMap<usize, usize>,

    _tag_max: usize,
    _relationship_max: usize,
    _settings_max: usize,
    _parents_max: usize,
    _jobs_max: usize,
    _namespace_max: usize,
    _file_max: usize,
}

impl NewinMemDB {
    pub fn new() -> NewinMemDB {
        let inst = NewinMemDB {
            //_tag_name_id: HashMap::new(),
            _tag_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),

            _tag_nns_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),
            _tag_nns_data_id: HashMap::new(),

            _jobs_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),
            _file_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),
            //_jobs_name_id: HashMap::new(),
            _file_name_id: HashMap::new(),
            _parents_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),
            _parents_name_id: HashMap::new(),
            
            _settings_name_id: HashMap::new(),
            _settings_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),
            
            _namespace_name_id: HashMap::new(),
            _namespace_id_data: HashMap::with_hasher(BuildNoHashHasher::default()),
            
            _relationship_tag_file: IntMap::default(),
            _relationship_file_tag: IntMap::default(),
            _relationship_dual: IntMap::default(),
            //_relationship_dual_t: HashMap::with_hasher(BuildNoHashHasher::default()),
            
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
    /// Dumps db data
    ///
    pub fn dumpe_data(&self) {
        use crate::pause;
        
        dbg!(&self._tag_id_data, &self._tag_nns_id_data , &self._tag_nns_data_id);
        pause();
        
        
        //&self. , &self. , &self. , &self. , &self. , &self. 
        dbg!( &self._file_name_id , &self._file_id_data );
        pause();
        dbg!(&self._relationship_tag_file);
        pause();
        dbg!(&self._parents_name_id);
        dbg!(&self._parents_id_data);
        pause();
    }
    
    
    ///
    /// Adds setting into internal DB.
    ///
    pub fn settings_add(
        &mut self,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        if !self._settings_name_id.contains_key(&name) {
            self._settings_name_id
                .insert(name.to_owned(), self._settings_max);
            //self._settings_name_id[&name] = self._settings_max;

            self._settings_id_data.insert(
                self._settings_max,
                sharedtypes::DbSettingObj {
                    name: name,
                    pretty: pretty,
                    num: num,
                    param: param,
                },
            );
            self._settings_max += 1;
        }
    }

    ///
    /// Adds jobs into internal db.
    ///
    pub fn jobref_new(&mut self, job: sharedtypes::DbJobsObj) {
        self._jobs_id_data.insert(self._jobs_max, job);
        self._jobs_max += 1;
    }

    ///
    /// Gets tag by id
    ///
    pub fn tag_id_get(&self, uid: &usize) -> Option<&sharedtypes::DbTagNNS> {
        match self._tag_nns_id_data.get(uid) {
            None => None,
            Some(nns) => {
                Some(nns)
            }
        }
    }

    ///
    /// Returns tag by ID.
    ///
    pub fn settings_get_name(&self, name: &String) -> Option<&sharedtypes::DbSettingObj> {
        match self._settings_name_id.get(name) {
            None => {
                return None;
            }
            Some(nameref) => return Some(&self._settings_id_data[nameref]),
        }
    }

    ///
    /// Returns namespace id by string
    ///
    pub fn namespace_get(&self, inp: &String) -> Option<&usize> {
        match self._namespace_name_id.get(inp) {
            None => {
                return None;
            }
            Some(inpref) => return Some(inpref),
        }
    }

    ///
    /// Returns namespace obj by id
    ///
    pub fn namespace_id_get(&self, id: &usize) -> Option<&sharedtypes::DbNamespaceObj> {
        self._namespace_id_data.get(id)
    }

    ///
    /// Returns namespace string by id
    ///
    pub fn namespace_get_string(&self, inp: &usize) -> Option<&sharedtypes::DbNamespaceObj> {
        match self._namespace_id_data.get(inp) {
            None => None,
            Some(nameref) => return Some(nameref),
        }
    }

    ///
    /// Returns a list of file id's based on a tag id.
    ///
    pub fn relationship_get_fileid(&self, tag: &usize) -> Option<&IntSet<usize>> {
        match self._relationship_tag_file.get(tag) {
            None => None,
            Some(relref) => return Some(relref),
        }
    }

    ///
    /// Returns a list of tag id's based on a file id
    ///
    pub fn relationship_get_tagid(&self, file: &usize) -> Option<&IntSet<usize>> {
        match self._relationship_file_tag.get(file) {
            None => None,
            Some(relref) => return Some(relref),
        }
    }

    ///
    /// relationship gets only one fileid
    ///
    pub fn relationship_get_one_fileid(&self, tag: &usize) -> Option<&usize> {
        match self._relationship_tag_file.get(tag) {
            None => None,
            Some(relref) => {
                if relref.is_empty() {
                    return None;
                } else {
                    let temp = relref.iter();
                    let mut temp_int = &0;
                    for each in temp {
                        temp_int=each;
                        break;
                    }
                    return Some(&temp_int)
                    //return Some(&relref.take(&0).unwrap());
                }
            }
        }
    }

    ///
    /// Returns the tag id of the nns
    ///
    pub fn tags_get_id(&self, tagobj: &sharedtypes::DbTagNNS) -> Option<&usize> {
        self._tag_nns_data_id.get(tagobj)
    }

    ///
    /// Returns the tag from id
    ///
    pub fn tags_get_data(&self, id: &usize) -> Option<&sharedtypes::DbTagNNS> {
        self._tag_nns_id_data.get(id)
    }

    ///
    /// Returns the max id in db
    ///
    pub fn tags_max_return(&self) -> &usize {
        &self._tag_max
    }

    ///
    /// inserts file into db returns file id
    ///
    pub fn file_put(&mut self, file: sharedtypes::DbFileObj) -> usize {
        let id = match file.id {
            None => self._file_max,
            Some(rid) => rid,
        };

        match file.id {
            None => {
                self._file_max += 1;
            }
            Some(_) => {}
        }

        self._file_name_id.insert(file.hash.to_owned(), id);
        self._file_id_data.insert(id, file);
        id
    }

    ///
    /// Returns a file based on ID
    ///
    pub fn file_get_id(&self, id: &usize) -> Option<&DbFileObj> {
        self._file_id_data.get(id)
    }

    ///
    /// get's file if from db hash
    ///
    pub fn file_get_hash(&self, hash: &String) -> Option<&usize> {
        self._file_name_id.get(hash)
    }

    ///
    /// Inserts namespace into db
    ///
    pub fn namespace_put(&mut self, namespace_obj: sharedtypes::DbNamespaceObj) -> usize {
        let namespace_exist = self
            ._namespace_name_id
            .insert(namespace_obj.name.to_owned(), namespace_obj.id);
        self._namespace_id_data
            .insert(namespace_obj.id, namespace_obj);
        if namespace_exist.is_none() {
            let temp = self._namespace_max;
            self._namespace_max += 1;
            return temp;
        } else {
            return self._namespace_max;
        }
    }

    ///
    /// Returns the max id of namespaces
    ///
    pub fn namespace_get_max(&self) -> usize {
        self._namespace_max
    }

    ///
    /// Adds a tag into db
    ///
    pub fn tags_put(&mut self, tag_info: sharedtypes::DbTagObjCompatability) {
        //println!("Addig tag: {:?}", &tag_info);
        let nns_obj = sharedtypes::DbTagObjNNS {
            id: tag_info.id,
            parents: tag_info.parents,
        };
        self._tag_id_data.insert(tag_info.id, nns_obj);
        self._tag_max += 1;
        if tag_info.id > self._tag_max {
            self._tag_max = tag_info.id;
        }
        self.insert_tag_nns(tag_info);
    }

    fn insert_tag_nns(&mut self, tag_info: sharedtypes::DbTagObjCompatability) {
        self._tag_nns_id_data.insert(
            tag_info.id,
            sharedtypes::DbTagNNS {
                name: tag_info.name.to_owned(),
                namespace: tag_info.namespace,
            },
        );
        self._tag_nns_data_id.insert(
            sharedtypes::DbTagNNS {
                name: tag_info.name,
                namespace: tag_info.namespace,
            },
            tag_info.id,
        );
    }

    ///
    /// checks if parents exist
    ///
    pub fn parents_get(&self, parent: &sharedtypes::DbParentsObj)-> Option<usize> {
        match self._parents_name_id.get(parent) {
            None => None,
            Some(id) => Some(id.to_owned())
        }
    }
    
    ///
    /// Puts a parent into db
    ///
    pub fn parents_put(
        &mut self,
        tag_namespace_id: usize,
        tag_id: usize,
        relate_namespace_id: usize,
        relate_tag_id: usize,
    ) -> usize {
        let parentdbojb = sharedtypes::DbParentsObj {
            tag_namespace_id: tag_namespace_id,
            tag_id: tag_id,
            relate_tag_id: relate_tag_id,
            relate_namespace_id: relate_namespace_id,
        };

        match self._parents_name_id.get(&parentdbojb) {
            None => {}
            Some(id) => return id.to_owned(),
        }
        println!("Cannot find parents {:?}", parentdbojb);

        //println!("Adding parents: {:?}", &parentdbojb);
        self._parents_id_data.insert(self._parents_max, parentdbojb);
        let parent = self._parents_name_id.insert(parentdbojb, self._parents_max);
        match parent {
            None => {
                self._parents_max += 1;
                return self._parents_max - 1;
            }
            Some(par_id) => return par_id,
        }
    }

    ///
    /// Checks if relationship exists
    ///
    pub fn relationship_get(&self, file: &usize, tag: &usize) -> bool {
        
        //let utotal: usize = [file.to_string(), tag.to_string()].concat().parse::<usize>().unwrap();
        //let relate_f = self._relationship_dual.get(&(*file, *tag));
        //let relate_t = self._relationship_dual_t.get(&tag).unwrap();
        //return relate_f.is_some();
        
        let tag_file = self._relationship_tag_file.get(tag);
        match tag_file {
            None => false,
            Some(tag_hash) => {
                let file_tag = self._relationship_file_tag.get(file);
                match file_tag {
                    None => false,
                    Some(file_hash) => {
                        if tag_hash.get(file).is_some() & file_hash.get(tag).is_some() {
                            true
                        } else {
                            false
                        }
                    }
                }
            }
        }
    }

    ///
    /// Adds relationship between db
    ///
    #[inline(always)]
    pub fn relationship_add(&mut self, file: usize, tag: usize) {
        //let utotal: usize = [file.to_string(), tag.to_string()].concat().parse::<usize>().unwrap();
        //self._relationship_dual.insert((file, tag), self._relationship_max);
        //self._relationship_dual_t.insert(tag, self._relationship_max);
        //self._relationship_max += 1;
        //return;
        
        //println!("Adding relationship : {} {}", &file, &tag);
        match self._relationship_tag_file.get_mut(&tag) {
            None => {
                let mut temp = IntSet::default();
                temp.insert(file);
                self._relationship_tag_file.insert(tag, temp);
            }

            Some(rel_tag) => {
                rel_tag.insert(file);
            }
        }

        match self._relationship_file_tag.get_mut(&file) {
            None => {
                let mut temp = IntSet::default();
                temp.insert(tag);
                self._relationship_file_tag.insert(file, temp);
            }
            Some(rel_file) => {
                rel_file.insert(tag);
            }
        }
        
        
        
    }

    pub fn jobs_add(&mut self, job: DbJobsObj) {
        self._jobs_id_data.insert(self._jobs_max, job);
        self._jobs_max += 1;
    }

    ///
    /// Get max of job
    ///
    pub fn jobs_get_max(&self) -> &usize {
        &self._jobs_max
    }

    ///
    /// Get all jobs
    ///
    pub fn jobs_get_all(
        &self,
    ) -> &HashMap<usize, sharedtypes::DbJobsObj, BuildHasherDefault<NoHashHasher<usize>>> {
        &self._jobs_id_data
    }
}
