use crate::database::Main;
use crate::download;
use crate::file::folder_make;
use crate::logging;
use crate::sharedtypes;
use crate::vec_of_strings;
use bytes::Bytes;
use chrono::Utc;
use core::panic;
use file_format::FileFormat;
use rusqlite::OptionalExtension;
use rusqlite::params;

const DEFAULT_PRIORITY_DOWNLOAD: usize = 10;
const DEFAULT_PRIORITY_PUT: usize = 5;
const DEFAULT_DOWNLOAD_DEFAULT: &str = "DownloadToDiskDefault";
const DEFAULT_DOWNLOAD_DISK: &str = "DownloadToDisk";
pub const DEFAULT_PUT_DISK: &str = "PutAtDefault";
const DEFAULT_PRIORITY_LOWEST: usize = 0;

impl Main {
    pub fn enclave_run_process(
        &mut self,
        file: &mut sharedtypes::FileObject,
        bytes: &Bytes,
        sha512hash: &String,
        source_url: Option<&String>,
        enclave_name: &str,
    ) -> bool {
        if let Some(enclave_id) = self.enclave_name_get_id(enclave_name) {
            return self.enclave_run_logic(file, bytes, sha512hash, source_url, &enclave_id);
        }
        false
    }

    ///
    /// Actually runs the enclave processing logic
    ///
    fn enclave_run_logic(
        &mut self,
        file: &mut sharedtypes::FileObject,
        bytes: &Bytes,
        sha512hash: &String,
        source_url: Option<&String>,
        enclave_id: &usize,
    ) -> bool {
        for condition_list_id in self
            .enclave_action_order_enclave_get_list_id(enclave_id)
            .iter()
        {
            logging::log(&format!(
                "Enclave FileHash {}: Pulled condition list id for {}, enclave_id: {}",
                &sha512hash, condition_list_id, enclave_id
            ));

            for (enclave_action_id, failed_enclave_action_id, condition_id) in
                self.enclave_condition_list_get(condition_list_id).iter()
            {
                let (action_bool, action_name) =
                    self.enclave_condition_evaluate(condition_id, file, bytes);

                let run_option_action_id = if action_bool {
                    Some(enclave_action_id)
                } else {
                    failed_enclave_action_id.as_ref()
                };

                if let Some(run_action_id) = run_option_action_id {
                    logging::log(&format!(
                        "Enclave FileHash {}: Running action id: {}",
                        &sha512hash, run_action_id
                    ));
                    if let Some(action_name) = action_name {
                        logging::log(&format!(
                            "Enclave FileHash {}: Running action name: {:?}",
                            &sha512hash, action_name
                        ));
                        let source_url_ns_id = self.create_default_source_url_ns_id();
                        if !self.enclave_run_action(
                            &action_name,
                            file,
                            bytes,
                            sha512hash,
                            source_url,
                            source_url_ns_id,
                            run_action_id,
                        ) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    ///
    /// Determines the default enclave(s) to run on a file
    ///
    pub fn enclave_determine_processing(
        &mut self,
        file: &mut sharedtypes::FileObject,
        bytes: Bytes,
        sha512hash: &String,
        source_url: Option<&String>,
    ) {
        logging::info_log(&format!(
            "Enclave FileHash {}: Starting to process",
            &sha512hash
        ));

        'priorityloop: for priority_id in self.enclave_priority_get().iter() {
            for enclave_id in self.enclave_get_id_from_priority(priority_id).iter() {
                if self.enclave_run_logic(file, &bytes, sha512hash, source_url, enclave_id) {
                    break 'priorityloop;
                }
            }
        }
    }

    ///
    /// Runs an action as it's valid
    /// Returns weather we should stop on this action
    ///
    fn enclave_run_action(
        &mut self,
        action: &sharedtypes::EnclaveAction,
        file: &mut sharedtypes::FileObject,
        bytes: &Bytes,
        sha512hash: &String,
        source_url: Option<&String>,
        source_url_ns_id: usize,
        enclave_id: &usize,
    ) -> bool {
        match action {
            sharedtypes::EnclaveAction::PutAtDefault => {
                let download_location = self.location_get();
                logging::log(&format!(
                    "Enclave FileHash {}: {}",
                    &sha512hash,
                    format!("Putting at Default location {}", &download_location)
                ));
                let _ = self.download_and_do_parsing(
                    bytes,
                    sha512hash,
                    source_url,
                    source_url_ns_id,
                    enclave_id,
                    &download_location,
                    file,
                );
                return false;
                //Some(fileid)
            }
            sharedtypes::EnclaveAction::AddTagAndNamespace((
                tag,
                namespace,
                tag_type,
                relates_to,
            )) => {
                file.tag_list.push(sharedtypes::TagObject {
                    namespace: namespace.clone(),
                    tag: tag.clone(),
                    tag_type: tag_type.clone(),
                    relates_to: relates_to.clone(),
                });
                //None
            }
            sharedtypes::EnclaveAction::DownloadToDefault => {
                let download_location = self.location_get();
                logging::log(&format!(
                    "Enclave FileHash {}: {}",
                    &sha512hash,
                    format!("Downloading to Default location {}", &download_location)
                ));
                let _ = self.download_and_do_parsing(
                    bytes,
                    sha512hash,
                    source_url,
                    source_url_ns_id,
                    enclave_id,
                    &download_location,
                    file,
                );
                return false;
                //Some(fileid)
            }
            sharedtypes::EnclaveAction::DownloadToLocation(_) => {} //None,
        }
        true
    }

    ///
    /// Adds info relating to the file and storage. Please add / parse tags sperately
    ///
    fn download_and_do_parsing(
        &mut self,
        bytes: &Bytes,
        sha512hash: &String,
        source_url: Option<&String>,
        source_url_ns_id: usize,
        enclave_id: &usize,
        download_location: &String,
        file: &mut sharedtypes::FileObject,
    ) -> usize {
        //let loc = self.location_get();

        self.storage_put(download_location);

        // error checking. We should have all dirs needed but hey if we're missing
        std::fs::create_dir_all(download_location).unwrap();

        // Gives file extension
        let file_ext = FileFormat::from_bytes(bytes).extension().to_string();

        let ext_id = self.extension_put_string(&file_ext);

        let download_loc = std::path::Path::new(&download_location)
            .canonicalize()
            .unwrap();
        //dbg!(&download_loc, &download_loc.file_name());

        download::write_to_disk(download_loc, bytes, sha512hash);
        //download::write_to_disk(download_loc, file, bytes, sha512hash);

        let storage_id = self.storage_get_id(download_location).unwrap();

        let filestorage = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
            hash: sha512hash.to_string(),
            ext_id,
            storage_id,
        });
        let fileid = self.file_add(filestorage);
        if let Some(source_url) = source_url {
            let tagid = self.tag_add(source_url, source_url_ns_id, true, None);
            self.relationship_add(fileid, tagid, true);
        }

        self.enclave_file_mapping_add(&fileid, enclave_id);
        fileid
    }

    ///
    /// Checks if a condition is true
    ///
    fn enclave_condition_evaluate(
        &self,
        condition_id: &usize,
        file: &sharedtypes::FileObject,
        bytes: &Bytes,
    ) -> (bool, Option<sharedtypes::EnclaveAction>) {
        if let Some((action, condition)) = self.enclave_condition_get_data(condition_id) {
            (
                match condition {
                    sharedtypes::EnclaveCondition::Any => true,
                    sharedtypes::EnclaveCondition::None => false,
                    sharedtypes::EnclaveCondition::FileSizeGreater(byte_len) => {
                        byte_len < bytes.len()
                    }
                    sharedtypes::EnclaveCondition::FileSizeLessthan(byte_len) => {
                        byte_len > bytes.len()
                    }
                    sharedtypes::EnclaveCondition::TagNameAndNamespace((tag_name, namespace)) => {
                        let mut out = false;
                        for tag in file.tag_list.iter() {
                            if tag.tag.contains(&tag_name)
                                && tag.namespace.name.contains(&namespace)
                            {
                                out = true;
                                break;
                            }
                        }
                        out
                    }
                },
                Some(action),
            )
        } else {
            (false, None)
        }
    }

    ///
    /// Adds a filemapping if it doesn't exist
    ///
    fn enclave_file_mapping_add(&mut self, file_id: &usize, enclave_id: &usize) {
        let utc = Utc::now();
        let timestamp = utc.timestamp_millis();

        if self.enclave_file_mapping_get(file_id, enclave_id).is_some() {
            let conn = self._conn.lock().unwrap();
            let mut prep = conn
            .prepare(
                "UPDATE FileEnclaveMapping SET timestamp = ? WHERE file_id = ? AND enclave_id = ?",
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
    fn enclave_file_mapping_get(&self, file_id: &usize, enclave_id: &usize) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT file_id from FileEnclaveMapping where file_id = ? AND enclave_id = ? LIMIT 1",
            params![file_id, enclave_id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
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

    pub fn enclave_create_default_file_import(&mut self) {
        self.enclave_action_put(
            &DEFAULT_PUT_DISK.into(),
            sharedtypes::EnclaveAction::PutAtDefault,
        );
        self.enclave_condition_put(
            &sharedtypes::EnclaveAction::PutAtDefault,
            &sharedtypes::EnclaveCondition::Any,
        );
        self.enclave_name_put(DEFAULT_PUT_DISK.into(), &DEFAULT_PRIORITY_PUT);

        let enclave_id = self.enclave_name_get_id(DEFAULT_PUT_DISK).unwrap();
        let condition_id = self
            .enclave_condition_get_id(&sharedtypes::EnclaveAction::PutAtDefault)
            .unwrap();
        let action_id = self
            .enclave_action_get_id(&DEFAULT_PUT_DISK.into())
            .unwrap();
        self.enclave_condition_link_put(&condition_id, &action_id, None);
        let condition_link_id = self
            .enclave_condition_link_get_id(&condition_id, &action_id)
            .unwrap();

        self.enclave_action_order_link_put(&enclave_id, &condition_link_id, &0);
    }

    ///
    /// Creates a enclave that is for downloading files
    /// Default behaviour is to download to the specified folder unless the input is false
    ///
    pub fn enclave_create_default_file_download(&mut self, location: String) {
        // Makes the default file download location
        {
            folder_make(&location);
        }

        let is_default_location = location == self.location_get();

        let default_or_alternative_location = if is_default_location {
            DEFAULT_DOWNLOAD_DEFAULT
        } else {
            DEFAULT_DOWNLOAD_DISK
        };

        let default_or_alternative_priority = if is_default_location {
            DEFAULT_PRIORITY_DOWNLOAD
        } else {
            DEFAULT_PRIORITY_LOWEST
        };

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
    fn enclave_name_put(&mut self, name: String, enclave_priority: &usize) {
        if self.enclave_name_get_id(&name).is_some() {
            return;
        }

        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO Enclave (enclave_name, priority) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .unwrap();
        let _ = prep.insert(params![name, enclave_priority]);
    }
    //  pub fn enclave_name_edit(&mut self, id: &usize, name: String) {}

    ///
    /// Gets the enclave name from the enclave id
    ///
    fn enclave_name_get_name(&self, id: &usize) -> Option<String> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT enclave_name from Enclave where id = ?",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Gets the enclave id from the enclave name
    ///
    pub fn enclave_name_get_id(&self, name: &str) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id from Enclave where enclave_name = ?",
            params![name],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Gets prioritys from Enclaves
    ///
    fn enclave_get_id_from_priority(&self, priority_id: &usize) -> Vec<usize> {
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
        for ids in row.flatten() {
            if !out.contains(&ids) {
                out.push(ids);
            }
        }

        out.sort();

        out
    }

    ///
    /// Gets prioritys from Enclaves
    ///
    fn enclave_priority_get(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let conn = self._conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT priority from Enclave ORDER BY priority DESC")
            .unwrap();
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
        for ids in row.flatten() {
            if !out.contains(&ids) {
                out.push(ids);
            }
        }

        out
    }

    ///
    /// Adds a condition to the database
    ///
    fn enclave_condition_put(
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
        conn.query_row(
            "SELECT id from EnclaveCondition where action_name = ?",
            params![serde_json::to_string(name).unwrap()],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Gets conditional action and condition from id
    ///
    fn enclave_condition_get_data(
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
    fn enclave_action_order_link_put(
        &mut self,
        enclave_id: &usize,
        enclave_conditional_list_id: &usize,
        enclave_action_position: &usize,
    ) {
        if self
            .enclave_action_order_link_get_id(enclave_id, enclave_conditional_list_id)
            .is_some()
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
    fn enclave_action_order_link_get_id(
        &self,
        enclave_id: &usize,
        enclave_conditional_list_id: &usize,
    ) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn
            .query_row(
                "SELECT id from EnclaveActionOrderList where enclave_id = ? AND enclave_conditional_list_id = ?",
                params![enclave_id, enclave_conditional_list_id],
                |row| row.get(0),
            )
            .optional().unwrap_or_default()
    }

    ///
    /// Gets enclave conditional jump by enclave_id
    ///
    fn enclave_action_order_enclave_get_list_id(&self, enclave_id: &usize) -> Vec<usize> {
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
        for ids in row.flatten() {
            if !out.contains(&ids) {
                out.push(ids);
            }
        }

        out.sort();

        out
    }

    ///
    /// Adds the conditional link between for a condition and it's priority in the enclave list
    ///
    fn enclave_condition_link_put(
        &mut self,
        condition_id: &usize,
        action_id: &usize,
        failed_enclave_id: Option<&usize>,
    ) {
        if self
            .enclave_condition_link_get_id(condition_id, action_id)
            .is_some()
        {
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
    fn enclave_condition_link_get_id(
        &self,
        condition_id: &usize,
        action_id: &usize,
    ) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id from EnclaveConditionList where enclave_action_id = ? AND condition_id = ?",
            params![action_id, condition_id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Gets conditional jumps for conditonlist id
    ///
    fn enclave_condition_list_get(
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
    /// Inserts the name and action into the database
    ///
    fn enclave_action_put(&mut self, name: &String, action: sharedtypes::EnclaveAction) {
        // Quick out if we already have this inserted
        if self.enclave_action_get_id(name).is_some() {
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
    fn enclave_action_get_id(&self, name: &String) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        conn.query_row(
            "SELECT id from EnclaveAction where action_name = ?",
            params![name],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or_default()
    }
}
