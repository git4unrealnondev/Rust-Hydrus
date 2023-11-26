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

pub enum TagRelateConjoin {
    Tag,
    Error,
    Relate,
    Conjoin,
    TagAndRelate,
    None,
}

pub struct NewinMemDB {
    //_tag_id_data: HashMap<usize, sharedtypes::DbTagObjNNS>,
    //_tag_name_id: HashMap<String, usize>,
    _tag_nns_id_data: HashMap<usize, sharedtypes::DbTagNNS>,
    _tag_nns_data_id: HashMap<sharedtypes::DbTagNNS, usize>,

    _settings_id_data: HashMap<usize, sharedtypes::DbSettingObj>,
    _settings_name_id: HashMap<String, usize>,

    _parents_id_data: HashMap<usize, sharedtypes::DbParentsObj>,
    _parents_name_id: HashMap<sharedtypes::DbParentsObj, usize>,

    _jobs_id_data: HashMap<usize, sharedtypes::DbJobsObj>,
    //_jobs_name_id: HashMap<String, usize>,
    _namespace_id_data: HashMap<usize, sharedtypes::DbNamespaceObj>,
    _namespace_name_id: HashMap<String, usize>,
    _namespace_id_tag: HashMap<usize, HashSet<usize>>,

    _file_id_data: HashMap<usize, sharedtypes::DbFileObj>,
    _file_name_id: HashMap<String, usize>,

    _relationship_file_tag: HashMap<usize, HashSet<usize>>,
    _relationship_tag_file: HashMap<usize, HashSet<usize>>,
    _relationship_dual: HashSet<usize>,
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
            //_tag_id_data: HashMap::default(),
            _tag_nns_id_data: HashMap::default(),
            _tag_nns_data_id: HashMap::new(),

            _jobs_id_data: HashMap::default(),
            _file_id_data: HashMap::default(),
            //_jobs_name_id: HashMap::new(),
            _file_name_id: HashMap::new(),
            _parents_id_data: HashMap::default(),
            _parents_name_id: HashMap::new(),

            _settings_name_id: HashMap::new(),
            _settings_id_data: HashMap::default(),

            _namespace_name_id: HashMap::new(),
            _namespace_id_data: HashMap::default(),
            _namespace_id_tag: HashMap::default(),

            _relationship_tag_file: HashMap::default(),
            _relationship_file_tag: HashMap::default(),
            _relationship_dual: HashSet::default(),
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

        dbg!(&self._tag_nns_id_data, &self._tag_nns_data_id);
        pause();

        //&self. , &self. , &self. , &self. , &self. , &self.
        dbg!(&self._file_name_id, &self._file_id_data);
        pause();
        dbg!(&self._relationship_tag_file);
        pause();
        dbg!(&self._parents_name_id);
        dbg!(&self._parents_id_data);
        pause();
        dbg!(&self._namespace_id_data);
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
    /// Retuns raw namespace id's
    ///
    pub fn namespace_keys(&self) -> Vec<usize> {
        //let test= self._namespace_id_data;
        let mut temp: Vec<usize> = Vec::new();
        for each in self._namespace_id_data.keys() {
            temp.push(each.clone())
        }
        return temp;
    }

    ///
    /// Returns tag id's based on namespace id.
    ///
    pub fn namespace_get_tagids(&self, id: &usize) -> Option<&HashSet<usize>> {
        self._namespace_id_tag.get(id)
    }

    ///
    /// Returns a list of file id's based on a tag id.
    ///
    pub fn relationship_get_fileid(&self, tag: &usize) -> Option<&HashSet<usize>> {
        match self._relationship_tag_file.get(tag) {
            None => None,
            Some(relref) => return Some(relref),
        }
    }

    ///
    /// Returns a list of tag id's based on a file id
    ///
    pub fn relationship_get_tagid(&self, file: &usize) -> Option<&HashSet<usize>> {
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
                        temp_int = each;
                        break;
                    }
                    return Some(&temp_int);
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
    /// Removes tag from db.
    ///
    pub fn tag_remove(&mut self, id: &usize) -> Option<()> {
        let rmove = self._tag_nns_id_data.remove(id);
        match rmove {
            None => return None,
            Some(tag_data) => {
                self._tag_nns_data_id.remove(&tag_data);
                return Some(());
            }
        }
    }

