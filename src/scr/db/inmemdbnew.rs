use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::DbFileObj;
use crate::sharedtypes::DbJobsObj;
use fnv::{FnvHashMap, FnvHashSet};
use std::clone;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
pub enum TagRelateConjoin {
    Tag,
    Error,
    Relate,
    Conjoin,
    TagAndRelate,
    None,
}
pub struct tes {
    test: HashSet<usize>,
}
impl tes {
    pub fn new() -> tes {
        let te = HashSet::default();

        tes { test: te }
    }
    pub fn inc(&mut self, i: usize) {
        self.test.insert(i);
    }
}

pub struct NewinMemDB {
    _tag_nns_id_data: FnvHashMap<usize, sharedtypes::DbTagNNS>,
    _tag_nns_data_id: HashMap<sharedtypes::DbTagNNS, usize>,

    _settings_id_data: HashMap<usize, sharedtypes::DbSettingObj>,
    _settings_name_id: HashMap<String, usize>,

    _parents_dual: HashSet<usize>,

    _parents_tag_rel: HashMap<usize, HashSet<usize>>,
    _parents_rel_tag: HashMap<usize, HashSet<usize>>,
    _parents_cantor_limitto: HashMap<usize, HashSet<usize>>,
    _parents_limitto_cantor: HashMap<usize, HashSet<usize>>,

    _jobs_id_data: HashMap<usize, sharedtypes::DbJobsObj>,
    //_jobs_name_id: HashMap<String, usize>,
    _namespace_id_data: HashMap<usize, sharedtypes::DbNamespaceObj>,
    _namespace_name_id: HashMap<String, usize>,
    _namespace_id_tag: HashMap<usize, HashSet<usize>>,

    _file_id_data: HashMap<usize, sharedtypes::DbFileObj>,
    _file_location_usize: FnvHashMap<usize, String>,
    _file_location_string: FnvHashMap<String, usize>,
    _file_name_id: HashMap<String, usize>,

    _relationship_file_tag: FnvHashMap<usize, FnvHashSet<usize>>,
    _relationship_tag_file: FnvHashMap<usize, FnvHashSet<usize>>,
    _relationship_dual: FnvHashSet<usize>,

