use crate::database::Main;
use crate::error;
use crate::logging;
use crate::sharedtypes;
use rusqlite::params;
use rusqlite::OptionalExtension;
use std::borrow::BorrowMut;
impl Main {
    pub fn parents_delete_sql(&mut self, id: &usize) {
        self.parents_delete_tag_id_sql(id);
        self.parents_delete_relate_tag_id_sql(id);
        self.parents_delete_limit_to_sql(id);
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
    /// Adds file via SQL
    pub(super) fn file_add_sql(
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

    /// Loads Relationships in from DB connection
    pub(super) fn load_relationships(&mut self) {
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

    /// Loads tags into db
    pub(super) fn load_tags(&mut self) {
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