    ///
    /// Removes relationship from db
    ///
    pub fn relationship_remove(&mut self, file_id: &usize, tag_id: &usize) {
        let cantor = &self.cantor_pair(file_id, tag_id);

        self._relationship_dual.remove(cantor);

        self._relationship_file_tag.remove(file_id);

        self._relationship_tag_file.remove(tag_id);
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
    /// Deletes a namespace from db
    ///
    pub fn namespace_delete(&mut self, namepsace_id: &usize) {
        let namespace_data = self._namespace_id_data.remove(namepsace_id);
        match namespace_data {
            None => return,
            Some(namespace_obj) => {
                self._namespace_name_id.remove(&namespace_obj.name);
                self._namespace_id_tag.remove(namepsace_id);
            }
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
    pub fn tags_put(&mut self, tag_info: sharedtypes::DbTagNNS) -> usize {
        let temp = self._tag_nns_data_id.get(&tag_info).clone();

        match temp {
            None => {
                let working_id = self._tag_max;

                let nns_obj = sharedtypes::DbTagObjCompatability {
                    id: working_id.clone(),
                    name: tag_info.name,
                    parents: None,
                    namespace: tag_info.namespace,
                };

                // Inserts the tagid into namespace.
                let namespace_opt = self._namespace_id_tag.get_mut(&nns_obj.namespace);
                match namespace_opt {
                    None => {
                        logging::info_log(&format!(
                            "Making namespace with id : {}",
                            &tag_info.namespace
                        ));
                        // Gets called when the namespace id wasn't found as a key
                        let mut idset = HashSet::new();
                        idset.insert(nns_obj.id);
                        self._namespace_id_tag.insert(tag_info.namespace, idset);
                    }
                    Some(namespace) => {
                        namespace.insert(nns_obj.id);
                    }
                };
                self.insert_tag_nns(nns_obj);
                self._tag_max += 1;
                return working_id;
            }
            Some(id) => return id.clone(),
        }
        /*
        //println!("Addig tag: {:?}", &tag_info);
        let nns_obj = sharedtypes::DbTagObjNNS {
            id: tag_info.id,
            //parents: tag_info.parents,
        };
        self._tag_id_data.insert(tag_info.id, nns_obj);
        self._tag_max += 1;
        if tag_info.id > self._tag_max {
            self._tag_max = tag_info.id;
        }

        // Inserts the tagid into namespace.
        let namespace_opt = self._namespace_id_tag.get_mut(&tag_info.namespace);
        match namespace_opt {
            None => {
                logging::info_log(&format!("Making namespace with id : {}", tag_info.namespace));
                // Gets called when the namespace id wasn't found as a key
                let mut idset = HashSet::new();
                idset.insert(tag_info.id);
                self._namespace_id_tag.insert(tag_info.namespace, idset);
            }
            Some(namespace) => {
                namespace.insert(tag_info.id);
            }
        };

        //namespace_inner.insert(tag_info.id);

        // Adds tag info to internal nns data.
        self.insert_tag_nns(tag_info);

        */
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
    pub fn parents_get(&self, parent: &sharedtypes::DbParentsObj) -> Option<&usize> {
        match self._parents_name_id.get(parent) {
            None => None,
            Some(id) => Some(id),
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

    fn cantor_pair(&self, n: &usize, m: &usize) -> usize {
        return (n + m) * (n + m + 1) / 2 + m;
    }

    ///
    /// Checks if relationship exists
    ///
    pub fn relationship_get(&self, file: &usize, tag: &usize) -> bool {
        //let utotal: usize = [file.to_string(), tag.to_string()].concat().parse::<usize>().unwrap();
        //let relate_f = self._relationship_dual.get(&(*file, *tag));
        let relate = self._relationship_dual.get(&self.cantor_pair(&file, &tag));
        return relate.is_some();
        /*
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
        }*/
    }

    ///
    /// Adds relationship between db
    ///
    #[inline(always)]
    pub fn relationship_add(&mut self, file: usize, tag: usize) {
        let cantor = self.cantor_pair(&file, &tag);
        self._relationship_dual.insert(cantor);

        match self._relationship_tag_file.get_mut(&tag) {
            None => {
                let mut temp = HashSet::default();
                temp.insert(file);
                self._relationship_tag_file.insert(tag, temp);
            }

            Some(rel_tag) => {
                rel_tag.insert(file);
            }
        }

        match self._relationship_file_tag.get_mut(&file) {
            None => {
                let mut temp = HashSet::default();
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
    pub fn jobs_get_all(&self) -> &HashMap<usize, sharedtypes::DbJobsObj> {
        &self._jobs_id_data
    }
}
