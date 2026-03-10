use crate::database::database::CacheType;
use crate::database::database::Main;
use crate::download::hash_file;
use crate::file;
use crate::helpers;
use crate::helpers::getfinpath;
use crate::logging;
use crate::sharedtypes;
use log::{error, info};
use remove_empty_subdirs::remove_empty_subdirs;
pub use rusqlite::types::ToSql;
pub use rusqlite::{Connection, Result, Transaction, params, types::Null};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use web_api::web_api;

#[web_api]
impl Main {
    ///
    /// Gets a scraper folder. If it doesn't exist then please create it in db
    ///
    pub fn loaded_scraper_folder(&self) -> PathBuf {
        match self.settings_get_name(&"scraperloadloc".to_string()) {
            Some(setting) => {
                if let Some(param) = &setting.param {
                    Path::new(param).to_path_buf()
                } else {
                    let mut write_conn = self.write_conn.lock();
                    let mut tn = write_conn.transaction().unwrap();
                    self.setting_add_internal(
                        &mut tn,
                        "scraperloadloc".to_string(),
                        Some("Where scrapers get loaded into.".to_string()),
                        None,
                        Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
                    );
                    tn.commit().unwrap();
                    Path::new(crate::DEFAULT_LOC_SCRAPER).to_path_buf()
                }
            }
            None => {
                let mut write_conn = self.write_conn.lock();
                let mut tn = write_conn.transaction().unwrap();
                self.setting_add_internal(
                    &mut tn,
                    "scraperloadloc".to_string(),
                    Some("Where scrapers get loaded into.".to_string()),
                    None,
                    Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
                );
                tn.commit().unwrap();
                Path::new(crate::DEFAULT_LOC_SCRAPER).to_path_buf()
            }
        }
    }

    ///
    /// Gets a plugin folder. If it doesn't exist then please create it in db
    ///
    pub fn loaded_plugin_folder(&self) -> PathBuf {
        match self.settings_get_name(&"pluginloadloc".to_string()) {
            Some(setting) => {
                if let Some(param) = &setting.param {
                    Path::new(param).to_path_buf()
                } else {
                    let mut write_conn = self.write_conn.lock();
                    let mut tn = write_conn.transaction().unwrap();
                    self.setting_add_internal(
                        &mut tn,
                        "pluginloadloc".to_string(),
                        Some("Where plugins get loaded into.".to_string()),
                        None,
                        Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
                    );
                    tn.commit().unwrap();
                    Path::new(crate::DEFAULT_LOC_PLUGIN).to_path_buf()
                }
            }
            None => {
                let mut write_conn = self.write_conn.lock();
                let mut tn = write_conn.transaction().unwrap();
                self.setting_add_internal(
                    &mut tn,
                    "pluginloadloc".to_string(),
                    Some("Where plugins get loaded into.".to_string()),
                    None,
                    Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
                );
                tn.commit().unwrap();
                Path::new(crate::DEFAULT_LOC_PLUGIN).to_path_buf()
            }
        }
    }
    /// Deletes a namespace by id
    pub fn delete_namespace_id(&self, nsid: &u64) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();

        self.namespace_delete_id(&tn, nsid);

