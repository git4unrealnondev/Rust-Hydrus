use crate::database::CacheType;
use crate::database::Main;
use crate::error;
use crate::logging;
use crate::sharedtypes;
use rusqlite::OptionalExtension;
use rusqlite::ToSql;
use rusqlite::params;
use rusqlite::types::Null;
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
        let max = conn
            .query_row("SELECT MAX(id) FROM Jobs", params![], |row| row.get(0))
            .unwrap_or(0);

        let count = conn
            .query_row("SELECT COUNT(*) FROM Jobs", params![], |row| row.get(0))
            .unwrap_or(0);

        if max > count { max + 1 } else { count + 1 }
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
    /// Get file if it exists by id
    ///
    pub fn namespace_get_tagids_sql(&self, ns_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();
        let conn = self._conn.lock().unwrap();
        let mut inp = conn
            .prepare("SELECT id FROM Tags where namespace = ?")
            .unwrap();
        let quer = inp
            .query_map(params![ns_id], |row| {
                let id = row.get(0).unwrap();
                Ok(id)
            })
            .unwrap();

        for each in quer.flatten() {
            out.insert(each);
        }

        out
    }

    ///
    /// Gets a job by id
    ///
    pub fn jobs_get_id_sql(&self, job_id: &usize) -> Option<sharedtypes::DbJobsObj> {
        let inp = "SELECT * FROM Jobs WHERE id = ? LIMIT 1";
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
        let inp = "INSERT INTO Parents(tag_id, relate_tag_id, limit_to) VALUES(?, ?, ?)";
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

    ///
    /// Returns a list of parents where: relate_tag_id
    /// exists
    ///
    pub fn parents_relate_tag_get(&self, relate_tag: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE relate_tag_id = ?")
            .unwrap();
        let temp = stmt
            .query_map(params![relate_tag], |row| {
                let tag_id: usize = row.get(0).unwrap();
                let relate_tag_id: usize = row.get(1).unwrap();
                let limit_to: Option<usize> = row.get(2).unwrap();

                Ok(sharedtypes::DbParentsObj {
                    tag_id,
                    relate_tag_id,
                    limit_to,
                })
            })
            .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of parents where: tag_id
    /// exists
    ///
    pub fn parents_tagid_tag_get(&self, tag_id: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE tag_id = ?")
            .unwrap();
        let temp = stmt
            .query_map(params![tag_id], |row| {
                let tag_id: usize = row.get(0).unwrap();
                let relate_tag_id: usize = row.get(1).unwrap();
                let limit_to: Option<usize> = row.get(2).unwrap();

                Ok(sharedtypes::DbParentsObj {
                    tag_id,
                    relate_tag_id,
                    limit_to,
                })
            })
            .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of relate_tag_ids where: tag_id
    /// exists
    ///
    pub fn parents_tagid_get(&self, relate_tag: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT tag_id FROM Parents WHERE relate_tag_id = ?")
            .unwrap();
        let temp = stmt
            .query_map(params![relate_tag], |row| {
                let tag_id: usize = row.get(0).unwrap();

                Ok(tag_id)
            })
            .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of relate_tag_ids where: tag_id
    /// exists
    ///
    pub fn parents_relatetagid_get(&self, tag_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT relate_tag_id FROM Parents WHERE tag_id = ?")
            .unwrap();
        let temp = stmt
            .query_map(params![tag_id], |row| {
                let relate_tag_id: usize = row.get(0).unwrap();

                Ok(relate_tag_id)
            })
            .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of parents where: limit_to
    /// exists
    ///
    pub fn parents_limitto_tag_get(&self, limitto: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        let mut out = HashSet::new();

        let conn = self._conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE limit_to = ?")
            .unwrap();
        let temp = stmt
            .query_map(params![limitto], |row| {
                let tag_id: usize = row.get(0).unwrap();
                let relate_tag_id: usize = row.get(1).unwrap();
                let limit_to: Option<usize> = row.get(2).unwrap();

                Ok(sharedtypes::DbParentsObj {
                    tag_id,
                    relate_tag_id,
                    limit_to,
                })
            })
            .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
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
    /// Note needs to be offset by one because sqlite starts at 1 but the internal sqlite counter
    /// starts at zero but the stupid actual count starts at 1
    ///
    pub fn tags_max_return_sql(&self) -> usize {
        let conn = self._conn.lock().unwrap();
        conn.query_row("SELECT MAX(id) FROM Tags", params![], |row| row.get(0))
            .unwrap_or(0)
            + 1
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
        let inp = "INSERT INTO Tags (id, name, namespace) VALUES(?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, namespace = EXCLUDED.namespace";
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
        if self.namespace_get_id_sql(name).is_some() {
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
        logging::info_log("Database is Loading: Parents".to_string());
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
    /// Returns the parents
    ///
    pub fn parents_get_id_list_sql(&self, par: &sharedtypes::DbParentsObj) -> HashSet<usize> {
        let mut out = HashSet::new();
        let limit_to = match par.limit_to {
            None => &Null as &dyn ToSql,
            Some(temp) => &temp.clone() as &dyn ToSql,
        };

        {
            let binding = self._conn.clone();
            let temp_test = binding.lock().unwrap();

            let temp = match par.limit_to {
            None => {
                temp_test.prepare("SELECT id FROM Parents WHERE tag_id = ? AND relate_tag_id = ? ")
            }
            Some(_) => temp_test.prepare(
                "SELECT id FROM Parents WHERE tag_id = ? AND relate_tag_id = ? AND limit_to = ?",
            ),
        };

            if let Ok(mut con) = temp {
                match par.limit_to {
                    None => {
                        let parents = con
                            .query_map(
                                [&par.tag_id as &dyn ToSql, &par.relate_tag_id as &dyn ToSql],
                                |row| {
                                    let kep: usize = row.get(0).unwrap();

                                    Ok(kep)
                                },
                            )
                            .unwrap()
                            .flatten();

                        for each in parents {
                            let ear: usize = each;
                            out.insert(ear);
                        }
                    }

                    Some(lim) => {
                        let parents = con
                            .query_map(
                                [
                                    &par.tag_id as &dyn ToSql,
                                    &par.relate_tag_id as &dyn ToSql,
                                    &lim as &dyn ToSql,
                                ],
                                |row| {
                                    let kep: usize = row.get(0).unwrap();

                                    Ok(kep)
                                },
                            )
                            .unwrap()
                            .flatten();
                        for each in parents {
                            let ear: usize = each;
                            out.insert(ear);
                        }
                    }
                }
            };
        }

        out
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
        conn.query_row(
            "SELECT id FROM File WHERE hash = ? LIMIT 1",
            params![hash],
            |row| row.get(0),
        )
        .unwrap_or_default()
    }

    ///
    /// Returns if an extension exists gey by ext string
    ///
    pub fn extension_get_id_sql(&self, ext: &str) -> Option<usize> {
        let conn = self._conn.lock().unwrap();

        conn.query_row(
            "select id from FileExtensions where extension = ?",
            params![ext],
            |row| row.get(0),
        )
        .unwrap_or_default()
    }
    ///
    /// Returns if an extension exists get by id
    ///
    pub fn extension_get_string_sql(&self, id: &usize) -> Option<String> {
        let conn = self._conn.lock().unwrap();

        conn.query_row(
            "select extension from FileExtensions where id = ?",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or_default()
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
            sharedtypes::DbFileStorage::NoExist(_fid) => {
                todo!()
            }
            sharedtypes::DbFileStorage::NoExistUnknown => {
                todo!()
            }
        }

        // Catches issue where a non bare DB would nuke itself
        if self._cache == CacheType::Bare
            && let Some(id) = self.file_get_hash(&hash) {
                return id;
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
        logging::info_log("Database is Loading: Relationships".to_string());
        let binding = self._conn.clone();
        let temp_test = binding.lock().unwrap();
        let temp = temp_test.prepare("SELECT fileid, tagid FROM Relationship");
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
        logging::info_log("Database is Loading: Settings".to_string());
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
        .unwrap_or(None)
    }

    ///
    /// Migrates a relationship's tag id
    ///
    pub fn migrate_relationship_tag_sql(&self, old_tag_id: &usize, new_tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        conn.execute(
            "UPDATE OR REPLACE Relationship SET tagid = ? WHERE tagid = ?",
            params![new_tag_id, old_tag_id],
        )
        .unwrap();
    }
    ///
    /// Migrates a relationship's tag id
    ///
    pub fn migrate_relationship_file_tag_sql(
        &self,
        file_id: &usize,
        old_tag_id: &usize,
        new_tag_id: &usize,
    ) {
        let conn = self._conn.lock().unwrap();
        conn.execute(
            "UPDATE OR REPLACE Relationship SET tagid = ? WHERE tagid = ? AND fileid=?",
            params![new_tag_id, old_tag_id, file_id],
        )
        .unwrap();
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
        logging::info_log("Database is Loading: dead_source_urls".to_string());

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
        logging::info_log("Database is Loading: Tags".to_string());

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
    }

    /// Sets advanced settings for journaling. NOTE Experimental badness
    pub fn db_open(&mut self) {
        let _ = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute("PRAGMA secure_delete = 0", params![]);
        let _ = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute("PRAGMA busy_timeout = 5000", params![]);

        /* let _ = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute("PRAGMA journal_mode = OFF", params![]);
        let _ = self
            ._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute("PRAGMA synchronous = OFF", params![]);
        let _ = self._conn.borrow_mut().lock().unwrap().execute(
            "
PRAGMA temp_store = MEMORY
",
            params![],
        );*/
        let _ = self._conn.borrow_mut().lock().unwrap().execute(
            "
PRAGMA page_size = 8192
",
            params![],
        );
        let _ = self._conn.borrow_mut().lock().unwrap().execute(
            "
PRAGMA cache_size = 2900000
",
            params![],
        );
    }

    /// Removes a job from sql table by id
    pub fn del_from_jobs_table_sql_better(&mut self, id: &usize) {
        let inp = "DELETE FROM Jobs WHERE id = ?";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![id.to_string()])
            .unwrap();
    }

    /// Removes a tag from sql table by name and namespace
    pub fn del_from_tags_by_name_and_namespace(&mut self, name: &String, namespace: &String) {
        let inp = "DELETE FROM Tags WHERE name = ? AND namespace = ?";
        self._conn
            .borrow_mut()
            .lock()
            .unwrap()
            .execute(inp, params![name, namespace])
            .unwrap();
    }

    /// Sqlite wrapper for deleteing a relationship from table.
    pub fn delete_relationship_sql(&mut self, file_id: &usize, tag_id: &usize) {
        logging::log(format!(
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
    pub fn delete_parent_sql(&mut self, tag_id: &usize, relate_tag_id: &usize) {
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
    pub fn delete_tag_sql(&mut self, tag_id: &usize) {
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
        logging::info_log(format!(
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
}

#[cfg(test)]
mod tests {
    use crate::VERS;

    use super::*;

    fn setup_default_db() -> Main {
        let mut db = Main::new(None, VERS);
        let parents = [
            sharedtypes::DbParentsObj {
                tag_id: 1,
                relate_tag_id: 2,
                limit_to: Some(3),
            },
            sharedtypes::DbParentsObj {
                tag_id: 2,
                relate_tag_id: 3,
                limit_to: Some(4),
            },
            sharedtypes::DbParentsObj {
                tag_id: 3,
                relate_tag_id: 4,
                limit_to: Some(5),
            },
        ];
        for parent in parents {
            db.parents_add(parent);
        }
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

    #[test]
    fn tag_retrieve() {
        let mut db = Main::new(None, VERS);
        dbg!(&db._cache);
        db.tag_add(&"te".to_string(), 0, true, None);
        assert!(db.tag_get_name("te".to_string(), 0).is_some());
    }
}
