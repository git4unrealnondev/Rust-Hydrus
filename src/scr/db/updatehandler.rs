use crate::database::Main;
use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::{DEFAULT_CACHECHECK, DEFAULT_CACHETIME, DEFAULT_PRIORITY};
use crate::vec_of_strings;
use chrono::Utc;
use eta::{Eta, TimeAcc};
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
                logging::info_log(&"Starting work on Jobs.".to_string());
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
                            cnt,
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
                    logging::info_log(
                        &"DB-Ugrade: Already processed Jobs table moving on.".to_string(),
                    );
                } else {
                    logging::panic_log(
                        &"DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                            .to_string(),
                    )
                }
            }
            _ => {
                logging::error_log(&format!(
                    "Weird table loading. Should be 5 or 7 for db upgrade. Pulled {}",
                    &jobs_cnt
                ));
                logging::panic_log(
                    &"DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                        .to_string(),
                );
            }
        }
        let tags_cnt = self.db_table_collumn_getnames(&"Tags".to_string()).len();
        match tags_cnt {
            3 => {
                if self.check_table_exists("Tags".to_string())
                    && !self.check_table_exists("Tags_Old".to_string())
                {
                    logging::info_log(
                        &"DB-Ugrade: Already processed Tags table moving on.".to_string(),
                    );
                } else {
                    logging::panic_log(
                        &"DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                            .to_string(),
                    )
                }
            }
            4 => {
                logging::info_log(&"Starting work on Tags.".to_string());
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
                logging::panic_log(
                    &"DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                        .to_string(),
                );
            }
        }
        let tags_cnt = self.db_table_collumn_getnames(&"Parents".to_string()).len();
        match tags_cnt {
            3 => {
                if self.check_table_exists("Parents".to_string())
                    && !self.check_table_exists("Parents_Old".to_string())
                {
                    logging::info_log(
                        &"DB-Ugrade: Already processed Parents table moving on.".to_string(),
                    );
                } else {
                    logging::panic_log(
                        &"DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                            .to_string(),
                    )
                }
            }
            4 => {
                logging::info_log(&"Starting work on Parents.".to_string());
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
                logging::panic_log(
                    &"DB IS IN WEIRD INCONSISTEINT STATE PLEASE ROLLBACK TO LATEST BACKUP."
                        .to_string(),
                );
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
                logging::info_log(&"Starting work on Jobs.".to_string());

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
            }
            9 => {
                logging::info_log(&"Already processed jobs. Skipping...".to_string());
            }
            _ => {}
        }
        self.db_version_set(4);
        self.transaction_flush();
    }

    ///
    /// Updates the DB from V4 to V5
    ///
    pub fn db_update_four_to_five(&mut self) {
        logging::info_log(&"Backing up db this could be messy.".to_string());
        self.backup_db();

        // Puts Files table into proper state
        match (
            self.check_table_exists("File_Old".to_string()),
            self.check_table_exists("File".to_string()),
        ) {
            (true, true) => {}
            (false, false) => {
                panic!(
                    "Cannot figure out where DB is at for file table four to five. Table does not exist."
                );
            }
            (true, false) => {}
            (false, true) => {
                self.alter_table(&"File".to_string(), &"File_Old".to_string());
                //self.transaction_flush();
            }
        }

        // Puts Tags table into proper state
        match (
            self.check_table_exists("Tags_Old".to_string()),
            self.check_table_exists("Tags".to_string()),
        ) {
            (true, true) => {}
            (false, false) => {
                panic!(
                    "Cannot figure out where DB is at for file table four to five. Table does not exist."
                );
            }
            (true, false) => {}
            (false, true) => {
                self.alter_table(&"Tags".to_string(), &"Tags_Old".to_string());
                //self.transaction_flush();
            }
        }

        // Puts Jobs table into proper state
        match (
            self.check_table_exists("Jobs_Old".to_string()),
            self.check_table_exists("Jobs".to_string()),
        ) {
            (true, true) => {}
            (false, false) => {
                panic!(
                    "Cannot figure out where DB is at for file table four to five. Table does not exist."
                );
            }
            (true, false) => {}
            (false, true) => {
                self.alter_table(&"Jobs".to_string(), &"Jobs_Old".to_string());
                //self.transaction_flush();
            }
        }

        logging::info_log(&"Creating tables inside of DB for V5 upgrade".to_string());
        self.enclave_create_database_v5();

        // Creating storage location in db
        let mut location_storage = std::collections::HashMap::new();
        {
            let conn = self._conn.lock().unwrap();

            let mut stmt = conn.prepare("SELECT location FROM File_Old").unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let location_string: String = row.get(0).unwrap();

                if location_storage.get(&location_string).is_none() {
                    dbg!("Adding {}", &location_string);
                    location_storage.insert(location_string, None);
                }
            }
        }

        // Adds storage locations into db
        for (location, id) in location_storage.iter_mut() {
            self.storage_put(location);
            *id = self.storage_get_id(location);
        }

        // Sets up missing enclave location
        self.enclave_create_default_file_download(self.location_get());

        // Super dirty way of getting around the borrow checker
        let mut location_to_enclave = std::collections::HashMap::new();
        {
            for (location_string, storage_id) in location_storage.iter() {
                self.enclave_create_default_file_download(location_string.clone());

                let file_enclave = format!("File_Download_location_{}", location_string);
                location_to_enclave
                    .insert(storage_id.unwrap(), self.enclave_name_get_id(&file_enclave));
            }
        }

        {
            let conn = self._conn.lock().unwrap();

            // Loads the Files into memory
            // Creates storage id's for them
            // Creates extension id's aswell
            //
            logging::info_log(&"Starting to process files for DB V5 Upgrade".to_string());
            let extension_sqlite_inp = "INSERT INTO FileExtensions VALUES (?, ?)";
            let file_sqlite_inp = "INSERT INTO File VALUES (?, ?, ?, ?)";
            let file_enclave_sqlite_inp = "INSERT INTO FileEnclaveMapping VALUES (?, ?, ?)";

            let mut extension_cnt = 0;
            let mut extension_storage = std::collections::HashMap::new();
            let mut stmt = conn.prepare("SELECT * FROM File_Old").unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let hash: String = row.get(1).unwrap();
                let extension: String = row.get(2).unwrap();
                let location_string: String = row.get(3).unwrap();

                let storage_id = location_storage.get(&location_string).unwrap().unwrap();

                let extension_id = match extension_storage.get(&extension) {
                    None => {
                        let original = extension_cnt;
                        extension_cnt += 1;
                        let _ = conn.execute(extension_sqlite_inp, params![original, extension]);
                        extension_storage.insert(extension, original);
                        original
                    }
                    Some(id) => *id,
                };
                logging::info_log(&format!("Adding File: {}", &hash));
                let utc = Utc::now();
                let timestamp = utc.timestamp_millis();
                let _ = conn.execute(
                    file_enclave_sqlite_inp,
                    params![id, location_to_enclave.get(&storage_id).unwrap(), timestamp],
                );
                let _ = conn.execute(file_sqlite_inp, params![id, hash, extension_id, storage_id]);
            }
        }
        self.db_drop_table(&"File_Old".to_string());

        if !self.check_table_exists("Tags".to_string()) {
            let keys = &vec_of_strings!("id", "name", "namespace");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL", "INTEGER NOT NULL");
            self.table_create(&"Tags".to_string(), keys, vals);
        }

        {
            let conn = self._conn.lock().unwrap();

            // Worst that can happen is that we cant pull numbers of items and we default to 1
            let mut row_count: usize = 1;
            {
                let sqlite_count = "select count(*) from Tags_Old";
                let mut stmt = conn.prepare(sqlite_count).unwrap();
                let mut rows = stmt.query([]).unwrap();
                while let Some(row) = rows.next().unwrap() {
                    row_count = row.get(0).unwrap();
                }
            }

            let count_five_percent = row_count.div_ceil(100);

            // Loads the Files into memory
            // Creates storage id's for them
            // Creates extension id's aswell
            //
            logging::info_log(&"Starting to process Tags for DB V5 Upgrade".to_string());

            let tag_sqlite_inp = "INSERT INTO Tags VALUES (?, ?, ?)";
            let mut stmt = conn.prepare("SELECT * FROM Tags_Old").unwrap();
            let mut rows = stmt.query([]).unwrap();

            let mut eta = Eta::new(row_count, TimeAcc::MILLI);
            let mut cnt = 0;
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let name: String = row.get(1).unwrap();
                let namespace: usize = row.get(2).unwrap();

                let _ = conn.execute(tag_sqlite_inp, params![id, name, namespace]);
                eta.step();
                cnt += 1;
                if (cnt % count_five_percent) == 0 {
                    logging::info_log(&format!("{}", &eta));
                }
            }
        }
        self.db_drop_table(&"Tags_Old".to_string());
        if !self.check_table_exists("Jobs".to_string()) {
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
                "INTEGER PRIMARY KEY",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL"
            );

            self.table_create(&"Jobs".to_string(), keys, vals);
        }

        {
            let conn = self._conn.lock().unwrap();

            // Loads the Files into memory
            // Creates storage id's for them
            // Creates extension id's aswell
            //
            logging::info_log(&"Starting to process Jobs for DB V5 Upgrade".to_string());

            let tag_sqlite_inp = "INSERT INTO Jobs VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
            let mut stmt = conn.prepare("SELECT * FROM Jobs_Old").unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let time: usize = row.get(1).unwrap();
                let reptime: usize = row.get(2).unwrap();
                let manager: String = row.get(3).unwrap();
                let site: String = row.get(4).unwrap();
                let param: String = row.get(5).unwrap();
                let committype: String = row.get(6).unwrap();
                let systemdata: String = row.get(7).unwrap();
                let userdata: String = row.get(8).unwrap();

                logging::info_log(&format!("Adding JobId: {}", &id));
                let _ = conn.execute(
                    tag_sqlite_inp,
                    params![
                        id, time, reptime, manager, site, param, committype, systemdata, userdata
                    ],
                );
            }
        }
        self.db_drop_table(&"Jobs_Old".to_string());

        self.db_version_set(5);
        //self.vacuum();
    }

    ///
    /// Handles the DB upgrade from five to six
    ///
    pub fn db_update_five_to_six(&mut self) {
        logging::info_log(&"Backing up db this could be messy.".to_string());
        self.backup_db();

        // If table does not exist then create the dead source url tables
        if !self.check_table_exists("dead_source_urls".to_string()) {
            self.table_create(
                &"dead_source_urls".to_string(),
                &vec_of_strings!("id", "dead_url"),
                &vec_of_strings!("INTEGER PRIMARY KEY", "TEXT NOT NULL"),
            );
        }

        if self.check_table_exists("Jobs_Old".to_string()) {
            self.db_drop_table(&"Jobs_Old".to_string());
        }

        if self.check_table_exists("Jobs".to_string()) {
            self.alter_table(&"Jobs".to_string(), &"Jobs_Old".to_string());

            let keys = &vec_of_strings!(
                "id",
                "time",
                "reptime",
                "priority",
                "cachetime",
                "cachechecktype",
                "Manager",
                "site",
                "param",
                "SystemData",
                "UserData"
            );
            let vals = &vec_of_strings!(
                "INTEGER PRIMARY KEY",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER",
                "INTEGER NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL",
                "TEXT NOT NULL"
            );

            self.table_create(&"Jobs".to_string(), keys, vals);

            let conn = self._conn.lock().unwrap();

            // Loads the Files into memory
            // Creates storage id's for them
            // Creates extension id's aswell
            //
            logging::info_log(&"Starting to process Jobs for DB V6 Upgrade".to_string());

            let tag_sqlite_inp = "INSERT INTO Jobs VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
            let mut stmt = conn.prepare("SELECT * FROM Jobs_Old").unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let time: usize = row.get(1).unwrap();
                let reptime: usize = row.get(2).unwrap();
                let manager: String = row.get(3).unwrap();
                let site: String = row.get(4).unwrap();
                let param: String = row.get(5).unwrap();
                let systemdata: String = row.get(7).unwrap();
                let userdata: String = row.get(8).unwrap();

                logging::info_log(&format!("Adding JobId: {}", &id));
                conn.execute(
                    tag_sqlite_inp,
                    params![
                        id,
                        time,
                        reptime,
                        DEFAULT_PRIORITY,
                        serde_json::to_string(&DEFAULT_CACHETIME).unwrap(),
                        serde_json::to_string(&DEFAULT_CACHECHECK).unwrap(),
                        manager,
                        site,
                        param,
                        systemdata,
                        userdata
                    ],
                )
                .unwrap();
            }
        }

        self.db_drop_table(&"Jobs_Old".to_string());
        self.db_version_set(6);
    }

    pub fn db_update_six_to_seven(&mut self) {
        logging::info_log(&"Backing up db this could be messy.".to_string());
        self.backup_db();

        if self.check_table_exists("Namespace_Old".to_string()) {
            self.db_drop_table(&"Namespace_Old".to_string());
        }

        if self.check_table_exists("Parents_Old".to_string()) {
            self.db_drop_table(&"Parents_Old".to_string());
        }

        if self.check_table_exists("Tags_Old".to_string()) {
            self.db_drop_table(&"Tags_Old".to_string());
        }

        if self.check_table_exists("Namespace".into()) {
            self.alter_table(&"Namespace".to_string(), &"Namespace_Old".to_string());

            let keys = &vec_of_strings!("id", "name", "description");
            let vals = &vec_of_strings!("INTEGER PRIMARY KEY NOT NULL", "TEXT NOT NULL", "TEXT");

            self.table_create(&"Namespace".to_string(), keys, vals);

            let conn = self._conn.lock().unwrap();

            logging::info_log(&"Starting to process Namespace for DB V7 Upgrade".to_string());

            let tag_sqlite_inp = "INSERT INTO Namespace (id, name, description) VALUES (?, ?, ?)";
            let mut stmt = conn
                .prepare("SELECT id, name, description FROM Namespace_Old")
                .unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let name: String = row.get(1).unwrap();
                let desc: Option<String> = row.get(2).unwrap();

                logging::info_log(&format!("Adding Namespace: {}", &id));
                conn.execute(tag_sqlite_inp, params![id, name, desc])
                    .unwrap();
            }
        }
        if self.check_table_exists("Parents".into()) {
            self.alter_table(&"Parents".to_string(), &"Parents_Old".to_string());

            let keys = &vec_of_strings!("id", "tag_id", "relate_tag_id", "limit_to");
            let vals = &vec_of_strings!(
                "INTEGER PRIMARY KEY NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER NOT NULL",
                "INTEGER"
            );

            self.table_create(&"Parents".to_string(), keys, vals);

            let conn = self._conn.lock().unwrap();

            logging::info_log(&"Starting to process Parents for DB V7 Upgrade".to_string());

            let tag_sqlite_inp =
                "INSERT INTO Parents (tag_id, relate_tag_id, limit_to) VALUES (?, ?, ?)";
            let mut stmt = conn
                .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents_Old")
                .unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let tagid: usize = row.get(0).unwrap();
                let relatetagid: usize = row.get(1).unwrap();
                let limitto: Option<usize> = row.get(2).unwrap();

                conn.execute(tag_sqlite_inp, params![tagid, relatetagid, limitto])
                    .unwrap();
            }
        }

        self.transaction_flush();

        if self.check_table_exists("Tags".into()) {
            self.alter_table(&"Tags".to_string(), &"Tags_Old".to_string());
            let keys = &vec_of_strings!("id", "name", "namespace");
            let vals = &vec_of_strings!(
                "INTEGER PRIMARY KEY NOT NULL",
                "TEXT NOT NULL",
                "INTEGER NOT NULL"
            );

            self.table_create(&"Tags".to_string(), keys, vals);
            let conn = self._conn.lock().unwrap();

            logging::info_log(&"Starting to process Tags for DB V7 Upgrade".to_string());

            let tag_sqlite_inp = "INSERT INTO Tags (id, name, namespace) VALUES (?, ?, ?)";
            let mut stmt = conn
                .prepare("SELECT id, name, namespace FROM Tags_Old")
                .unwrap();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let id: usize = row.get(0).unwrap();
                let name: String = row.get(1).unwrap();
                let namespace: usize = row.get(2).unwrap();

                conn.execute(tag_sqlite_inp, params![id, name, namespace])
                    .unwrap();
            }
        }
        self.db_drop_table(&"Namespace_Old".to_string());
        self.db_drop_table(&"Parents_Old".to_string());
        self.db_drop_table(&"Tags_Old".to_string());
        self.db_version_set(7);
    }
}