        tn.commit().unwrap();
    }

    pub fn check_default_source_urls(&self, action: &sharedtypes::CheckSourceUrlsEnum) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.check_default_source_urls_internal(&tn, action);
        tn.commit().unwrap();
    }

    /// Checks relationships with table for any dead tagids
    pub fn check_relationship_tag_relations(&self) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.check_relationship_tag_relations_internal(&tn);
        tn.commit().unwrap();
    }

    /// Removes a job from the database by id. Removes from both memdb and sql.
    pub fn del_from_jobs_byid(&self, id: Option<u64>) {
        if let Some(ref id) = id {
            self.del_from_jobs_inmemdb(id);
            self.del_from_jobs_table_sql_better(id);
        }
    }

    pub fn file_add(&self, file: sharedtypes::DbFileStorage) -> u64 {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.file_add_internal(&tn, &file);
        tn.commit().unwrap();
        out
    }

    pub fn storage_put(&self, location: &String) -> u64 {
        if let Some(out) = self.storage_get_id(location) {
            return out;
        }
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.storage_put_internal(&tn, location);
        tn.commit().unwrap();
        out
    }

    /// Adds tags to fileid  commits to db
    pub fn add_tags_to_fileid(
        &self,
        file_id: Option<u64>,
        tag_actions: &Vec<sharedtypes::FileTagAction>,
    ) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.add_tags_to_fileid_internal(&tn, file_id, tag_actions);
        tn.commit().unwrap();
        out
    }

    pub fn delete_tag(&self, tag: &u64) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.delete_tag_sql(&tn, tag);
        tn.commit().unwrap();
    }

    pub fn parents_tagid_remove(&self, tagid: &u64) -> HashSet<sharedtypes::DbParentsObj> {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.parents_tagid_remove_internal(&tn, tagid);
        tn.commit().unwrap();
        out
    }

    pub fn add_relationship(&self, file: &u64, tag: &u64) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.add_relationship_sql(&tn, file, tag);
        tn.commit().unwrap();
    }
    pub fn delete_relationship(&self, file: &u64, tag: &u64) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.delete_relationship_sql(&tn, file, tag);
        tn.commit().unwrap();
    }

    ///
    /// Adds the tag to the db. commits on finish
    ///
    pub fn tag_add_tagobject(&self, tag: &sharedtypes::TagObject) -> Option<u64> {
        let out;
        let mut write_conn = self.write_conn.lock();
        {
            let tn = write_conn.transaction().unwrap();
            out = self.tag_add_tagobject_internal(&tn, tag);
            tn.commit().unwrap();
        }
        out
    }
    /// condesnes everything in db
    pub fn condense_db_all(&self) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.condense_db_all_internal(&tn);
        tn.commit().unwrap();
    }

    /// Sets a relationship between a fileid old and new tagid
    pub fn condense_tags(&self) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.condense_tags_internal(&tn);
        tn.commit().unwrap();
    }

    /// Sets a relationship between a fileid old and new tagid
    pub fn migrate_tag(&self, old_tag_id: &u64, new_tag_id: &u64) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.migrate_tag_internal(&tn, old_tag_id, new_tag_id);
        tn.commit().unwrap();
    }

    /// Sets a relationship between a fileid old and new tagid
    pub fn migrate_relationship_file_tag(&self, file_id: &u64, old_tag_id: &u64, new_tag_id: &u64) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.migrate_relationship_file_tag_internal(&tn, file_id, old_tag_id, new_tag_id);
        tn.commit().unwrap();
    }

    /// Updates the database for inmemdb and sql
    pub fn jobs_update_db(&self, jobs_obj: sharedtypes::DbJobsObj) {
        match self._cache {
            /*CacheType::Bare => {
                if self.jobs_get_id_sql( &jobs_obj.id.unwrap()).is_none() {
                    self.jobs_add_sql( &jobs_obj)
                } else {
                    self.jobs_update_by_id( &jobs_obj);
                }
            }*/
            _ => {
                if self._inmemdb.write().jobref_new(jobs_obj.clone()) {
                    self.jobs_update_by_id(&jobs_obj);
                } else {
                    self.jobs_add_sql(&jobs_obj)
                }
            }
        }
    }

    /// Removes a parent selectivly
    pub fn parents_selective_remove(&self, parentobj: &sharedtypes::DbParentsObj) {
        todo!("Need to fix this for sqlite");
        self._inmemdb.write().parents_selective_remove(parentobj);
    }

    /// Adds a parent into the db
    pub fn parents_add(&self, par: sharedtypes::DbParentsObj) -> u64 {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.parents_add_internal(&tn, par);
        tn.commit().unwrap();
        out
    }

    /// Adds tag into db
    pub fn tag_add(&self, tags: &String, namespace: u64, id: Option<u64>) -> u64 {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.tag_add_internal(&tn, tags, namespace, id);
        tn.commit().unwrap();
        out
    }

    /// Checks if table is loaded in mem and if not then loads it.
    pub fn load_table(&self, table: &sharedtypes::LoadDBTable) {
        // Blocks the thread until another thread has finished loading the table.
        while self.tables_loading.read().contains(table) {
            let dur = std::time::Duration::from_secs(1);
            std::thread::sleep(dur);
        }
        if !self.tables_loaded.read().contains(table) {
            self.tables_loading.write().push(*table);
            match &table {
                sharedtypes::LoadDBTable::Files => {
                    self.load_files();
                }
                sharedtypes::LoadDBTable::Jobs => {
                    self.load_jobs();
                }
                sharedtypes::LoadDBTable::Namespace => {
                    self.load_namespace();
                }
                sharedtypes::LoadDBTable::Parents => {
                    self.load_parents();
                }
                sharedtypes::LoadDBTable::Relationship => {
                    self.load_relationships();
                }
                sharedtypes::LoadDBTable::Settings => {
                    self.load_settings();
                }
                sharedtypes::LoadDBTable::Tags => {
                    self.load_tags();
                }
                sharedtypes::LoadDBTable::DeadSourceUrls => {
                    self.load_dead_urls();
                }
                sharedtypes::LoadDBTable::All => {
                    self.load_table(&sharedtypes::LoadDBTable::Tags);
                    self.load_table(&sharedtypes::LoadDBTable::Files);
                    self.load_table(&sharedtypes::LoadDBTable::Jobs);
                    self.load_table(&sharedtypes::LoadDBTable::Namespace);
                    self.load_table(&sharedtypes::LoadDBTable::Parents);
                    self.load_table(&sharedtypes::LoadDBTable::Relationship);
                    self.load_table(&sharedtypes::LoadDBTable::Settings);
                }
            }
            self.tables_loaded.write().push(*table);
            self.tables_loading.write().retain(|&x| x != *table);
        }
    }

    pub fn setting_add(
        &self,
        name: String,
        pretty: Option<String>,
        num: Option<u64>,
        param: Option<String>,
    ) {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        self.setting_add_internal(&tn, name, pretty, num, param);
        tn.commit().unwrap();
    }
    ///
    /// Adds a dead url into the db
    ///
    pub fn add_dead_url(&self, url_string: &String) {
        self.add_dead_url_sql(url_string);
    }

    /// Searches the database using FTS5 allows getting a list of tags and their count based on a
    /// search string and a limit of tagids to get
    pub fn search_tags(
        &self,
        search_string: &String,
        limit_to: &u64,
        fts_or_count: sharedtypes::TagPartialSearchType,
    ) -> Vec<(sharedtypes::Tag, u64, u64)> {
        let mut out = Vec::new();

        for (tag_id, count) in self
            .search_tags_sql(search_string, limit_to, fts_or_count)
            .iter()
        {
            if let Some(tag) = self.tag_id_get(tag_id)
                && let Some(namespace) = self.namespace_get_string(&tag.namespace)
            {
                out.push((
                    sharedtypes::Tag {
                        tag: tag.name,
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: namespace.name,
                            description: namespace.description,
                        },
                    },
                    *tag_id,
                    *count,
                ));
            }
        }

        out
    }
    /// Searches the database using FTS5 allows getting a list of tagids and their count based on a
    /// search string and a limit of tagids to get
    pub fn search_tags_ids(
        &self,
        search_string: &String,
        limit_to: &u64,
        fts_or_count: sharedtypes::TagPartialSearchType,
    ) -> Vec<(u64, u64)> {
        self.search_tags_sql(search_string, limit_to, fts_or_count)
    }

    /// A test function to return 1
    pub fn test(&self) -> u32 {
        1
    }

    /// Returns the db version number
    pub fn db_vers_get(&self) -> u64 {
        self._active_vers
    }
    ///
    /// Returns a list of loaded tag ids
    ///
    pub fn tags_get_list_id(&self) -> HashSet<u64> {
        self.tags_get_id_list_sql()
    }

    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(&self, tag: &u64) -> HashSet<u64> {

        if matches!(self._cache, CacheType::RelationshipRoaring) {
            let list = self.relationship_roaring_storage.read().relationship_search_fileid_roaring_and(&[*tag]);
            let mut out = HashSet::new();
            for item in list {
                out.insert(item);
            }
            return out;
        }

        self.relationship_get_fileid_sql(tag)
    }

    /// Gets one fileid from one tagid
    pub fn relationship_get_one_fileid(&self, tag: &u64) -> Option<u64> {


        if matches!(self._cache, CacheType::RelationshipRoaring) {

            if let Some(list) = self.relationship_roaring_storage.read().relationship_search_fileid_roaring_and(&[*tag]).pop() {
                return Some(list.into());
            };
        }


        //self._inmemdb.relationship_get_one_fileid(tag)
        let temp = self.relationship_get_fileid(tag);
        let out = temp.iter().next();
        out.copied()
    }

    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(&self, file_id: &u64) -> HashSet<u64> {
        if matches!(self._cache, CacheType::RelationshipRoaring) {
          /*  let mut out = HashSet::new();
            for tag in self.relationship_roaring_storage.read().relationship_search_tagid_roaring(file_id) {
                out.insert(tag);
            }
            return out;*/

        }
        self.relationship_get_tagid_sql(file_id)
    }

    pub fn settings_get_name(&self, name: &String) -> Option<sharedtypes::DbSettingObj> {
        self._inmemdb.read().settings_get_name(name).cloned()
    }

    ///
    /// Correct any weird paths existing inside of the db.
    ///
    pub fn check_db_paths(&self) {
        let db_paths = self.storage_get_all();

        let file_dump = self.setup_defaut_misplaced_location();
        let file_dump_path = Path::new(&file_dump);

        for db_path in db_paths.iter() {
            for entry in walkdir::WalkDir::new(db_path).into_iter().flatten() {
                if entry.path().is_file()
                    && let Some(filename) = entry.path().file_name()
                {
                    if self
                        .file_get_hash(&filename.to_string_lossy().to_string())
                        .is_none()
                    {
                        let (hash, _) = hash_file(
                            &entry.path().to_string_lossy().to_string(),
                            &sharedtypes::HashesSupported::Sha512("".to_string()),
                        )
                        .unwrap();
                        if self.file_get_hash(&hash).is_some() {
                            let cleaned_filepath = entry.path().to_path_buf();

                            let cleaned_filename = cleaned_filepath.as_path().file_name().unwrap();

                            let test_path = Path::new(&getfinpath(
                                db_path,
                                &entry
                                    .path()
                                    .file_name()
                                    .unwrap()
                                    .to_string_lossy()
                                    .to_string(),
                                true,
                            ))
                            .join(Path::new(&cleaned_filename));

                            logging::error_log(format!(
                                "While checking file paths I found a file path that had the wrong name / extension. {} Moving to: {}",
                                &entry.path().display(),
                                test_path.as_path().display()
                            ));

                            file::folder_make(
                                &file_dump_path
                                    .join(entry.path().parent().unwrap())
                                    .to_string_lossy()
                                    .to_string(),
                            );
                            std::fs::rename(entry.path(), test_path).unwrap();
                        } else {
                            logging::error_log(format!(
                                "While checking file paths I found a file path that shouldn't exist. {} Moving to: {}",
                                &entry.path().display(),
                                file_dump_path.display()
                            ));

                            file::folder_make(
                                &file_dump_path
                                    .join(entry.path().parent().unwrap())
                                    .to_string_lossy()
                                    .to_string(),
                            );
                            std::fs::rename(entry.path(), file_dump_path.join(entry.path()))
                                .unwrap();
                        }
                    } else {
                        let cleaned_filepath = entry.path().to_path_buf();

                        let cleaned_filename = cleaned_filepath.as_path().file_name().unwrap();

                        let test_path = Path::new(&getfinpath(
                            db_path,
                            &entry
                                .path()
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            true,
                        ))
                        .join(Path::new(&cleaned_filename));

                        if entry.path() != test_path {
                            logging::error_log(format!(
                                "While checking file paths I found a file path that is incorrect. {} Moving to: {}",
                                &entry.path().display(),
                                test_path.as_path().display()
                            ));
                            std::fs::rename(entry.path(), test_path).unwrap();
                        }
                    }

                    //dbg!(&entry.path().file_name());
                }
            }
            // Cleaning up empty folders from moving files.
            remove_empty_subdirs(Path::new(db_path)).unwrap();
        }
    }

    /// Backs up the DB file.
    pub fn backup_db(&self) {
        use chrono::prelude::*;

        // If we don't have a location set then eazy out
        let dbloc = match self.get_db_loc() {
            None => {
                return;
            }
            Some(location) => location,
        };

        let current_date = Utc::now();
        let year = current_date.year();
        let month = current_date.month();
        let day = current_date.day();
        let dbbackuploc = self.settings_get_name(&"db_backup_location".to_string());

        // Default location for the DB
        let defaultloc = String::from("dbbackup");

        // Gets the DB file location for copying.
        let mut add_backup_location = None;

        // Gets the db backup folder from DB or uses the "defaultloc" variable
        let backupfolder = match dbbackuploc {
            None => {
                add_backup_location = Some(defaultloc.clone());
                defaultloc
            }
            Some(dbsetting) => match &dbsetting.param {
                None => {
                    add_backup_location = Some(defaultloc.clone());
                    defaultloc
                }
                Some(loc) => loc.to_string(),
            },
        };
        let properbackuplocation;
        let properbackupfile;

        // Starting to do localization. gets changed at compile time. Super lazy way to do
        // it tho
        let mut cnt = 0;
        loop {
            if cfg!(target_os = "windows") {
                if Path::new(&format!(
                    "{}\\{}\\{}\\{}\\db{}.db",
                    backupfolder, year, month, day, cnt
                ))
                .exists()
                {
                    cnt += 1;
                } else {
                    properbackupfile = format!(
                        "{}\\{}\\{}\\{}\\db{}.db",
                        backupfolder, year, month, day, cnt
                    );
                    properbackuplocation =
                        format!("{}\\{}\\{}\\{}\\", backupfolder, year, month, day);
                    break;
                }
            } else if Path::new(&format!(
                "{}/{}/{}/{}/db{}.db",
                backupfolder, year, month, day, cnt
            ))
            .exists()
            {
                cnt += 1;
            } else {
                properbackupfile =
                    format!("{}/{}/{}/{}/db{}.db", backupfolder, year, month, day, cnt);
                properbackuplocation = format!("{}/{}/{}/{}/", backupfolder, year, month, day);
                break;
            }
        }

        *self.write_conn_istransaction.lock() = true;
        {
            let temp = self.write_conn.lock();
            // Creates and copies the DB into the backup folder.
            std::fs::create_dir_all(properbackuplocation.clone()).unwrap();
            logging::info_log(format!(
                "Copying db from: {} to: {}",
                &dbloc, &properbackupfile
            ));
            std::fs::copy(dbloc, properbackupfile).unwrap();
            *self.write_conn_istransaction.lock() = false;
        }
        if let Some(newbackupfolder) = add_backup_location {
            let mut write_conn = self.write_conn.lock();
            let mut tn = write_conn.transaction().unwrap();
            let out = self.setting_add_internal(
                &mut tn,
                "db_backup_location".to_string(),
                Some("The location that the DB get's backed up to".to_string()),
                None,
                Some(newbackupfolder),
            );
            tn.commit().unwrap();
            out
        }
        logging::info_log("Finished backing up the DB.".to_string());
    }

    /// Returns a files bytes if the file exists. Note if called from intcom then this
    /// locks the DB while getting the file. One workaround it to use get_file and read
    /// bytes in manually in seperate thread. that way minimal locking happens.
    pub fn get_file_bytes(&self, file_id: &u64) -> Option<Vec<u8>> {
        let loc = self.get_file(file_id);
        if let Some(loc) = loc {
            return Some(std::fs::read(loc).unwrap());
        }
        None
    }

    /// Gets the location of a file in the file system
    pub fn get_file(&self, file_id: &u64) -> Option<String> {
        let file = self.file_get_id(file_id);
        if let Some(file_obj) = file {
            // Checks that the file with existing info exists
            let file = match file_obj {
                sharedtypes::DbFileStorage::Exist(file) => file,
                _ => return None,
            };

            let location = self.storage_get_string(&file.storage_id).unwrap();

            let sup = [location, self.location_get()];
            for each in sup {
                // Cleans the file path if it contains a '/' in it
                let loc = if each.ends_with('/') | each.ends_with('\\') {
                    each[0..each.len() - 1].to_string()
                } else {
                    each.to_string()
                };
                let folderloc = helpers::getfinpath(&loc, &file.hash, false);

                let out;
                if cfg!(unix) {
                    out = format!("{}/{}", folderloc, file.hash);
                } else if cfg!(windows) {
                    out = format!("{}\\{}", folderloc, file.hash);
                } else {
                    logging::error_log("UNSUPPORTED OS FOR GETFILE CALLING.".to_string());
                    return None;
                }

                // No idea why this is faster? Metadata maybe
                if let Ok(filepath) = std::fs::canonicalize(&out) {
                    return Some(filepath.to_string_lossy().to_string());
                }

                // New revision of the downloader adds the extension to the file downloaded.
                // This will rename the file if it uses the old file ext
                if let Some(ref ext_str) = self.extension_get_string(&file.ext_id)
                    && Path::new(&out).with_extension(ext_str).exists()
                {
                    return Some(
                        std::fs::canonicalize(
                            Path::new(&out)
                                .with_extension(ext_str)
                                .to_string_lossy()
                                .to_string(),
                        )
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    );
                }
            }
        }
        None
    }

    ///
    ///Checks if a url is dead
    ///
    pub fn check_dead_url(&self, url_to_check: &String) -> bool {
        self.does_dead_source_exist(url_to_check)
    }

    /// Gets all running jobs in the db
    pub fn jobs_get_isrunning(&self) -> HashSet<sharedtypes::DbJobsObj> {
        match self._cache {
            CacheType::Bare => {
                todo!("Bare implementation not implemineted");
            }
            _ => self._inmemdb.read().jobref_get_isrunning(),
        }
    }
    ///
    /// Returns all locations currently inside of the db.
    ///
    pub fn storage_get_all(&self) -> Vec<String> {
        let mut out = Vec::new();
        for id in 1..u64::MAX {
            if let Some(location) = self.storage_get_string(&id) {
                out.push(location);
            } else {
                break;
            }
        }
        out
    }

    /// Handles the searching of the DB dynamically. Returns the file id's associated
    /// with the search.
    /// Returns file IDs matching the search.
    /// Supports AND, OR, NOT operations.
    pub fn search_db_files(
        &self,
        search: sharedtypes::SearchObj,
        limit: Option<u64>,
    ) -> Option<Vec<u64>> {
        use rusqlite::params_from_iter;
        let start_time = Instant::now();
        // Separate AND / OR / NOT
        let mut and_tags = Vec::new();
        let mut or_groups: Vec<Vec<u64>> = Vec::new();
        let mut not_groups: Vec<Vec<u64>> = Vec::new();

        for holder in search.searches {
            match holder {
                sharedtypes::SearchHolder::And(ids) => and_tags.extend(ids),
                sharedtypes::SearchHolder::Or(ids) if !ids.is_empty() => or_groups.push(ids),
                sharedtypes::SearchHolder::Not(ids) if !ids.is_empty() => not_groups.push(ids),
                _ => {}
            }
        }

        if and_tags.is_empty() {
            return None;
        }

        // Cache returns locally if popular
        if matches!(self._cache, CacheType::RelationshipRoaring) {
            let mut test = self
                .relationship_roaring_storage
                .read()
                .relationship_search_fileid_roaring_and(&and_tags);

            
            test.sort_by_key(|&key| Reverse(key));
if let Some(limit) = limit {
                test.truncate(limit as usize);
            }


            return Some(test);
        }

        // Pick rarest AND tag dynamically
        let placeholders = vec!["?"; and_tags.len()].join(", ");
        let driver_sql = format!(
            "SELECT id FROM Tags WHERE id IN ({}) ORDER BY count ASC LIMIT 1",
            placeholders
        );

        let mut conn = self.get_database_connection();
        let driver_tag: u64 =
            match conn.query_row(&driver_sql, params_from_iter(&and_tags), |row| row.get(0)) {
                Ok(tag) => tag,
                Err(_) => return None, // treat errors as no results
            };

        let remaining_and: Vec<u64> = and_tags
            .into_iter()
            .filter(|&id| id != driver_tag)
            .collect();

        let table =
            if self.is_tag_count_greater_rel_limit(&conn.transaction().unwrap(), &driver_tag) {
                "Relationship_Popular"
            } else {
                "Relationship"
            };

        // Build search SQL
        let mut sql = format!(
            "SELECT r.fileid
         FROM {table} r 
         WHERE r.tagid = ?",
        );
        let mut params: Vec<u64> = vec![driver_tag];

        for tag in &remaining_and {
            let table = if self.is_tag_count_greater_rel_limit(&conn.transaction().unwrap(), &tag) {
                "Relationship_Popular"
            } else {
                "Relationship"
            };

            sql.push_str(&format!(
                "
            AND EXISTS (
                SELECT 1 FROM {table} r2
                WHERE r2.tagid = ?
                  AND r2.fileid = r.fileid
            )"
            ));
            params.push(*tag);
        }

        for group in &or_groups {
            let placeholders = vec!["?"; group.len()].join(", ");
            sql.push_str(&format!(
                "
            AND EXISTS (
                SELECT 1 FROM Relationship r3
                WHERE r3.tagid IN ({})
                  AND r3.fileid = r.fileid
            )",
                placeholders
            ));
            params.extend(group);
        }

        for group in &not_groups {
            let placeholders = vec!["?"; group.len()].join(", ");
            sql.push_str(&format!(
                "
            AND NOT EXISTS (
                SELECT 1 FROM Relationship r4
                WHERE r4.tagid IN ({})
                  AND r4.fileid = r.fileid
            )",
                placeholders
            ));
            params.extend(group);
        }
        sql.push_str(" ORDER BY r.fileid DESC");
        if let Some(lim) = limit {
            sql.push_str(" LIMIT ?");
            params.push(lim);
        }

        dbg!(&sql, &params);

        // Execute query
        let mut stmt = match conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let rows = match stmt.query_map(params_from_iter(params.iter()), |row| row.get(0)) {
            Ok(r) => r,
            Err(_) => return None,
        };

        let mut results = Vec::new();
        for r in rows {
            if let Ok(fileid) = r {
                results.push(fileid);
            }
        }
        let duration = start_time.elapsed();

        // Print the duration in a readable format (e.g., seconds, milliseconds)
        println!("Time taken: {:?}", duration);
        println!("Time taken in seconds: {}", duration.as_secs_f64());

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }
    /// Gets all jobs loaded in the db
    pub fn jobs_get_all(&self) -> HashMap<u64, sharedtypes::DbJobsObj> {
        match &self._cache {
            //CacheType::Bare => self.jobs_get_all_sql(),
            _ => self._inmemdb.read().jobs_get_all().clone(),
        }
    }

    /// Pull job by id TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    pub fn jobs_get(&self, id: &u64) -> Option<sharedtypes::DbJobsObj> {
        match self._cache {
            // CacheType::Bare => self.jobs_get_id_sql( id),
            _ => self._inmemdb.read().jobs_get(id).cloned(),
        }
    }

    ///
    /// Gets a tag by id
    ///
    pub fn tag_id_get(&self, uid: &u64) -> Option<sharedtypes::DbTagNNS> {
        self.tags_get_dbtagnns_sql(uid)
    }

    /// Vacuums database. cleans everything.
    pub(super) fn vacuum(&self) {
        logging::info_log("Starting Vacuum db!".to_string());
        {
            self.transaction_flush();
            let tn = self.write_conn.lock();
            tn.execute("VACUUM", []).unwrap();
        }
        logging::info_log("Finishing Vacuum db!".to_string());
    }

    /// Analyzes the sqlite database. Shouldn't need this but will be nice for indexes
    pub fn analyze(&self) {
        logging::info_log("Starting to analyze db!".to_string());
        {
            self.transaction_flush();
            let tn = self.write_conn.lock();
            tn.execute("ANALYZE", []).unwrap();
        }
        logging::info_log("Finishing analyze db!".to_string());
    }

    ///
    /// Convience function to get a list of files that are images
    ///
    pub fn extensions_images_get_fileid(&self) -> HashSet<u64> {
        let exts = &["jpg".to_string(), "png".to_string(), "tiff".to_string()];
        self.extensions_get_fileid_extstr_sql(exts)
    }

    ///
    /// Convience function to get a list of files that are videos
    ///
    pub fn extensions_videos_get_fileid(&self) -> HashSet<u64> {
        let exts = &[
            "mkv".to_string(),
            "webm".to_string(),
            "gif".to_string(),
            "mp4".to_string(),
        ];
        self.extensions_get_fileid_extstr_sql(exts)
    }

    ///
    /// Gets an ID if a extension string exists
    ///
    pub fn extension_get_string(&self, ext_id: &u64) -> Option<String> {
        self.extension_get_string_sql(ext_id)
    }
    ///
    /// Gets a fileid from a hash
    ///
    pub fn file_get_hash(&self, hash: &String) -> Option<u64> {
        self.file_get_id_sql(hash)
    }

    /// Gets a file from storage from its id
    pub fn file_get_id(&self, file_id: &u64) -> Option<sharedtypes::DbFileStorage> {
        self.files_get_id_sql(file_id)
    }

    /// Returns all file id's loaded in db
    pub fn file_get_list_id(&self) -> HashSet<u64> {
        self.file_get_list_id_sql()
    }

    pub fn file_get_list_all(&self) -> HashMap<u64, sharedtypes::DbFileStorage> {
        let mut out = HashMap::new();
        for fid in self.file_get_list_id() {
            if let Some(file) = self.file_get_id(&fid) {
                out.insert(fid, file);
            }
        }
        out
    }

    ///
    /// Gets a tagid from a unique tag and namespace combo
    ///
    pub fn tag_get_name(&self, tag: String, namespace: u64) -> Option<u64> {
        let tagobj = &sharedtypes::DbTagNNS {
            name: tag,
            namespace,
        };

        self.tag_get_name_tagobject(tagobj)
    }

    ///
    /// Gets a tagid from a tagobject
    ///
    pub fn tag_get_name_tagobject(&self, tagobj: &sharedtypes::DbTagNNS) -> Option<u64> {
        self.tags_get_id_sql(tagobj)
    }

    /// db get namespace wrapper
    pub fn namespace_get(&self, namespace: &String) -> Option<u64> {
        let mut tn = self.get_database_connection();
        self.namespace_get_id_sql(&tn, namespace)
    }

    /// Returns namespace as a string from an ID returns None if it doesn't exist.
    pub fn namespace_get_string(&self, ns_id: &u64) -> Option<sharedtypes::DbNamespaceObj> {
        self.namespace_get_namespaceobj_sql(ns_id)
    }

    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(&self, id: &u64) -> HashSet<u64> {
        self.namespace_get_tagids_sql(id)
    }

    /// Checks if a tag exists in a namespace
    pub fn namespace_contains_id(&self, namespace_id: &u64, tag_id: &u64) -> bool {
        self.namespace_contains_id_sql(tag_id, namespace_id)
    }

    /// Retuns namespace id's
    pub fn namespace_keys(&self) -> Vec<u64> {
        self.namespace_keys_sql()
    }

    ///
    /// Gets a parent id if they exist
    ///
    pub(super) fn parents_get(&self, parent: &sharedtypes::DbParentsObj) -> Option<u64> {
        let tagid = self.parents_get_id_list_sql(parent);

        if tagid.is_empty() {
            None
        } else {
            let tags: Vec<u64> = tagid.into_iter().collect();
            Some(tags[0])
        }
    }

    /// Relates the list of relationships assoicated with tag
    pub fn parents_rel_get(&self, relid: &u64) -> HashSet<u64> {
        self.parents_tagid_get(relid)
    }

    /// Relates the list of tags assoicated with relations
    pub fn parents_tag_get(&self, tagid: &u64) -> HashSet<u64> {
        self.parents_relatetagid_get(tagid)
    }

    /// Returns the location of the file storage path. Helper function
    pub fn location_get(&self) -> String {
        self.settings_get_name(&"FilesLoc".to_string())
            .unwrap()
            .param
            .as_ref()
            .unwrap()
            .to_owned()
    }

    ///
    /// commits an exclusive write transaction
    ///
    pub fn transaction_flush(&self) {
        let mut transaction = self.write_conn_istransaction.lock();
        if *transaction {
            logging::log("Flushing to disk");
            let conn = self.write_conn.lock();
            conn.execute("COMMIT", []).unwrap();
            *transaction = false;
        }
    }

    pub fn namespace_add(&self, name: &String, description: &Option<String>) -> u64 {
        let mut write_conn = self.write_conn.lock();
        let tn = write_conn.transaction().unwrap();
        let out = self.namespace_add_internal(&tn, name, description);
        tn.commit().unwrap();
        out
    }

    ///
    /// Adds a ns into the db if the id already exists
    ///
    pub fn namespace_add_id_exists(&self, ns: sharedtypes::DbNamespaceObj) -> u64 {
        let mut write_conn = self.write_conn.lock();
        let mut tn = write_conn.transaction().unwrap();
        self.namespace_add_sql(&tn, &ns.name, &ns.description, Some(ns.id));
        let out = self.namespace_get_id_sql(&tn,&ns.name).unwrap();
        tn.commit().unwrap();
        out
    }
    ///
    /// Gets a default namespace id if it doesn't exist
    ///
    pub fn create_default_source_url_ns_id(&self) -> u64 {
        match self.namespace_get(&"source_url".to_string()) {
            None => self.namespace_add(
                &"source_url".to_string(),
                &Some("Source URL for a file.".to_string()),
            ),
            Some(id) => id,
        }
    }
}