    _file_location_count: usize,
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
        NewinMemDB {
            _tag_nns_id_data: HashMap::default(),
            _tag_nns_data_id: HashMap::new(),

            _jobs_id_data: HashMap::default(),
            _file_id_data: HashMap::default(),
            _file_location_usize: HashMap::default(),
            _file_location_string: HashMap::default(),
            //_jobs_name_id: HashMap::new(),
            _file_name_id: HashMap::new(),
            _parents_dual: HashSet::default(),

            _parents_rel_tag: HashMap::default(),
            _parents_tag_rel: HashMap::default(),
            _parents_cantor_limitto: HashMap::default(),
            _parents_limitto_cantor: HashMap::default(),

            _settings_name_id: HashMap::new(),
            _settings_id_data: HashMap::default(),

            _namespace_name_id: HashMap::new(),
            _namespace_id_data: HashMap::default(),
            _namespace_id_tag: HashMap::default(),

            _relationship_tag_file: HashMap::default(),
            _relationship_file_tag: HashMap::default(),
            _relationship_dual: HashSet::default(),

            _file_location_count: 0,
            _tag_max: 0,
            _settings_max: 0,
            _parents_max: 0,
            _jobs_max: 0,
            _namespace_max: 0,
            _file_max: 0,
            _relationship_max: 0,
        }
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
        dbg!(&self._namespace_id_data);
        pause();
    }

    ///
    /// Adds setting into internal DB.
    /// Updates setting if it doesn't exist.
    ///
    pub fn settings_add(
        &mut self,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        match self._settings_name_id.get(&name) {
            None => {
                self._settings_name_id
                    .insert(name.to_owned(), self._settings_max);
                self._settings_id_data.insert(
                    self._settings_max,
                    sharedtypes::DbSettingObj {
                        name,
                        pretty,
                        num,
                        param,
                    },
                );
                self._settings_max += 1;
            }
            Some(setting_id) => {
                logging::info_log(&format!(
                    "Updating setting_id: {} with {} {:?} {:?} {:?}.",
                    &setting_id, &name, &pretty, &num, &param
                ));
                self._settings_id_data.insert(
                    *setting_id,
                    sharedtypes::DbSettingObj {
                        name,
                        pretty,
                        num,
                        param,
                    },
                );
            }
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
    /// Removes a job from the internal db.
    /// Dont want to remove from the _jobs_max count because all ID's should be unique
    /// (ensures consistency between caches of data)
    ///
    pub fn jobref_remove(&mut self, id: &usize) {
        let _ = self._jobs_id_data.remove(id);
    }

    ///
    /// Job flip is running
    ///
    pub fn jobref_flip_isrunning(&mut self, id: &usize) -> Option<bool> {
        match self._jobs_id_data.get_mut(id) {
            Some(job) => match job.isrunning {
                true => {
                    job.isrunning = false;
                    Some(false)
                }
                false => {
                    job.isrunning = true;
                    Some(true)
                }
            },
            None => None,
        }
    }

    ///
    /// Gets all running jobs
    ///
    pub fn jobref_get_isrunning(&self) -> HashSet<&sharedtypes::DbJobsObj> {
        let mut out = HashSet::new();
        for each in self._jobs_id_data.values() {
            if each.isrunning {
                out.insert(each);
            }
        }
        out
    }

    ///
    /// Returns tag by ID.
    ///
    pub fn settings_get_name(&self, name: &String) -> Option<&sharedtypes::DbSettingObj> {
        match self._settings_name_id.get(name) {
            None => None,
            Some(nameref) => Some(&self._settings_id_data[nameref]),
        }
    }

    ///
    /// Returns namespace id by string
    ///
    pub fn namespace_get(&self, inp: &String) -> Option<&usize> {
        match self._namespace_name_id.get(inp) {
            None => None,
            Some(inpref) => Some(inpref),
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
            temp.push(*each);
        }
        temp
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
    pub fn relationship_get_fileid(&self, tag: &usize) -> Option<HashSet<usize>> {
        match self._relationship_tag_file.get(tag) {
            None => None,
            Some(relref) => {
                let mut out = HashSet::default();
                for each in relref {
                    out.insert(*each);
                }
                Some(out)
            }
        }
    }

    ///
    /// Returns a list of tag id's based on a file id
    ///
    pub fn relationship_get_tagid(&self, file: &usize) -> Option<HashSet<usize>> {
        match self._relationship_file_tag.get(file) {
            None => None,
            Some(relref) => {
                return Some(HashSet::from_iter(relref.clone()));
            }
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
                    None
                } else {
                    let temp = relref.iter();
                    let mut temp_int = &0;
                    for each in temp {
                        temp_int = each;
                        break;
                    }
                    Some(temp_int)
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
    /// Clears inmemdb tags structures
    ///
    pub fn tags_clear(&mut self) {
        self._tag_max = 0;
        self._tag_nns_id_data.clear();
        self._tag_nns_data_id.clear();
    }

    ///
    /// Clears inmemdb parents structures
    ///
    pub fn parents_clear(&mut self) {
        self._parents_max = 0;
        self._parents_dual.clear();
        self._parents_rel_tag.clear();
        self._parents_tag_rel.clear();
    }

    ///
    /// Clears inmemdb relationships structures
    ///
    pub fn relationships_clear(&mut self) {
        self._relationship_max = 0;
        self._relationship_file_tag.clear();
        self._relationship_tag_file.clear();
        self._relationship_dual.clear();
    }

    ///
    ///
    ///
    pub fn tags_get_list_id(&self) -> HashSet<usize> {
        let mut temp: HashSet<usize> = HashSet::new();
        for each in self._tag_nns_id_data.keys() {
            temp.insert(each.to_owned());
        }
        temp
    }

    ///
    /// Removes tag from db.
    ///
    pub fn tag_remove(&mut self, id: &usize) -> Option<()> {
        let rmove = self._tag_nns_id_data.remove(id);
        match rmove {
            None => None,
            Some(tag_data) => {
                self._tag_nns_data_id.remove(&tag_data);
                let ns_mgmnt = self._namespace_id_tag.get_mut(&tag_data.namespace);
                match ns_mgmnt {
                    None => {}
                    Some(ns_data) => {
                        ns_data.remove(id);
                    }
                }
                Some(())
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
    pub fn tags_max_return(&self) -> usize {
        self._tag_max
    }

    ///
    /// Resets the tag counter to 0.
    ///
    pub fn tags_max_reset(&mut self) {
        self._tag_max = 0;
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
            Some(file_id) => {
                if file_id >= self._file_max {
                    self._file_max = file_id;
                    self._file_max += 1;
                }
            }
        }

        self._file_name_id.insert(file.hash.to_owned(), id);
        // let locid = self.file_location_get_id(file.location);
        self._file_id_data.insert(
            id,
            sharedtypes::DbFileObj {
                id: file.id,
                hash: file.hash,
                ext: file.ext,
                location: file.location,
            },
        );
        id
    }

    ///
    /// Gets location id from string
    ///
    fn file_location_get_id(&mut self, loc: String) -> usize {
        match self._file_location_string.get(&loc) {
            Some(num) => *num,
            None => {
                self._file_location_string
                    .insert(loc.to_owned(), self._file_location_count);
                self._file_location_usize
                    .insert(self._file_location_count, loc);
                self._file_location_count += 1;
                self._file_location_count - 1
            }
        }
    }

    ///
    /// Gets location string from id
    ///
    fn file_location_get_string(&self, id: &usize) -> Option<String> {
        self._file_location_usize.get(id).map(|num| num.to_string())
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
    /// Returns the file id's in db
    ///
    pub fn file_get_list_id(&self) -> HashSet<usize> {
        let mut temp: HashSet<usize> = HashSet::default();
        for each in self._file_id_data.keys() {
            temp.insert(each.to_owned());
        }
        temp
    }

    ///
    /// Returns all file objects in db
    ///
    pub fn file_get_list_all(&self) -> &HashMap<usize, DbFileObj> {
        &self._file_id_data
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
            temp
        } else {
            self._namespace_max
        }
    }

    ///
    /// Deletes a namespace from db
    ///
    pub fn namespace_delete(&mut self, namepsace_id: &usize) {
        let namespace_data = self._namespace_id_data.remove(namepsace_id);
        match namespace_data {
            None => (),
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
    pub fn tags_put(&mut self, tag_info: &sharedtypes::DbTagNNS, id: Option<usize>) -> usize {
        let temp = self._tag_nns_data_id.get(tag_info);

        match temp {
            None => {
                let working_id = match id {
                    None => self._tag_max,
                    Some(tid) => tid,
                };

                //let working_id = self._tag_max;

                // Inserts the tagid into namespace.
                let namespace_opt = self._namespace_id_tag.get_mut(&tag_info.namespace);
                match namespace_opt {
                    None => {
                        logging::info_log(&format!(
                            "Making namespace with id : {}",
                            &tag_info.namespace
                        ));
                        // Gets called when the namespace id wasn't found as a key
                        let mut idset = HashSet::new();
                        idset.insert(working_id);
                        self._namespace_id_tag.insert(tag_info.namespace, idset);
                    }
                    Some(namespace) => {
                        namespace.insert(working_id);
                    }
                };

                self.insert_tag_nns(sharedtypes::DbTagObjCompatability {
                    id: working_id,
                    name: tag_info.name.clone(),
                    namespace: tag_info.namespace,
                });
                self._tag_max += 1;
                match id {
                    None => {}
                    Some(id) => {
                        if self._tag_max < id {
                            self._tag_max = id;
                        }
                    }
                }
                working_id
            }
            Some(id) => *id,
        }
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
        let cantor = &self.cantor_pair(&parent.tag_id, &parent.relate_tag_id);
        match parent.limit_to {
            None => self._parents_dual.get(cantor),
            Some(limit_to_id) => match self._parents_limitto_cantor.get(&limit_to_id) {
                None => None,
                Some(limit_to_cantors) => {
                    if limit_to_cantors.contains(cantor) {
                        self._parents_dual.get(cantor)
                    } else {
                        None
                    }
                }
            },
        }
    }

    ///
    /// Returns the list of relationships assicated with tag
    ///
    pub fn parents_rel_get(
        &self,
        relate_tag_id: &usize,
        limit_to: Option<usize>,
    ) -> Option<HashSet<usize>> {
        let out = self._parents_tag_rel.get(relate_tag_id);
        if let Some(out) = out {
            let temp = out.clone();
            match limit_to {
                None => {
                    return Some(temp);
                }
                Some(limitto) => {
                    for each in temp.iter() {
                        let cantor = self.cantor_pair(each, relate_tag_id);
                        if let Some(cantorlist) = self._parents_limitto_cantor.get(&limitto) {
                            let re = temp.intersection(cantorlist);
                            let te = re.cloned().collect();
                            return Some(te);
                        }
                    }
                }
            }
        }
        None
    }

    ///
    /// Returns the list of tags assicated with relationship
    ///
    pub fn parents_tag_get(
        &self,
        tag_id: &usize,
        limit_to: Option<usize>,
    ) -> Option<HashSet<usize>> {
        let out = self._parents_rel_tag.get(tag_id);
        if let Some(out) = out {
            let temp = out.clone();
            match limit_to {
                None => {
                    return Some(temp);
                }
                Some(limitto) => {
                    for each in temp.iter() {
                        let cantor = self.cantor_pair(each, tag_id);
                        if let Some(cantorlist) = self._parents_limitto_cantor.get(&limitto) {
                            let re = temp.intersection(cantorlist);
                            let te = re.cloned().collect();
                            return Some(te);
                        }
                    }
                }
            }
        }
        None
    }

    ///
    /// Removes parent's from internal db based on tag id
    ///
    #[inline(never)]
    pub fn parents_remove(&mut self, tag_id: &usize) -> HashSet<(usize, usize)> {
        let mut ret: HashSet<(usize, usize)> = HashSet::new();
        let rel_op = self.parents_rel_get(tag_id, None);

        match rel_op {
            None => return ret,
            Some(rel_hs) => {
                for rel in rel_hs {
                    logging::info_log(&format!("Parents_Remove: {} {}", &rel, tag_id));
                    println!("Parents_Remove: {} {}", &rel, tag_id);
                    self._parents_rel_tag.remove(&rel);
                    self._parents_tag_rel.remove(tag_id);
                    let cantor = self.cantor_pair(tag_id, &rel);
                    self._parents_dual.remove(&cantor);
                    ret.insert((*tag_id, rel));
                }
            }
        }
        ret
    }

    ///
    /// Removes a list of parents based on relational tag id
    /// Returns a list of tag id's that were removed
    /// DANGEROUS AVOID TO USE IF POSSIBLE
    ///
    pub fn parents_reltagid_remove(&mut self, relate_tag_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();
        match self.parents_rel_get(relate_tag_id, None) {
            None => {}
            Some(tag_id_set) => {
                for tag_id in tag_id_set.clone() {
                    out.insert(tag_id);
                    self.parents_selective_remove(&sharedtypes::DbParentsObj {
                        tag_id,
                        relate_tag_id: *relate_tag_id,
                        limit_to: None,
                    });
                }
            }
        }
        out
    }

    ///
    /// Removes a list of parents based on tag id
    /// Returns a list of relational tag id's that were removed
    /// DANGEROUS AVOID TO USE IF POSSIBLE
    ///
    pub fn parents_tagid_remove(&mut self, tag_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();
        match self.parents_tag_get(tag_id, None) {
            None => {}
            Some(relate_tag_id_set) => {
                for relate_tag_id in relate_tag_id_set.clone() {
                    out.insert(relate_tag_id);
                    self.parents_selective_remove(&sharedtypes::DbParentsObj {
                        tag_id: *tag_id,
                        relate_tag_id,
                        limit_to: None,
                    });
                }
            }
        }
        out
    }

    ///
    /// Removes a parent selectivly from the Db
    /// USE THIS IF POSSIBLE
    ///
    pub fn parents_selective_remove(&mut self, parentobj: &sharedtypes::DbParentsObj) {
        let cantor = self.cantor_pair(&parentobj.tag_id, &parentobj.relate_tag_id);
        match self._parents_dual.remove(&cantor) {
            false => return,
            true => {
                match self._parents_cantor_limitto.get_mut(&cantor) {
                    None => {}
                    Some(set) => {
                        for each in set.iter() {
                            self._parents_limitto_cantor
                                .get_mut(each)
                                .unwrap()
                                .remove(&cantor);
                        }
                    }
                }

                match self._parents_rel_tag.get_mut(&parentobj.relate_tag_id) {
                    None => {}
                    Some(relset) => {
                        relset.remove(&parentobj.tag_id);
                    }
                }

                match self._parents_tag_rel.get_mut(&parentobj.tag_id) {
                    None => {}
                    Some(tagset) => {
                        tagset.remove(&parentobj.relate_tag_id);
                    }
                }
            }
        }
    }

    ///
    /// Puts a parent into db
    /// NOTE: If limit_to is set and their was a previous parent that didn't have a limit_to it
    /// will be overwritten.
    ///
    pub fn parents_put(&mut self, parent: sharedtypes::DbParentsObj) -> usize {
        let mut increment_parents = false;
        // Catch to prevent further processing.
        match self.parents_get(&parent) {
            None => {
                increment_parents = true;
            }
            Some(id) => return id.to_owned(),
        }

        let cantor = self.cantor_pair(&parent.tag_id, &parent.relate_tag_id);
        self._parents_dual.insert(cantor);
        let par = self._parents_max;
        self._parents_max += 1;

        // Manages links betwen cantor and limit arguments
        if let Some(limitto) = parent.limit_to {
            match self._parents_cantor_limitto.get_mut(&cantor) {
                None => {
                    let mut out = HashSet::new();
                    out.insert(limitto);
                    self._parents_cantor_limitto.insert(cantor, out);
                }
                Some(parentcantor) => {
                    parentcantor.insert(limitto);
                }
            }

            match self._parents_limitto_cantor.get_mut(&limitto) {
                None => {
                    increment_parents = false;
                    let mut out = HashSet::new();
                    out.insert(cantor);
                    self._parents_limitto_cantor.insert(limitto, out);
                }
                Some(parentlimit) => {
                    parentlimit.insert(cantor);
                }
            }
        }

        //Manages the relations and tags between parents
        let rel = self._parents_rel_tag.get_mut(&parent.relate_tag_id);
        match rel {
            None => {
                let mut temp: HashSet<usize> = HashSet::default();
                temp.insert(parent.tag_id);
                self._parents_rel_tag.insert(parent.relate_tag_id, temp);
            }
            Some(rel_id) => {
                rel_id.insert(parent.tag_id); //parents_tag_get
            }
        }

        let tag = self._parents_tag_rel.get_mut(&parent.tag_id);
        match tag {
            None => {
                let mut temp: HashSet<usize> = HashSet::default();
                temp.insert(parent.relate_tag_id);
                self._parents_tag_rel.insert(parent.tag_id, temp);
            }
            Some(tag_id) => {
                tag_id.insert(parent.relate_tag_id);
            }
        }
        // If we've been setup to increment then we should have this in the parents list
        if increment_parents {
            par
        } else {
            *self.parents_get(&parent).unwrap()
        }
    }

    ///
    /// Removes a parent from the internal db
    ///
    pub fn parents_remove_id(&mut self, parent_id: &usize) {
        if let Some(pid) = self._parents_dual.get(parent_id) {
            let lim = self._parents_cantor_limitto.remove(parent_id);
            if let Some(lim) = lim {
                for each in lim.iter() {
                    if let Some(can) = self._parents_limitto_cantor.get_mut(each) {
                        can.remove(parent_id);
                    }
                }
            }
            let (tag_id, relate_tag_id) = self.cantor_unpair(pid);
            if let Some(rels) = self._parents_tag_rel.get_mut(&tag_id) {
                rels.remove(&relate_tag_id);
            }
            if let Some(tags) = self._parents_rel_tag.get_mut(&relate_tag_id) {
                tags.remove(&tag_id);
            }
        }
    }

    ///
    /// Gets a unique value based on two inputs
    ///
    fn cantor_pair(&self, n: &usize, m: &usize) -> usize {
        (n + m) * (n + m + 1) / 2 + m
    }

    ///
    /// Gets the unique inputs from a cantor number
    ///
    fn cantor_unpair(&self, z: &usize) -> (usize, usize) {
        let w64 = (8 * z + 1) as f64;
        let w64two = ((w64.sqrt() - 1.0) / 2.0).floor() as usize;
        let t = (w64two * w64two + w64two) / 2;
        let m = z - t;
        let n = w64two - m;
        return (n, m);
    }

    ///
    /// Checks if relationship exists
    ///
    pub fn relationship_get(&self, file: &usize, tag: &usize) -> bool {
        //let utotal: usize = [file.to_string(), tag.to_string()].concat().parse::<usize>().unwrap();
        //let relate_f = self._relationship_dual.get(&(*file, *tag));
        let relate = self._relationship_dual.get(&self.cantor_pair(file, tag));
        relate.is_some()
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
                let mut temp = FnvHashSet::default();
                temp.insert(file);
                self._relationship_tag_file.insert(tag, temp);
            }

            Some(rel_tag) => {
                rel_tag.insert(file);
            }
        }
        match self._relationship_file_tag.get_mut(&file) {
            None => {
                let mut temp = FnvHashSet::default();
                temp.insert(tag);
                self._relationship_file_tag.insert(file, temp);
            }
            Some(rel_file) => {
                rel_file.insert(tag);
            }
        }
    }

    ///
    /// Adds job into internal db
    ///
    pub fn jobs_add(&mut self, job: DbJobsObj) {
        self._jobs_id_data.insert(self._jobs_max, job);
        self._jobs_max += 1;
    }

    ///
    /// Gets a job by id
    ///
    pub fn jobs_get(&self, id: &usize) -> Option<&sharedtypes::DbJobsObj> {
        self._jobs_id_data.get(id)
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

#[cfg(test)]
mod inmemdb {
    use std::collections::BTreeMap;

    use sharedtypes::DbParentsObj;

    use super::*;

    fn setup_db() -> NewinMemDB {
        inmemdb::NewinMemDB::new()
    }

    ///
    /// Tests if we can setup the db.
    ///
    #[test]
    fn test_setup_db() {
        let _ = setup_db();
    }

    ///
    /// Tests if the integration with putting parents are deduplicated properly
    ///
    #[test]
    fn parents_add() {
        let mut db = setup_db();
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });
        assert!(db._parents_max == 1);
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: None,
        });
        assert!(db._parents_max == 2);
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: Some(1),
        });
        assert_eq!(db._parents_max, 3);
    }

    ///
    /// Tests if integration with getting works
    ///
    #[test]
    fn parents_get() {
        let mut db = setup_db();
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });
        assert!(db._parents_max == 1);
        let rela = db.parents_get(&DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });

        assert_eq!(rela, Some(&0));
        let rela = db.parents_get(&DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: Some(3),
        });

        assert_eq!(rela, None);
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: Some(3),
        });
        let rela = db.parents_get(&DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: Some(3),
        });
        assert_eq!(rela, Some(&0));
        let rela = db.parents_get(&DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });
        assert_eq!(rela, Some(&0));

        assert_eq!(db._parents_max, 2);
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: Some(3),
        });
        let rela = db.parents_get(&DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: None,
        });

        assert_eq!(rela, Some(db.cantor_pair(&1, &0)).as_ref());
    }

    #[test]
    fn parents_rel_get() {
        let mut db = setup_db();
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 1,
            limit_to: Some(1),
        });
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 2,
            limit_to: Some(2),
        });
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: Some(2),
        });

        let ll = db.parents_rel_get(&0, None).unwrap();
        assert_eq!(ll.len(), 2);
        let ll = db.parents_rel_get(&0, Some(1)).unwrap();
        assert_eq!(ll.len(), 1);
        assert_eq!(db.parents_rel_get(&0, Some(4)), None);

        dbg!(
            &db._parents_dual,
            &db._parents_limitto_cantor,
            &db._parents_rel_tag,
            &db._parents_tag_rel,
            &db._parents_cantor_limitto,
            &db._parents_max
        );
    }

    #[test]
    fn parents_nondedup_put() {
        let mut db = setup_db();
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: Some(1),
        });
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: Some(2),
        });
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: Some(2),
        });

        assert_eq!(db._parents_max, 2);
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });
        assert_eq!(db._parents_max, 2);

        let rela = db.parents_get(&DbParentsObj {
            tag_id: 0,
            relate_tag_id: 0,
            limit_to: None,
        });
    }

    #[test]
    fn parents_remove() {
        let mi = 3;
        let mj = 3;
        let mut db = setup_db();
        for i in 0..mi {
            for j in 0..mj {
                db.parents_put(sharedtypes::DbParentsObj {
                    tag_id: i,
                    relate_tag_id: j,
                    limit_to: None,
                });
            }
        }
        assert_eq!(db._parents_max, mi * mj);
        db.parents_selective_remove(&DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: None,
        });
        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 100,
            relate_tag_id: 100,
            limit_to: Some(50),
        });

        db.parents_put(sharedtypes::DbParentsObj {
            tag_id: 100,
            relate_tag_id: 101,
            limit_to: Some(50),
        });

        let rela = db.parents_get(&DbParentsObj {
            tag_id: 1,
            relate_tag_id: 0,
            limit_to: None,
        });

        assert_ne!(rela, Some(db.cantor_pair(&1, &0)).as_ref());
        db.parents_selective_remove(&DbParentsObj {
            tag_id: 100,
            relate_tag_id: 101,
            limit_to: Some(60),
        });
        let rela = db.parents_get(&DbParentsObj {
            tag_id: 100,
            relate_tag_id: 101,
            limit_to: None,
        });

        assert_eq!(db._parents_max, 11);

        assert_ne!(rela, Some(db.cantor_pair(&100, &101)).as_ref());

        dbg!(
            &db._parents_dual,
            &db._parents_limitto_cantor,
            &db._parents_rel_tag,
            &db._parents_tag_rel,
            &db._parents_cantor_limitto,
            &db._parents_max
        );
    }

    #[test]
    fn jobs_check() {
        let mut db = setup_db();
        let none = db.jobs_get(&0);
        assert_eq!(None, none);

        let jobobj = sharedtypes::DbJobsObj {
            id: *db.jobs_get_max(),
            time: None,
            reptime: None,
            site: "test".to_owned(),
            param: None,
            jobmanager: sharedtypes::DbJobsManager {
                jobtype: sharedtypes::DbJobType::Params,
                recreation: None,
                additionaldata: None,
            },
            committype: None,
            isrunning: false,
            system_data: BTreeMap::new(),
            user_data: BTreeMap::new(),
        };

        db.jobs_add(jobobj.clone());
        assert_eq!(db.jobs_get(&0).unwrap(), &jobobj);
    }
}
