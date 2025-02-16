use crate::database::Main;
use crate::download;
use crate::logging;
use crate::sharedtypes;
use crate::vec_of_strings;
use bytes::Bytes;
use chrono::DateTime;
use chrono::{NaiveDate, Utc};
use core::panic;
use file_format::FileFormat;
use rusqlite::params;
use rusqlite::types::FromSql;
use rusqlite::OptionalExtension;
impl Main {
    pub fn enclave_determine_processing(
        &mut self,
        file: &sharedtypes::FileObject,
        bytes: Bytes,
        sha512hash: &String,
        source_url: &String,
    ) {
        logging::info_log(&format!("Starting to process: {}", file.hash));

        for priority_id in self.enclave_priority_get().iter() {
            for enclave_id in self.enclave_get_id_from_priority(priority_id).iter() {
                for condition_list_id in self
                    .enclave_action_order_enclave_get_list_id(enclave_id)
                    .iter()
                {
                    logging::info_log(&format!(
                        "Pulled condition list id for {}, enclave_id: {}",
                        condition_list_id, enclave_id
                    ));

                    for (enclave_action_id, failed_enclave_action_id, condition_id) in
                        self.enclave_condition_list_get(condition_list_id).iter()
                    {
                        dbg!(enclave_action_id, failed_enclave_action_id, condition_id);

                        let (action_bool, action_name) =
                            self.enclave_condition_evaluate(condition_id, file, &bytes);

                        let run_option_action_id = if action_bool {
                            Some(enclave_action_id)
                        } else {
                            failed_enclave_action_id.as_ref()
                        };

                        if let Some(run_action_id) = run_option_action_id {
                            logging::info_log(&format!("Running action id: {}", run_action_id));
                            if let Some(action_name) = action_name {
                                logging::info_log(&format!(
                                    "Running action name: {:?}",
                                    action_name
                                ));
                                let source_url_ns_id = self.create_default_source_url_ns_id();
                                self.enclave_run_action(
                                    &action_name,
                                    &file,
                                    &bytes,
                                    sha512hash,
                                    source_url,
                                    source_url_ns_id,
                                    run_action_id,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    ///
    /// Runs an action as it's valid
    ///
    pub fn enclave_run_action(
        &mut self,
        action: &sharedtypes::EnclaveAction,
        file: &sharedtypes::FileObject,
        bytes: &Bytes,
        sha512hash: &String,
        source_url: &String,
        source_url_ns_id: usize,
        enclave_id: &usize,
    ) -> Option<usize> {
        match action {
            sharedtypes::EnclaveAction::DownloadToDefault => {
                let loc = self.location_get();

                self.storage_put(&loc);

                // Gives file extension
                let file_ext = FileFormat::from_bytes(&bytes).extension().to_string();

                let ext_id = self.extension_put_string(&file_ext);

                let download_loc = std::path::Path::new(&loc).canonicalize().unwrap();
                dbg!(&download_loc, &download_loc.file_name());

                download::write_to_disk(download_loc, file, bytes, sha512hash);

                let storage_id = self.storage_get_id(&loc).unwrap();

                let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                    hash: sha512hash.to_string(),
                    ext_id,
                    storage_id,
                });
                let fileid = self.file_add(file, true);
                let tagid = self.tag_add(source_url, source_url_ns_id, true, None);
                self.relationship_add(fileid, tagid, true);
                self.enclave_file_mapping_add(&fileid, enclave_id);
                Some(fileid.clone())
            }
            sharedtypes::EnclaveAction::DownloadToLocation(location_id) => None,
        }
    }

    ///
    /// Checks if a condition is true
    ///
    pub fn enclave_condition_evaluate(
        &self,
        condition_id: &usize,
        file: &sharedtypes::FileObject,
        bytes: &Bytes,
    ) -> (bool, Option<sharedtypes::EnclaveAction>) {
        if let Some((action, condition)) = self.enclave_condition_get_data(condition_id) {
            return (
                match condition {
                    sharedtypes::EnclaveCondition::Any => true,
                    sharedtypes::EnclaveCondition::None => false,
                    sharedtypes::EnclaveCondition::FileSizeGreater(byte_len) => {
                        byte_len < bytes.len()
                    }
                    sharedtypes::EnclaveCondition::FileSizeLessthan(byte_len) => {
                        byte_len > bytes.len()
                    }
                },
                Some(action),
            );
        } else {
            (false, None)
        }
    }

    ///
    /// Adds a filemapping if it doesn't exist
    ///
    pub fn enclave_file_mapping_add(&mut self, file_id: &usize, enclave_id: &usize) {
        let utc = Utc::now();
        let timestamp = utc.timestamp_millis();

        if self.enclave_file_mapping_get(file_id, enclave_id).is_some() {
            let conn = self._conn.lock().unwrap();
            let mut prep = conn
            .prepare(
                "UPDATE FileEnclaveMapping SET timestamp = ? WHERE fileid = ? AND enclave_id = ?)",
            )
            .unwrap();

            let _ = prep.insert(params![timestamp, file_id, enclave_id]);
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare(
                "INSERT INTO FileEnclaveMapping (file_id, enclave_id, timestamp) VALUES (?, ?, ?)",
            )
            .unwrap();
        let _ = prep.insert(params![file_id, enclave_id, timestamp]);
    }

    ///
    /// Checks if a file mapping pair exists
    ///
    pub fn enclave_file_mapping_get(&self, file_id: &usize, enclave_id: &usize) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT file_id from FileEnclaveMapping where file_id = ? AND enclave_id = ?",
                params![file_id, enclave_id],
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
    /// Creates tje database tables for a V5 upgrade
    ///
    pub fn enclave_create_database_v5(&mut self) {
        // Creates a location to store the location of files
        if !self.check_table_exists("FileStorageLocations".to_string()) {
            let keys = &vec_of_strings!("id", "location");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL");
            self.table_create(&"FileStorageLocations".to_string(), keys, vals);
        }

        // Creates a location to store the location of the extension of a file
        if !self.check_table_exists("FileExtensions".to_string()) {
            let keys = &vec_of_strings!("id", "extension");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL UNIQUE");
            self.table_create(&"FileExtensions".to_string(), keys, vals);
        }

        // Creates a file table that is for V5
        let keys = &vec_of_strings!("id", "hash", "extension", "storage_id");
        let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT", "INTEGER", "INTEGER");
        self.table_create(&"File".to_string(), keys, vals);

        if !self.check_table_exists("EnclaveAction".to_string()) {
            let keys = &vec_of_strings!("id", "action_name", "action_text");
            let vals = &vec_of_strings!(
                "INTEGER PRIMARY KEY NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL"
            );
            self.table_create(&"EnclaveAction".to_string(), keys, vals);
        }

        // Maps enclave id's to file ids
        if !self.check_table_exists("FileEnclaveMapping".to_string()) {
            let keys = &vec_of_strings!("file_id", "enclave_id", "timestamp");
            let vals = &vec_of_strings!("INTEGER NOT NULL", "INTEGER NOT NULL", "INTEGER NOT NULL");
            self.table_create(&"FileEnclaveMapping".to_string(), keys, vals);
        }

        // Enclave condition
        if !self.check_table_exists("EnclaveCondition".to_string()) {
            let keys = &vec_of_strings!("id", "action_name", "action_condition");
            let vals = &vec_of_strings!(
                "INTEGER NOT NULL PRIMARY KEY",
                "TEXT NOT NULL",
                "TEXT NOT NULL"
            );
            self.table_create(&"EnclaveCondition".to_string(), keys, vals);
        }

        // Lists of conditions if X do Y
        if !self.check_table_exists("EnclaveConditionList".to_string()) {
            let keys = &vec_of_strings!(
                "id",
                "enclave_action_id",
                "failed_enclave_action_id",
                "condition_id"
            );
            let vals = &vec_of_strings!(
                "INTEGER NOT NULL PRIMARY KEY",
                "INTEGER NOT NULL",
                "INTEGER",
                "INTEGER NOT NULL"
            );
            self.table_create(&"EnclaveConditionList".to_string(), keys, vals);
        }

        // Intermedidate table
        if !self.check_table_exists("EnclaveActionOrderList".to_string()) {
            let keys = &vec_of_strings!(
                "id",
                "enclave_id",
                "enclave_conditional_list_id",
                "enclave_action_position"
            );
            let vals = &vec_of_strings!(
                "INTEGER PRIMARY KEY NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL"
            );
            self.table_create(&"EnclaveActionOrderList".to_string(), keys, vals);
        }

        if !self.check_table_exists("Enclave".to_string()) {
            let keys = &vec_of_strings!("id", "enclave_name", "priority");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL", "INTEGER NOT NULL");
            self.table_create(&"Enclave".to_string(), keys, vals);
        }
    }

    ///
    /// Creates a enclave that is for downloading files
    /// Default behaviour is to download to the specified folder unless the input is false
    ///
    pub fn enclave_create_default_file_download(&mut self, location: String) {
        let is_default_location = location == self.location_get();

        let default_or_alternative_location = if is_default_location {
            "DownloadToDiskDefault"
        } else {
            "DownloadToDisk"
        };

        let default_or_alternative_priority = if is_default_location { 5 } else { 0 };

        // By default we should download a file to the default file location
        // However if we have an old file location then we shouldn't download to it.
        let alt_condition = if is_default_location {
            sharedtypes::EnclaveCondition::Any
        } else {
            sharedtypes::EnclaveCondition::None
        };

        let default_or_alternative_name = if is_default_location {
            sharedtypes::EnclaveAction::DownloadToDefault
        } else {
            sharedtypes::EnclaveAction::DownloadToLocation(default_or_alternative_priority)
        };

        let alt_action = if is_default_location {
            sharedtypes::EnclaveAction::DownloadToDefault
        } else {
            let storage_id = self.storage_get_id(&location);
            if let Some(storage_id) = storage_id {
                sharedtypes::EnclaveAction::DownloadToLocation(storage_id)
            } else {
                panic!("Cannot find storage location {}", &location);
            }
        };

        let default_file_enclave = format!("File_Download_location_{}", location);
        self.enclave_name_put(
            default_file_enclave.clone(),
            &default_or_alternative_priority,
        );
        self.enclave_condition_put(&default_or_alternative_name, &alt_condition);
        self.enclave_action_put(&default_or_alternative_location.to_string(), alt_action);

        let enclave_id = self.enclave_name_get_id(&default_file_enclave).unwrap();
        let condition_id = self
            .enclave_condition_get_id(&default_or_alternative_name)
            .unwrap();
        let action_id = self
            .enclave_action_get_id(&default_or_alternative_location.to_string())
            .unwrap();
        self.enclave_condition_link_put(&condition_id, &action_id, None);
        let condition_link_id = self
            .enclave_condition_link_get_id(&condition_id, &action_id)
            .unwrap();

        self.enclave_action_order_link_put(&enclave_id, &condition_link_id, &0);
    }

    ///
    /// Inserts the enclave's name into the database
    /// Kinda a dumb way but hey it works
    ///
    pub fn enclave_name_put(&mut self, name: String, enclave_priority: &usize) {
        if self.enclave_name_get_id(&name).is_some() {
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO Enclave (enclave_name, priority) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .unwrap();
        let _ = prep.insert(params![name, enclave_priority]);
    }
    pub fn enclave_name_edit(&mut self, id: &usize, name: String) {}

    ///
    /// Gets the enclave name from the enclave id
    ///
    pub fn enclave_name_get_name(&self, id: &usize) -> Option<String> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT enclave_name from Enclave where id = ?",
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
    /// Gets the enclave id from the enclave name
    ///
    pub fn enclave_name_get_id(&self, name: &str) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from Enclave where enclave_name = ?",
                params![name],
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
    /// Gets prioritys from Enclaves
    ///
    pub fn enclave_get_id_from_priority(&self, priority_id: &usize) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let conn = self._conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id from Enclave WHERE priority = ?")
            .unwrap();
        let row = stmt
            .query_map(params![priority_id], |row| {
                let id: Option<usize> = row.get(0).unwrap();
                if let Some(id) = id {
                    Ok(id)
                } else {
                    Err(rusqlite::Error::InvalidQuery)
                }
            })
            .unwrap();
        for ids in row {
            if let Ok(ids) = ids {
                if !out.contains(&ids) {
                    out.push(ids);
                }
            }
        }

        out.sort();

        out
    }

    ///
    /// Gets prioritys from Enclaves
    ///
    pub fn enclave_priority_get(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let conn = self._conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT priority from Enclave").unwrap();
        let row = stmt
            .query_map([], |row| {
                let id: Option<usize> = row.get(0).unwrap();
                if let Some(id) = id {
                    Ok(id)
                } else {
                    Err(rusqlite::Error::InvalidQuery)
                }
            })
            .unwrap();
        for ids in row {
            if let Ok(ids) = ids {
                if !out.contains(&ids) {
                    out.push(ids);
                }
            }
        }

        out.sort();

        out
    }

    ///
    /// Adds a condition to the database
    ///
    pub fn enclave_condition_put(
        &mut self,
        action: &sharedtypes::EnclaveAction,
        condition: &sharedtypes::EnclaveCondition,
    ) {
        if self.enclave_condition_get_id(action).is_some() {
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO EnclaveCondition (action_name, action_condition) VALUES (?, ?)")
            .unwrap();
        let _ = prep.insert(params![
            serde_json::to_string(action).unwrap(),
            serde_json::to_string(condition).unwrap()
        ]);
    }
    pub fn enclave_condition_get_id(&self, name: &sharedtypes::EnclaveAction) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from EnclaveCondition where action_name = ?",
                params![serde_json::to_string(name).unwrap()],
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
    /// Gets conditional action and condition from id
    ///
    pub fn enclave_condition_get_data(
        &self,
        condition_id: &usize,
    ) -> Option<(sharedtypes::EnclaveAction, sharedtypes::EnclaveCondition)> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT action_name, action_condition from EnclaveCondition where id = ?",
                params![condition_id],
                |row| {
                    let action: String = row.get(0).unwrap();
                    let condition: String = row.get(1).unwrap();
                    Ok(Some((
                        serde_json::from_str(&action).unwrap(),
                        serde_json::from_str(&condition).unwrap(),
                    )))
                },
            )
            .optional()
        {
            out?
        } else {
            None
        }
    }

    ///
    /// Adds a conditional action to the list of enclave actions
    ///
    pub fn enclave_action_order_link_put(
        &mut self,
        enclave_id: &usize,
        enclave_conditional_list_id: &usize,
        enclave_action_position: &usize,
    ) {
        if let Some(_) =
            self.enclave_action_order_link_get_id(enclave_id, enclave_conditional_list_id)
        {
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO EnclaveActionOrderList (enclave_id,enclave_conditional_list_id, enclave_action_position) VALUES (? ,? ,?)")
            .unwrap();
        let _ = prep.insert(params![
            enclave_id,
            enclave_conditional_list_id,
            enclave_action_position
        ]);
    }
    ///
    /// Gives id from conditional linked list
    ///
    pub fn enclave_action_order_link_get_id(
        &self,
        enclave_id: &usize,
        enclave_conditional_list_id: &usize,
    ) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from EnclaveActionOrderList where enclave_id = ? AND enclave_conditional_list_id = ?",
                params![enclave_id, enclave_conditional_list_id],
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
    /// Gets enclave conditional jump by enclave_id
    ///
    pub fn enclave_action_order_enclave_get_list_id(&self, enclave_id: &usize) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let conn = self._conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT enclave_conditional_list_id from EnclaveActionOrderList where enclave_id = ? ORDER BY enclave_action_position DESC")
            .unwrap();
        let row = stmt
            .query_map([enclave_id], |row| {
                let id: Option<usize> = row.get(0).unwrap();
                if let Some(id) = id {
                    Ok(id)
                } else {
                    Err(rusqlite::Error::InvalidQuery)
                }
            })
            .unwrap();
        for ids in row {
            if let Ok(ids) = ids {
                if !out.contains(&ids) {
                    out.push(ids);
                }
            }
        }

        out.sort();

        out
    }

    ///
    /// Gets enclave action order IDs based on file enclave id
    ///

    ///
    /// Adds the conditional link between for a condition and it's priority in the enclave list
    ///
    pub fn enclave_condition_link_put(
        &mut self,
        condition_id: &usize,
        action_id: &usize,
        failed_enclave_id: Option<&usize>,
    ) {
        if let Some(_) = self.enclave_condition_link_get_id(condition_id, action_id) {
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO EnclaveConditionList (enclave_action_id,failed_enclave_action_id,condition_id) VALUES (? ,? ,?)")
            .unwrap();
        let _ = prep.insert(params![action_id, failed_enclave_id, condition_id,]);
    }

    ///
    /// Gives id from conditional linked list
    ///
    pub fn enclave_condition_link_get_id(
        &self,
        condition_id: &usize,
        action_id: &usize,
    ) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from EnclaveConditionList where enclave_action_id = ? AND condition_id = ?",
                params![action_id, condition_id],
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
    /// Gets conditional jumps for conditonlist id
    ///
    pub fn enclave_condition_list_get(
        &mut self,
        condition_list_id: &usize,
    ) -> Option<(usize, Option<usize>, usize)> {
        let conn = self._conn.lock().unwrap();

        if let Ok(out) = conn
            .query_row(
                "SELECT enclave_action_id, failed_enclave_action_id, condition_id from EnclaveConditionList where id = ?",
                params![condition_list_id],
                |row| Ok(Some((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap()))),
            )
            .optional()
        {
            out?
        } else {
            None
        }
    }

    ///
    /// Gets condition ID
    ///

    ///
    /// Inserts the name and action into the database
    ///
    pub fn enclave_action_put(&mut self, name: &String, action: sharedtypes::EnclaveAction) {
        // Quick out if we already have this inserted
        if let Some(_) = self.enclave_action_get_id(name) {
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare(
                "INSERT OR REPLACE INTO EnclaveAction (action_name, action_text) VALUES (?, ?)",
            )
            .unwrap();
        let _ = prep.insert(params![name, serde_json::to_string(&action).unwrap()]);
    }

    ///
    /// Gets an action's ID based on name
    ///
    pub fn enclave_action_get_id(&self, name: &String) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from EnclaveAction where action_name = ?",
                params![name],
                |row| row.get(0),
            )
            .optional()
        {
            out
        } else {
            None
        }
    }
}
