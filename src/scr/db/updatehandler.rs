use crate::database::Main;
use crate::logging;
use crate::sharedtypes;
use crate::vec_of_strings;
use rusqlite::params;
use std::collections::BTreeMap;
impl Main {
    /// Migrates from version 2 to version 3 SQLITE Only bb
    pub fn db_update_two_to_three(&mut self) {
        dbg!("db update");

        // self.backup_db();
        let jobs_cnt = self.db_table_collumn_getnames(&"Jobs".to_string()).len();
        match jobs_cnt {
            5 => {
                logging::info_log(&format!("Starting work on Jobs."));
                if !self.check_table_exists("Jobs_Old".to_string()) {
                    self.alter_table(&"Jobs".to_string(), &"Jobs_Old".to_string());
                }
                let mut storage = std::collections::HashSet::new();
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt = conn.prepare("SELECT * FROM Jobs_Old").unwrap();
                    let mut rows = stmt.query([]).unwrap();
                    let mut cnt = 0;

                    // Loads the V2 jobs into memory
                    while let Some(row) = rows.next().unwrap() {
                        let time: usize = row.get(0).unwrap();
                        let reptime: usize = row.get(1).unwrap();
                        let site: String = row.get(2).unwrap();
                        let param: String = row.get(3).unwrap();
                        let committype: String = row.get(4).unwrap();
                        storage.insert((
                            cnt.clone(),
                            time,
                            reptime,
                            serde_json::to_string(&sharedtypes::DbJobsManager {
                                jobtype: sharedtypes::DbJobType::Params,
                                recreation: None,
                                // additionaldata: None,
                            })
                            .unwrap(),
                            site,
                            param,
                            committype,
                        ));
                        cnt += 1;
                    }
                }
                let keys = &vec_of_strings!(
                    "id",
                    "time",
                    "reptime",
                    "Manager",
                    "site",
                    "param",
                    "CommitType"
                );
                let vals = &vec_of_strings!(
                    "INTEGER", "INTEGER", "INTEGER", "TEXT", "TEXT", "TEXT", "TEXT"
                );
                self.table_create(&"Jobs".to_string(), keys, vals);

                // Putting blank parenthesis forces rust to drop conn which is locking our
                // reference to self
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt =
                        conn
                            .prepare(
                                "INSERT INTO Jobs (id,time,reptime,Manager,site,param,CommitType) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                            )
                            .unwrap();
                    for item in storage.into_iter() {
                        stmt.execute(item).unwrap();
                    }
                }
                self.db_drop_table(&"Jobs_Old".to_string());
                self.transaction_flush();
            }
            7 => {
                if self.check_table_exists("Jobs".to_string())
                    && !self.check_table_exists("Jobs_Old".to_string())
                {
                    logging::info_log(&format!(
                        "DB-Ugrade: Already processed Jobs table moving on."
                    ));
                } else {
                    logging::panic_log(&format!(
                        "DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                    ))
                }
            }
            _ => {
                logging::error_log(&format!(
                    "Weird table loading. Should be 5 or 7 for db upgrade. Pulled {}",
                    &jobs_cnt
                ));
                logging::panic_log(&format!(
                    "DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                ));
            }
        }
        let tags_cnt = self.db_table_collumn_getnames(&"Tags".to_string()).len();
        match tags_cnt {
            3 => {
                if self.check_table_exists("Tags".to_string())
                    && !self.check_table_exists("Tags_Old".to_string())
                {
                    logging::info_log(&format!(
                        "DB-Ugrade: Already processed Tags table moving on."
                    ));
                } else {
                    logging::panic_log(&format!(
                        "DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                    ))
                }
            }
            4 => {
                logging::info_log(&format!("Starting work on Tags."));
                if !self.check_table_exists("Tags_Old".to_string()) {
                    self.alter_table(&"Tags".to_string(), &"Tags_Old".to_string());
                }
                let mut storage = std::collections::HashSet::new();
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt = conn.prepare("SELECT * FROM Tags_Old").unwrap();
                    let mut rows = stmt.query([]).unwrap();

                    // Loads the V2 jobs into memory
                    while let Some(row) = rows.next().unwrap() {
                        let id: usize = row.get(0).unwrap();
                        let name: String = row.get(1).unwrap();
                        let namespace: usize = row.get(3).unwrap();
                        storage.insert((id, name, namespace));
                    }
                }
                let keys = &vec_of_strings!("id", "name", "namespace");
                let vals = &vec_of_strings!("INTEGER", "TEXT", "INTEGER");
                self.table_create(&"Tags".to_string(), keys, vals);

                // Putting blank parenthesis forces rust to drop conn which is locking our
                // reference to self
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt = conn
                        .prepare("INSERT INTO Tags (id,name,namespace) VALUES (?1,?2,?3)")
                        .unwrap();
                    for item in storage.into_iter() {
                        stmt.execute(item).unwrap();
                    }
                }
                self.db_drop_table(&"Tags_Old".to_string());
                self.transaction_flush();
            }
            _ => {
                logging::error_log(&format!(
                    "Weird tags table loading. Should be 3 or 4 for db upgrade. Was {}",
                    tags_cnt
                ));
                logging::panic_log(&format!(
                    "DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                ));
            }
        }
        let tags_cnt = self.db_table_collumn_getnames(&"Parents".to_string()).len();
        match tags_cnt {
            3 => {
                if self.check_table_exists("Parents".to_string())
                    && !self.check_table_exists("Parents_Old".to_string())
                {
                    logging::info_log(&format!(
                        "DB-Ugrade: Already processed Parents table moving on."
                    ));
                } else {
                    logging::panic_log(&format!(
                        "DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                    ))
                }
            }
            4 => {
                logging::info_log(&format!("Starting work on Parents."));
                if !self.check_table_exists("Parents_Old".to_string()) {
                    self.alter_table(&"Parents".to_string(), &"Parents_Old".to_string());
                }
                let mut storage = std::collections::HashSet::new();
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt = conn.prepare("SELECT * FROM Parents_Old").unwrap();
                    let mut rows = stmt.query([]).unwrap();

                    // Loads the V2 jobs into memory
                    while let Some(row) = rows.next().unwrap() {
                        let tag: usize = row.get(1).unwrap();
                        let relate: usize = row.get(3).unwrap();
                        storage.insert((tag, relate));
                    }
                }
                let keys = &vec_of_strings!("tag_id", "relate_tag_id", "limit_to");
                let vals = &vec_of_strings!("INTEGER", "INTEGER", "INTEGER");
                self.table_create(&"Parents".to_string(), keys, vals);

                // Putting blank parenthesis forces rust to drop conn which is locking our
                // reference to self
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt = conn
                        .prepare("INSERT INTO Parents (tag_id,relate_tag_id) VALUES (?1,?2)")
                        .unwrap();
                    for item in storage.into_iter() {
                        stmt.execute(item).unwrap();
                    }
                }
                self.db_drop_table(&"Parents_Old".to_string());
                self.transaction_flush();
            }
            _ => {
                logging::error_log(&format!(
                    "Weird parents table loading. Should be 3 or 4 for db upgrade. Was {}",
                    tags_cnt
                ));
                logging::panic_log(&format!(
                    "DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                ));
            }
        }
        self.db_version_set(3);
        self.transaction_flush();
    }

    /// Manages the upgrading process from V3 to V4.
    pub fn db_update_three_to_four(&mut self) {
        let jobs_cnt = self.db_table_collumn_getnames(&"Jobs".to_string()).len();
        dbg!(&jobs_cnt);
        let mut storage = std::collections::HashSet::new();
        match jobs_cnt {
            7 => {
                logging::info_log(&format!("Starting work on Jobs."));

                // Renames table to Jobs_Old
                if !self.check_table_exists("Jobs_Old".to_string()) {
                    self.alter_table(&"Jobs".to_string(), &"Jobs_Old".to_string());
                }
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt = conn.prepare("SELECT * FROM Jobs_Old").unwrap();
                    let mut rows = stmt.query([]).unwrap();

                    // Loads the V3 jobs into memory
                    while let Some(row) = rows.next().unwrap() {
                        let id: usize = row.get(0).unwrap();
                        let time: usize = row.get(1).unwrap();
                        let reptime: usize = row.get(2).unwrap();
                        let manager: String = row.get(3).unwrap();
                        let site: String = row.get(4).unwrap();
                        let param: String = row.get(5).unwrap();
                        let committype: String = row.get(6).unwrap();
                        storage.insert((
                            id,
                            time,
                            reptime,
                            manager,
                            site,
                            param,
                            committype,
                            serde_json::to_string(&BTreeMap::<String, String>::new()).unwrap(),
                            serde_json::to_string(&BTreeMap::<String, String>::new()).unwrap(),
                        ));
                    }
                }
                let keys = &vec_of_strings!(
                    "id",
                    "time",
                    "reptime",
                    "Manager",
                    "site",
                    "param",
                    "CommitType",
                    "SystemData",
                    "UserData"
                );
                let vals = &vec_of_strings!(
                    "INTEGER", "INTEGER", "INTEGER", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT"
                );
                self.table_create(&"Jobs".to_string(), keys, vals);
                {
                    let conn = self._conn.lock().unwrap();
                    let mut stmt =
                        conn
                            .prepare(
                                "INSERT INTO Jobs (id,time,reptime,Manager,site,param,CommitType,SystemData,UserData) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                            )
                            .unwrap();
                    for item in storage.into_iter() {
                        stmt.execute(item).unwrap();
                    }
                }
                self.db_drop_table(&"Jobs_Old".to_string());
                self.transaction_flush();
            }
            9 => {
                logging::info_log(&format!("Already processed jobs. Skipping..."));
            }
            _ => {}
        }
    }

    ///
    /// Updates the DB from V4 to V5
    ///
    pub fn db_update_four_to_five(&mut self) {
        logging::info_log(&"Backing up db this could be messy.".to_string());
        self.backup_db();

        // Creates a location to store the location of files
        if !self.check_table_exists("FileStorageLocations".to_string()) {
            let keys = &vec_of_strings!("id", "location");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL");
            self.table_create(&"FileStorageLocations".to_string(), keys, vals);
        }

        // Creates a location to store the location of the extension of a file
        if !self.check_table_exists("FileExtensions".to_string()) {
            let keys = &vec_of_strings!("id", "extension");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL");
            self.table_create(&"FileExtensions".to_string(), keys, vals);
        }

        // Puts Files table into proper state
        match (
            self.check_table_exists("File_Old".to_string()),
            self.check_table_exists("File".to_string()),
        ) {
            (true, true) => {}
            (false, false) => {
                panic!("Cannot figure out where DB is at for file table four to five. Table does not exist.");
            }
            (true, false) => {}
            (false, true) => {
                self.alter_table(&"File".to_string(), &"File_Old".to_string());
                //self.transaction_flush();
            }
        }
        // Creates a file table that is for V5
        let keys = &vec_of_strings!("id", "hash", "extension", "enclave_id");
        let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL", "INTEGER", "INTEGER");
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

        if !self.check_table_exists("EnclaveCondition".to_string()) {
            let keys = &vec_of_strings!("id", "action_name", "action_condition");
            let vals = &vec_of_strings!(
                "INTEGER NOT NULL PRIMARY KEY",
                "TEXT NOT NULL",
                "TEXT NOT NULL"
            );
            self.table_create(&"EnclaveCondition".to_string(), keys, vals);
        }
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
            let keys = &vec_of_strings!("id", "enclave_name");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL");
            self.table_create(&"Enclave".to_string(), keys, vals);
        }

        // Sets up missing enclave location
        self.enclave_create_default_file_download(self.location_get());

        {
            let mut storage = std::collections::HashSet::new();
            dbg!("Needs to transition files");
            let conn = self._conn.lock().unwrap();

            let mut stmt = conn.prepare("SELECT * FROM File_Old").unwrap();
            let mut rows = stmt.query([]).unwrap();

            let file_sqlite_inp = "INSERT INTO File VALUES (?, ?, ?, ?)";

            // Loads the Files into memory
            // Creates storage id's for them
            // Creates extension id's aswell
            let storage_sqlite_inp = "INSERT INTO FileStorageLocations VALUES (?, ?)";
            let extension_sqlite_inp = "INSERT INTO FileExtensions VALUES (?, ?)";

            let mut location_cnt = 0;
            let mut extension_cnt = 0;
            let mut location_storage = std::collections::HashMap::new();
            let mut extension_storage = std::collections::HashMap::new();
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let hash: String = row.get(1).unwrap();
                let extension: String = row.get(2).unwrap();
                let location_string: String = row.get(3).unwrap();

                let storageid = match location_storage.get(&location_string) {
                    None => {
                        let original = location_cnt;
                        location_cnt += 1;
                        let _ =
                            conn.execute(storage_sqlite_inp, params![original, location_string]);
                        location_storage.insert(location_string, original);
                        original
                    }
                    Some(id) => id.clone(),
                };

                let extensionid = match extension_storage.get(&extension) {
                    None => {
                        let original = extension_cnt;
                        extension_cnt += 1;
                        let _ = conn.execute(extension_sqlite_inp, params![original, extension]);
                        extension_storage.insert(extension, original);
                        original
                    }
                    Some(id) => id.clone(),
                };

                storage.insert((id, hash, extensionid, storageid));
            }
            dbg!(&storage);
            dbg!(&location_storage);
            dbg!(&extension_storage);
        }
        self.transaction_flush();
    }
}
