use crate::database::CacheType;
use crate::database::Main;
use crate::error;
use crate::logging;
use crate::sharedtypes;
use rusqlite::params;
use rusqlite::types::Null;
use rusqlite::OptionalExtension;
use rusqlite::ToSql;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::collections::HashSet;
impl Main {
    ///
    /// Gets all jobs from the sql tables
    ///
    pub fn jobs_get_all_sql(&self) -> HashMap<usize, sharedtypes::DbJobsObj> {
        let mut out = HashMap::new();
        let max_jobs = self.jobs_return_count_sql();

        for job_id in 0..max_jobs {
            if let Some(job) = self.jobs_get_id_sql(&job_id) {
                out.insert(job_id, job.clone());
            }
        }

        out
    }

    ///
    /// Returns the total count of the jobs table
    ///
    pub fn jobs_return_count_sql(&self) -> usize {
        let conn = self._conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM Jobs", params![], |row| row.get(0))
            .unwrap_or(0)
    }
    ///
    /// Returns the total count of the namespace table
    ///
    pub fn namespace_return_count_sql(&self) -> usize {
        let conn = self._conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM Namespace", params![], |row| {
            row.get(0)
        })
        .unwrap_or(0)
    }

    ///
    /// Get file if it exists by id
    ///
    pub fn files_get_id_sql(&self, file_id: &usize) -> Option<sharedtypes::DbFileStorage> {
        let conn = self._conn.lock().unwrap();
        let inp = "SELECT * FROM File where id = ?";
        conn.query_row(inp, params![file_id], |row| {
            let id = row.get(0).unwrap();
            let hash = row.get(1).unwrap();
            let ext_id = row.get(2).unwrap();
            let storage_id = row.get(3).unwrap_or(sharedtypes::DEFAULT_PRIORITY);
            Ok(Some(sharedtypes::DbFileStorage::Exist(
                sharedtypes::DbFileObj {
                    id,
                    hash,
                    ext_id,
                    storage_id,
                },
            )))
        })
        .unwrap_or(None)
    }

    ///
    /// Gets a job by id
    ///
    pub fn jobs_get_id_sql(&self, job_id: &usize) -> Option<sharedtypes::DbJobsObj> {
        let inp = "SELECT * FROM Jobs WHERE id = ?";
        let conn = self._conn.lock().unwrap();
        conn.query_row(inp, params![job_id], |row| {
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
            Ok(Some(sharedtypes::DbJobsObj {
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
            }))
        })
        .unwrap_or(None)
    }

    /// Adds a job to sql
    pub fn jobs_add_sql(&mut self, data: &sharedtypes::DbJobsObj) {
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

    /// Wrapper that handles inserting parents info into DB.
    pub fn parents_add_sql(&mut self, parent: &sharedtypes::DbParentsObj) {
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

    pub fn parents_delete_sql(&mut self, id: &usize) {
        self.parents_delete_tag_id_sql(id);
        self.parents_delete_relate_tag_id_sql(id);
        self.parents_delete_limit_to_sql(id);
    }

    pub fn does_dead_source_exist(&self, url: &String) -> bool {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id from dead_source_urls WHERE dead_url = ?",
            params![url],
            |row| Ok(row.get(0).unwrap_or(false)),
        )
        .unwrap_or(false)
    }

    ///
    /// Removes ALL of a tag_id from the parents collumn
    ///
    pub fn parents_delete_tag_id_sql(&mut self, tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM Parents WHERE tag_id = ?", params![tag_id]);
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    pub fn parents_delete_relate_tag_id_sql(&mut self, relate_tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM Parents WHERE relate_tag_id = ?",
            params![relate_tag_id],
        );
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    pub fn parents_delete_limit_to_sql(&mut self, limit_to: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM Parents WHERE limit_to = ?", params![limit_to]);
    }

    ///
    /// Gets a file storage location id
    ///
    pub fn storage_get_id(&self, location: &String) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id from FileStorageLocations where location = ?",
            params![location],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Gets tag count
    ///
    pub fn tags_max_return_sql(&self) -> usize {
        let conn = self._conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM Tags", params![], |row| row.get(0))
            .unwrap_or(0)
    }

    ///
    /// Gets a tag by id
    ///
    pub fn tags_get_dbtagnns_sql(&self, tag_id: &usize) -> Option<sharedtypes::DbTagNNS> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT name, namespace from Tags where id = ?",
            params![tag_id],
            |row| match (row.get(0), row.get(1)) {
                (Ok(Some(name)), Ok(Some(namespace_id))) => Ok(Some(sharedtypes::DbTagNNS {
                    name,
                    namespace: namespace_id,
                })),
                _ => Ok(None),
            },
        )
        .optional()
        .unwrap()?
    }

    ///
    /// Gets a list of tag ids
    ///
    pub fn tags_get_id_list_sql(&self) -> HashSet<usize> {
        let inp = "SELECT id FROM Tags";
        let conn = self._conn.lock().unwrap();

        let mut stmt = conn.prepare(inp).unwrap();
        let temp = stmt.query_map([], |row| Ok(row.get(0).unwrap())).unwrap();

        let mut out = HashSet::new();
        for item in temp {
            out.insert(item.unwrap());
        }
        out
    }

    ///
    /// Gets a string from the ID of the storage location
    ///
    pub fn storage_get_string(&self, id: &usize) -> Option<String> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT location from FileStorageLocations where id = ?",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
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
    /// Adds tags into sql database
    pub(super) fn tag_add_sql(&mut self, tag_id: &usize, tags: &String, namespace: &usize) {
        let inp = "INSERT INTO Tags VALUES(?, ?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![tag_id, tags, namespace]);
        self.db_commit_man();
    }

    /// Adds namespace to the SQL database
    pub(super) fn namespace_add_sql(
        &mut self,
        name: &String,
        description: &Option<String>,
        name_id: &usize,
    ) {
        if let Some(id) = self.namespace_get_id_sql(name) {
            return;
        }

        let inp = "INSERT INTO Namespace VALUES(?, ?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![name_id, name, description]);
        self.db_commit_man();
    }

    /// Loads Parents in from DB Connection
    pub(super) fn load_parents(&mut self) {
        if self._cache == CacheType::Bare {
            return;
        }
        logging::info_log(&"Database is Loading: Parents".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents");
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

    ///
    /// Adds a extension and an id OPTIONAL into the db
    ///
    pub fn extension_put_id_ext_sql(&mut self, id: Option<usize>, ext: &str) -> usize {
        {
            let conn = self._conn.lock().unwrap();
            conn.execute(
                "insert or ignore into FileExtensions(id, extension) VALUES (?,?)",
                params![id, ext],
            )
            .unwrap();
        }

        self.extension_get_id_sql(ext).unwrap()
    }
    ///
    /// Returns id if a hash exists
    ///
    pub fn file_get_id_sql(&self, hash: &str) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row("SELECT id FROM File WHERE hash = ?", params![hash], |row| {
            row.get(0)
        })
        .unwrap()
    }

    ///
    /// Returns if an extension exists gey by ext string
    ///
    pub fn extension_get_id_sql(&self, ext: &str) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        let out = conn
            .query_row(
                "select id from FileExtensions where extension = ?",
                params![ext],
                |row| row.get(0),
            )
            .unwrap();
        out
    }
    ///
    /// Returns if an extension exists get by id
    ///
    pub fn extension_get_string_sql(&self, id: &usize) -> Option<String> {
        let conn = self._conn.lock().unwrap();
        let out = conn
            .query_row(
                "select extension from FileExtensions where id = ?",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        out
    }

    /// Adds file via SQL
    pub(super) fn file_add_sql(&mut self, file: &sharedtypes::DbFileStorage) -> usize {
        let out_file_id;
        let file_id;
        let hash;
        let extension;
        let storage_id;
        match file {
            sharedtypes::DbFileStorage::Exist(file) => {
                file_id = Some(file.id);
                hash = file.hash.clone();
                extension = file.ext_id;
                storage_id = file.storage_id;
            }
            sharedtypes::DbFileStorage::NoIdExist(file) => {
                file_id = None;
                hash = file.hash.clone();
                extension = file.ext_id;
                storage_id = file.storage_id;
            }
            sharedtypes::DbFileStorage::NoExist(fid) => {
                todo!()
            }
            sharedtypes::DbFileStorage::NoExistUnknown => {
                todo!()
            }
        }

        let inp = "INSERT INTO File VALUES(?, ?, ?, ?)";
        let _out = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![file_id, hash, extension, storage_id]);

        if let Some(id) = file_id {
            out_file_id = id;
        } else {
            out_file_id = self.file_get_hash(&hash).unwrap();
        }

        self.db_commit_man();

        out_file_id
    }

    /// Loads Relationships in from DB connection
    pub(super) fn load_relationships(&mut self) {
        if self._cache == CacheType::Bare {
            return;
        }
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

    ///
    /// Gets a list of fileid associated with a tagid
    ///
    pub fn relationship_get_fileid_sql(&self, tag_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT fileid from Relationship where tagid = ?")
            .unwrap();
        let temp = stmt.query_map(params![tag_id], |row| row.get(0)).unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }
    ///
    /// Gets a list of tagid associated with a fileid
    ///
    pub fn relationship_get_tagid_sql(&self, file_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT tagid from Relationship where fileid = ?")
            .unwrap();
        let temp = stmt.query_map(params![file_id], |row| row.get(0)).unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    /// Loads settings into db
    pub(super) fn load_settings(&mut self) {
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

    pub(super) fn add_dead_url_sql(&mut self, url: &String) {
        let conn = self._conn.lock().unwrap();
        let _ = conn
            .execute(
                "INSERT INTO dead_source_urls(dead_url) VALUES (?)",
                params![url],
            )
            .unwrap();
    }
    ///
    /// Returns id if a tag exists
    ///
    pub fn tags_get_id_sql(&self, db_tag_nns: &sharedtypes::DbTagNNS) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id FROM Tags WHERE name = ? AND namespace = ?",
            params![db_tag_nns.name, db_tag_nns.namespace],
            |row| row.get(0),
        )
        .unwrap()
    }

    ///
    /// Returns id if a namespace exists
    ///
    pub fn namespace_get_id_sql(&self, namespace: &String) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id FROM Namespace WHERE name = ?",
            params![namespace],
            |row| row.get(0),
        )
        .unwrap_or(None)
    }
    ///
    /// Returns dbnamespace if a namespace id exists
    ///
    pub fn namespace_get_namespaceobj_sql(
        &self,
        ns_id: &usize,
    ) -> Option<sharedtypes::DbNamespaceObj> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT * FROM Namespace WHERE id = ?",
            params![ns_id],
            |row| {
                if let (Ok(id), Ok(name)) = (row.get(0), row.get(1)) {
                    Ok(Some(sharedtypes::DbNamespaceObj {
                        id,
                        name,
                        description: row.get(2).unwrap(),
                    }))
                } else {
                    Ok(None)
                }
            },
        )
        .unwrap()
    }

    ///
    /// Loads the DB into memory
    ///
    pub(super) fn load_dead_urls(&mut self) {
        logging::info_log(&"Database is Loading: dead_source_urls".to_string());

        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT * FROM dead_source_urls");

        if let Ok(mut con) = temp {
            let tag = con.query_map([], |row| {
                let url: String = row.get(0).unwrap();
                Ok(url)
            });
            match tag {
                Ok(tags) => {
                    for each in tags {
                        if let Ok(res) = each {
                            // if let Some(id) = self._inmemdb.tags_get_data(&res.id) {
                            // logging::info_log(&format!( "Already have tag {:?} adding {} {} {}", id,
                            // res.name, res.namespace, res.id )); continue;
                            // delete_tags.insert((res.name.clone(), res.namespace.clone())); }
                            self.add_dead_url(&res);
                        } else {
                            error!("Bad dead_source_url cant load {:?}", each);
                        }
                    }
                }
                Err(errer) => {
                    error!(
                        "WARNING COULD NOT LOAD dead_source_url: {:?} DUE TO ERROR",
                        errer
                    );
                }
            }
        }
    }

    /// Loads tags into db
    pub(super) fn load_tags(&mut self) {
        if self._cache == CacheType::Bare {
            return;
        }
        logging::info_log(&"Database is Loading: Tags".to_string());

        // let mut delete_tags = HashSet::new();
        {
            let binding = self._conn.clone();
            let temp_test = binding.lock().unwrap();
            let temp = temp_test.prepare("SELECT * FROM Tags");
            if let Ok(mut con) = temp {
                let tag = con.query_map([], |row| {
                    let name: String = match row.get(1) {
                        Ok(out) => out,
                        Err(err) => {
                            let temp: u64 = row.get(1).unwrap();
                            dbg!(err, temp);
                            panic!();
                        }
                    };

                    Ok(sharedtypes::DbTagObjCompatability {
                        id: row.get(0).unwrap(),
                        name,
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
        let _ = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute("PRAGMA secure_delete = 0;", params![]);
        // self.execute("PRAGMA journal_mode = MEMORY".to_string()); self.execute("PRAGMA
        // synchronous = OFF".to_string()); info!("Setting synchronous = OFF");
    }
}

#[cfg(test)]
mod tests {
    use crate::VERS;

    use super::*;

    fn setup_default_db() -> Main {
        let mut db = Main::new(None, VERS);
        db.parents_add(1, 2, Some(3), true);
        db.parents_add(2, 3, Some(4), true);
        db.parents_add(3, 4, Some(5), true);
        db
    }

    #[test]
    fn sql_parents_add() {}
    #[test]
    fn sql_parents_del_tag_id() {}
    #[test]
    fn sql_parents_del_relate_tag_id() {}
    #[test]
    fn sql_parents_del_limit_to() {}
}
