#![forbid(unsafe_code)]
use crate::Mutex;
use crate::RwLock;
use crate::download::hash_file;
use crate::file;
use crate::globalload::GlobalLoad;
use crate::helpers;
use crate::helpers::check_url;
use crate::helpers::getfinpath;
use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::DEFAULT_CACHECHECK;
use crate::sharedtypes::ScraperParam;
use eta::{Eta, TimeAcc};
use log::{error, info};
use r2d2::Pool;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::prelude::*;
use remove_empty_subdirs::remove_empty_subdirs;
pub use rusqlite::types::ToSql;
pub use rusqlite::{Connection, Result, Transaction, params, types::Null};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use web_api::web_api;

use crate::database::inmemdbnew::NewinMemDB;
/// I dont want to keep writing .to_string on EVERY vector of strings. Keeps me
/// lazy. vec_of_strings["one", "two"];
#[macro_export]
macro_rules! vec_of_strings{
    ($($x: expr), *) =>(vec![$($x.to_string()), *]);
}

/*/// Returns an open tnection to use.
pub fn dbinit(dbpath: &String) -> tnection {
    // Engaging &Transaction Handling
    tnection::open(dbpath).unwrap()
}*/

#[derive(Clone, Debug)]
pub enum CacheType {
    // Default option. Will use in memory DB to make store cached data.
    InMemdb,
    // Not yet implmented will be used for using sqlite 3 inmemory db calls.
    InMemory(Pool<SqliteConnectionManager>),
    // Will be use to query the DB directly. No caching.
    Bare,
}

#[derive(Clone)]
/// Holder of database self variables
pub struct Main {
    pub(super) _dbpath: Option<String>,
    pub(super) _vers: usize,
    pub(super) pool: Pool<SqliteConnectionManager>,
    pub write_conn: Arc<Mutex<PooledConnection<SqliteConnectionManager>>>,
    pub(super) write_conn_istransaction: Arc<Mutex<bool>>,
    pub(super) _active_vers: usize,
    pub(super) _inmemdb: Arc<RwLock<NewinMemDB>>,
    pub(super) tables_loaded: Arc<RwLock<Vec<sharedtypes::LoadDBTable>>>,
    pub(super) tables_loading: Arc<RwLock<Vec<sharedtypes::LoadDBTable>>>,
    pub(super) _cache: CacheType,
    pub globalload: Option<Arc<GlobalLoad>>,
    pub(super) localref: Option<Arc<RwLock<Main>>>,
}

/// Handles transactional pushes.
pub fn transaction_execute(trans: &Transaction, inp: String) {
    trans.execute(&inp, params![]).unwrap();
}

/// Read only items from the db
#[web_api]
impl Main {
    /// Searches the database using FTS5 allows getting a list of tags and their count based on a
    /// search string and a limit of tagids to get
    pub fn search_tags(
        &self,
        search_string: &String,
        limit_to: &usize,
        fts_or_count: sharedtypes::TagPartialSearchType
    ) -> Vec<(sharedtypes::Tag, usize, usize)> {
        let mut out = Vec::new();

        for (tag_id, count) in self.search_tags_sql(search_string, limit_to, fts_or_count).iter() {
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
    pub fn search_tags_ids(&self, search_string: &String, limit_to: &usize, fts_or_count: sharedtypes::TagPartialSearchType) -> Vec<(usize, usize)> {
        self.search_tags_sql(search_string, limit_to, fts_or_count)
    }

    /// A test function to return 1
    pub fn test(&self) -> u32 {
        1
    }

    /// Returns the db version number
    pub fn db_vers_get(&self) -> usize {
        self._active_vers
    }
    ///
    /// Returns a list of loaded tag ids
    ///
    pub fn tags_get_list_id(&self) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.tags_get_id_list_sql(),
            _ => self._inmemdb.read().tags_get_list_id(),
        }
    }

    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(&self, tag: &usize) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.relationship_get_fileid_sql(tag),
            _ => self._inmemdb.read().relationship_get_fileid(tag),
        }
    }

    /// Gets one fileid from one tagid
    pub fn relationship_get_one_fileid(&self, tag: &usize) -> Option<usize> {
        //self._inmemdb.relationship_get_one_fileid(tag)
        let temp = self.relationship_get_fileid(tag);
        let out = temp.iter().next();
        out.copied()
    }

    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(&self, file_id: &usize) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.relationship_get_tagid_sql(file_id),
            _ => self._inmemdb.read().relationship_get_tagid(file_id),
        }
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

        self.transaction_flush();
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
            self.setting_add(
                "db_backup_location".to_string(),
                Some("The location that the DB get's backed up to".to_string()),
                None,
                Some(newbackupfolder),
            )
        }
        self.transaction_flush();
        logging::info_log("Finished backing up the DB.".to_string());
    }

    /// Returns a files bytes if the file exists. Note if called from intcom then this
    /// locks the DB while getting the file. One workaround it to use get_file and read
    /// bytes in manually in seperate thread. that way minimal locking happens.
    pub fn get_file_bytes(&self, file_id: &usize) -> Option<Vec<u8>> {
        let loc = self.get_file(file_id);
        if let Some(loc) = loc {
            return Some(std::fs::read(loc).unwrap());
        }
        None
    }

    /// Gets the location of a file in the file system
    pub fn get_file(&self, file_id: &usize) -> Option<String> {
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
        match self._cache {
            CacheType::Bare => self.does_dead_source_exist(url_to_check),
            _ => match self._inmemdb.read().does_dead_source_exist(url_to_check) {
                true => true,
                // Need to check if we have a cache miss
                false => self.does_dead_source_exist(url_to_check),
            },
        }
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
        for id in 1..usize::MAX {
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
    limit: Option<usize>,
) -> Option<Vec<usize>> {
    let tn = self.pool.get().unwrap();
    let mut included_files: Option<HashSet<usize>> = None;

    for search_item in search.searches.into_iter() {
        let tag_ids = match &search_item {
            sharedtypes::SearchHolder::And(ids)
            | sharedtypes::SearchHolder::Or(ids)
            | sharedtypes::SearchHolder::Not(ids) => ids,
        };

        if tag_ids.is_empty() {
            continue;
        }

        // Build SQL placeholders
        let placeholders = vec!["?"; tag_ids.len()].join(", ");
        let sql = match &search_item {
            sharedtypes::SearchHolder::And(_) => format!(
                "SELECT fileid
                 FROM Relationship
                 WHERE tagid IN ({})
                 GROUP BY fileid
                 HAVING COUNT(DISTINCT tagid) = {}",
                placeholders,
                tag_ids.len()
            ),
            sharedtypes::SearchHolder::Or(_) => format!(
                "SELECT DISTINCT fileid
                 FROM Relationship
                 WHERE tagid IN ({})",
                placeholders
            ),
            sharedtypes::SearchHolder::Not(_) => format!(
                "SELECT DISTINCT fileid
                 FROM Relationship
                 WHERE tagid IN ({})",
                placeholders
            ),
        };

        // Run the query
        let mut stmt = tn.prepare(&sql).unwrap();
        let result_fileids: HashSet<usize> = stmt
            .query_map(rusqlite::params_from_iter(tag_ids.iter()), |row| row.get(0))
            .unwrap()
            .flatten()
            .collect();

        match &search_item {
            sharedtypes::SearchHolder::And(_) => {
                included_files = Some(match included_files {
                    Some(current) => current
                        .intersection(&result_fileids)
                        .cloned()
                        .collect(),
                    None => result_fileids,
                });
            }
            sharedtypes::SearchHolder::Or(_) => {
                included_files = Some(match included_files {
                    Some(current) => current
                        .union(&result_fileids)
                        .cloned()
                        .collect(),
                    None => result_fileids,
                });
            }
            sharedtypes::SearchHolder::Not(_) => {
                if let Some(current) = included_files.as_mut() {
                    for fileid in result_fileids {
                        current.remove(&fileid);
                    }
                } else {
                    // If included_files is empty, treat NOT as excluding from all files
                    // Optional: fetch all fileids once if needed
                    let mut all_files_stmt = tn.prepare("SELECT id FROM File").unwrap();
                    let all_files: HashSet<usize> = all_files_stmt
                        .query_map([], |row| row.get(0))
                        .unwrap()
                        .flatten()
                        .collect();
                    let filtered: HashSet<_> =
                        all_files.difference(&result_fileids).cloned().collect();
                    included_files = Some(filtered);
                }
            }
        }
    }

    // Convert the final HashSet to a Vec and apply limit
    let mut out = match included_files {
        Some(s) => s.into_iter().collect::<Vec<usize>>(),
        None => Vec::new(),
    };

    // Optional: sort for deterministic order
    out.sort_unstable();

    if let Some(lim) = limit {
        out.truncate(lim);
    }

    Some(out)
}
    /// Gets all jobs loaded in the db
    pub fn jobs_get_all(&self) -> HashMap<usize, sharedtypes::DbJobsObj> {
        match &self._cache {
            //CacheType::Bare => self.jobs_get_all_sql(),
            _ => self._inmemdb.read().jobs_get_all().clone(),
        }
    }

    /// Pull job by id TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    pub fn jobs_get(&self, id: &usize) -> Option<sharedtypes::DbJobsObj> {
        match self._cache {
            // CacheType::Bare => self.jobs_get_id_sql( id),
            _ => self._inmemdb.read().jobs_get(id).cloned(),
        }
    }

    ///
    /// Gets a tag by id
    ///
    pub fn tag_id_get(&self, uid: &usize) -> Option<sharedtypes::DbTagNNS> {
        match self._cache {
            CacheType::Bare => self.tags_get_dbtagnns_sql(uid),
            _ => self._inmemdb.read().tags_get_data(uid).cloned(),
        }
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
    pub(super) fn analyze(&self) {
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
    pub fn extensions_images_get_fileid(&self) -> HashSet<usize> {
        let exts = &["jpg".to_string(), "png".to_string(), "tiff".to_string()];
        self.extensions_get_fileid_extstr_sql(exts)
    }

    ///
    /// Convience function to get a list of files that are videos
    ///
    pub fn extensions_videos_get_fileid(&self) -> HashSet<usize> {
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
    pub fn extension_get_id(&self, ext: &String) -> Option<usize> {
        match self._cache {
            CacheType::Bare => self.extension_get_id_sql(ext),
            _ => self._inmemdb.read().extension_get_id(ext).copied(),
        }
    }
    ///
    /// Gets an ID if a extension string exists
    ///
    pub fn extension_get_string(&self, ext_id: &usize) -> Option<String> {
        match self._cache {
            CacheType::Bare => self.extension_get_string_sql(ext_id),
            _ => self._inmemdb.read().extension_get_string(ext_id).cloned(),
        }
    }
    ///
    /// Gets a fileid from a hash
    ///
    pub fn file_get_hash(&self, hash: &String) -> Option<usize> {
        match self._cache {
            CacheType::Bare => self.file_get_id_sql(hash),
            _ => self._inmemdb.read().file_get_hash(hash).copied(),
        }
    }

    /// Gets a file from storage from its id
    pub fn file_get_id(&self, file_id: &usize) -> Option<sharedtypes::DbFileStorage> {
        match self._cache {
            CacheType::Bare => self.files_get_id_sql(file_id),
            _ => self._inmemdb.read().file_get_id(file_id).cloned(),
        }
    }

    /// Returns all file id's loaded in db
    pub fn file_get_list_id(&self) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.file_get_list_id_sql(),
            _ => self._inmemdb.read().file_get_list_id(),
        }
    }

    pub fn file_get_list_all(&self) -> HashMap<usize, sharedtypes::DbFileStorage> {
        match self._cache {
            CacheType::Bare => {
                let mut out = HashMap::new();
                for fid in self.file_get_list_id() {
                    if let Some(file) = self.file_get_id(&fid) {
                        out.insert(fid, file);
                    }
                }
                out
            }
            _ => self._inmemdb.read().file_get_list_all().clone(),
        }
    }

    ///
    /// Gets a tagid from a unique tag and namespace combo
    ///
    pub fn tag_get_name(&self, tag: String, namespace: usize) -> Option<usize> {
        let tagobj = &sharedtypes::DbTagNNS {
            name: tag,
            namespace,
        };

        self.tag_get_name_tagobject(tagobj)
    }

    ///
    /// Gets a tagid from a tagobject
    ///
    pub fn tag_get_name_tagobject(&self, tagobj: &sharedtypes::DbTagNNS) -> Option<usize> {
        match self._cache {
            CacheType::Bare => self.tags_get_id_sql(tagobj),
            _ => self._inmemdb.read().tags_get_id(tagobj).copied(),
        }
    }

    /// db get namespace wrapper
    pub fn namespace_get(&self, namespace: &String) -> Option<usize> {
        match self._cache {
            CacheType::Bare => self.namespace_get_id_sql(namespace),
            _ => self._inmemdb.read().namespace_get(namespace).copied(),
        }
    }

    /// Returns namespace as a string from an ID returns None if it doesn't exist.
    pub fn namespace_get_string(&self, ns_id: &usize) -> Option<sharedtypes::DbNamespaceObj> {
        match self._cache {
            CacheType::Bare => self.namespace_get_namespaceobj_sql(ns_id),
            _ => self._inmemdb.read().namespace_id_get(ns_id).cloned(),
        }
    }

    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(&self, id: &usize) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.namespace_get_tagids_sql(id),
            _ => self._inmemdb.read().namespace_get_tagids(id),
        }
    }

    /// Checks if a tag exists in a namespace
    pub fn namespace_contains_id(&self, namespace_id: &usize, tag_id: &usize) -> bool {
        match self._cache {
            CacheType::Bare => self.namespace_contains_id_sql(tag_id, namespace_id),
            _ => self.namespace_get_tagids(namespace_id).contains(tag_id),
        }
    }

    /// Retuns namespace id's
    pub fn namespace_keys(&self) -> Vec<usize> {
        match self._cache {
            CacheType::Bare => self.namespace_keys_sql(),
            _ => self._inmemdb.read().namespace_keys(),
        }
    }

    ///
    /// Gets a parent id if they exist
    ///
    pub(super) fn parents_get(&self, parent: &sharedtypes::DbParentsObj) -> Option<usize> {
        match self._cache {
            CacheType::Bare => {
                let tagid = self.parents_get_id_list_sql(parent);

                if tagid.is_empty() {
                    None
                } else {
                    let tags: Vec<usize> = tagid.into_iter().collect();
                    Some(tags[0])
                }
            }
            _ => self._inmemdb.read().parents_get(parent).copied(),
        }
    }

    /// Relates the list of relationships assoicated with tag
    pub fn parents_rel_get(&self, relid: &usize) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.parents_tagid_get(relid),
            _ => self._inmemdb.read().parents_rel_get(relid, None),
        }
    }

    /// Relates the list of tags assoicated with relations
    pub fn parents_tag_get(&self, tagid: &usize) -> HashSet<usize> {
        match self._cache {
            CacheType::Bare => self.parents_relatetagid_get(tagid),
            _ => self._inmemdb.read().parents_tag_get(tagid, None),
        }
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
}

/// Contains DB functions.
impl Main {
    /// Sets up new db instance.
    pub fn new(path: Option<String>, vers: usize) -> Self {
        // Initiates two tnections to the DB. Cheap workaround to avoid loading errors.
        let mut first_time_load_flag = false;
        let mut main = match path {
            Some(ref file_path) => {
                first_time_load_flag = Path::new(&file_path).exists();
                let memdb = Arc::new(RwLock::new(NewinMemDB::new()));
                let manager = SqliteConnectionManager::memory();
                let pool = r2d2::Builder::new().max_size(20).build(manager).unwrap();
                let write_conn = Arc::new(Mutex::new(pool.get().unwrap()));
                let write_conn_istransaction = Arc::new(Mutex::new(false));
                let memdbmain = Main {
                    _dbpath: path.clone(),
                    _vers: vers,
                    pool,
                    write_conn,
                    write_conn_istransaction,
                    _active_vers: 0,
                    _inmemdb: memdb.clone(),
                    tables_loaded: Arc::new(vec![].into()),
                    tables_loading: Arc::new(vec![].into()),
                    _cache: CacheType::Bare,
                    globalload: None,
                    localref: None,
                };
                //                let tnection = dbinit(file_path);
                let manager = SqliteConnectionManager::file(file_path).with_init(|conn| {
                    conn.execute_batch(
                        "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA secure_delete = 0;
            PRAGMA busy_timeout = 20000;
            PRAGMA page_size = 8192;
            PRAGMA cache_size = -800000;
            PRAGMA mmap_size = 30000000000;
            PRAGMA temp_store = MEMORY;
            PRAGMA wal_autocheckpoint = 20000;
    ",
                    )?;

                    // Enable SQL tracing
                    /*   conn.trace(Some(|sql| {
                        println!("[SQL TRACE] {}", sql);
                    }));*/
                    Ok(())
                });
                let pool = r2d2::Builder::new()
                    .idle_timeout(Some(Duration::from_secs(5)))
                    .max_size(10)
                    .build(manager)
                    .unwrap();
                let write_conn = Arc::new(Mutex::new(pool.get().unwrap()));
                let write_conn_istransaction = Arc::new(Mutex::new(false));
                let main = Main {
                    _dbpath: path,
                    _vers: vers,
                    pool,
                    write_conn,
                    write_conn_istransaction,
                    _active_vers: 0,
                    _inmemdb: memdb.clone(),
                    tables_loaded: Arc::new(vec![].into()),
                    tables_loading: Arc::new(vec![].into()),
                    _cache: CacheType::InMemdb,
                    globalload: None,
                    localref: None,
                };
                main
            }
            None => {
                first_time_load_flag = false;
                let memdb = Arc::new(RwLock::new(NewinMemDB::new()));
                let db_uuid = uuid::Uuid::new_v4();
                let memdb_uri = format!("file:memdb_{}?mode=memory&cache=shared", db_uuid);

                let manager = SqliteConnectionManager::file(&memdb_uri).with_init(|conn| {
                    conn.execute_batch(
                        "
PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA secure_delete = 0;PRAGMA busy_timeout = 20000;
            PRAGMA page_size = 8192;
            PRAGMA cache_size = -500000;

                        ",
                    )?;
                    conn.trace(Some(|sql| println!("[SQL TRACE] {}", sql)));
                    Ok(())
                });

                let pool = r2d2::Builder::new().max_size(100).build(manager).unwrap();

                // Grab a "write" connection for operations that need exclusive access
                let write_conn = Arc::new(Mutex::new(pool.get().unwrap()));
                let write_conn_istransaction = Arc::new(Mutex::new(false));

                let main = Main {
                    _dbpath: None,
                    _vers: vers,
                    pool,
                    write_conn,
                    write_conn_istransaction,
                    _active_vers: 0,
                    _inmemdb: memdb.clone(),
                    tables_loaded: Arc::new(vec![].into()),
                    tables_loading: Arc::new(vec![].into()),
                    _cache: CacheType::Bare,
                    globalload: None,
                    localref: None,
                };
                main
            }
        };
        // let path = String::from("./main.db");
        // Sets default settings for db settings.
        if !first_time_load_flag {
            // Database Doesn't exist
            {
                let mut write_conn = main.write_conn.lock();
                let mut transaction = write_conn.transaction().unwrap();
                main.first_db(&mut transaction);
                transaction.commit().unwrap();
            }

            main.transaction_flush();

            dbg!("made first db");
            main.updatedb();
            dbg!("made update db");
            main.load_table(&sharedtypes::LoadDBTable::Settings);
            dbg!("made loaded table db");
            main.load_caching();
            dbg!("cached  db");
        } else {
            // Database does exist.
            logging::log(format!(
                "Database Exists: {} : Skipping creation.",
                first_time_load_flag
            ));
            main.load_table(&sharedtypes::LoadDBTable::Settings);
            main.load_caching();
        }
        main.transaction_flush();
        main
    }

    /// Clears in memdb structures
    pub fn clear_cache(&self) {
        match self._cache {
            CacheType::Bare => {}
            _ => {
                self._inmemdb.write().clear_all();
            }
        }
    }

    ///
    /// Starts an exclusive write transaction
    ///
    pub(super) fn transaction_exclusive_start(&self) {
        self.transaction_flush();

        let mut transaction = self.write_conn_istransaction.lock();
        if !*transaction {
            *transaction = true;
            self.write_conn
                .lock()
                .execute("BEGIN EXCLUSIVE TRANSACTION", [])
                .unwrap();
        }
    }
    ///
    /// Starts an exclusive write transaction
    ///
    pub(super) fn transaction_start(&self) {
        self.transaction_flush();

        let mut transaction = self.write_conn_istransaction.lock();
        if !*transaction {
            *transaction = true;
            self.write_conn
                .lock()
                .execute("BEGIN TRANSACTION", [])
                .unwrap();
        }
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

    ///
    /// Gets one tnection from the sqlite pool
    ///
    pub fn get_database_connection(&self) -> PooledConnection<SqliteConnectionManager> {
        loop {
            match self.pool.get() {
                Ok(out) => {
                    return out;
                }
                Err(_) => {
                    dbg!("a");
                }
            }
        }
    }

    ///
    /// Loads the cache configuration
    ///
    pub fn load_caching(&mut self) {
        let temp;

        loop {
            let cache = match self.settings_get_name(&"dbcachemode".into()) {
                None => {
                    self.setup_default_cache();
                    self.settings_get_name(&"dbcachemode".into())
                        .unwrap()
                        .param
                        .clone()
                }
                Some(setting) => setting.param.clone(),
            };

            if let Some(ref cache) = cache {
                let cachemode = match cache.as_str() {
                    "Bare" => Some(CacheType::Bare),
                    "InMemdb" => Some(CacheType::InMemdb),
                    "InMemory" => {
                        let manager = SqliteConnectionManager::memory();

                        let pool = r2d2::Builder::new()
                            .idle_timeout(Some(Duration::from_secs(5)))
                            .max_size(1)
                            .build(manager)
                            .unwrap();

                        Some(CacheType::InMemory(pool))
                    }

                    _ => {
                        self.setup_default_cache();
                        None
                    }
                };
                if let Some(cachemode) = cachemode {
                    temp = cachemode;
                    break;
                }
            } else {
                self.setup_default_cache();
            }
        }

        self._cache = temp
    }

    ///
    /// Sets up the default location to dump misbehaving files
    ///
    pub fn setup_defaut_misplaced_location(&self) -> String {
        let mut conn = self.get_database_connection();
        let loc = self.location_get();
        let outpath = Path::new(&loc);
        let out = outpath.parent().unwrap().join(Path::new("Files-Dump"));
        file::folder_make(&out.to_string_lossy().to_string());
        out.to_string_lossy().to_string()
    }

    ///
    /// Checks the relationships table for any dead tagids
    /// Only cleans the tags where they are linked to a fileid
    ///
    pub fn check_relationship_tag_relations(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Files);
        self.load_table(&sharedtypes::LoadDBTable::Relationship);
        self.load_table(&sharedtypes::LoadDBTable::Tags);
        let mut flag = false;
        logging::info_log("Relationship-Tag-Relations checker starting check");

        for fid in self.file_get_list_id().iter() {
            for tid in self.relationship_get_tagid(fid).iter() {
                if self.tag_id_get(tid).is_none() {
                    flag = true;
                    logging::log(format!(
                        "Relationship-Tag-Relations checker found bad tid {tid} relating to fid: {fid} deleting empty relationship"
                    ));
                    self.relationship_remove(fid, tid);
                }
            }
        }

        if flag {
            let mut write_conn = self.get_database_connection();
            let mut tn = write_conn.transaction().unwrap();
            logging::info_log("Relationship-Tag-Relations checker condensing tags");
            self.condense_tags(&mut tn);
            tn.commit().unwrap();
            self.vacuum();
        }
        logging::info_log("Relationship-Tag-Relations checker ending check");
        self.transaction_flush();
    }

    ///
    /// Adds a dead url into the db
    ///
    pub fn add_dead_url(&self, url: &String) {
        self.add_dead_url_sql(url);
        match self._cache {
            CacheType::Bare => {}
            _ => {
                self._inmemdb.write().add_dead_source_url(url.to_string());
            }
        }
    }

    /// Adds the job to the inmemdb
    pub fn jobs_add_new_todb(&self, job: sharedtypes::DbJobsObj) {
        match self._cache {
            /*CacheType::Bare => {
                self.jobs_add_sql( &job);
            }*/
            _ => {
                self._inmemdb.write().jobref_new(job);
            }
        }
    }

    ///
    /// Flips the running of a job by id
    /// Returns the status of the job if it exists
    ///
    pub fn jobs_flip_running(&self, id: &usize) -> Option<bool> {
        match self._cache {
            CacheType::Bare => {
                todo!("Bare implementation not implemineted");
            }
            _ => self._inmemdb.write().jobref_flip_isrunning(id),
        }
    }

    /// File Sanity Checker This will check that the files by id will have a matching
    /// location & hash.
    pub fn db_sanity_check_file(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Files);
        todo!("Need to fix this files are sucky");
        let flist = self.file_get_list_id();
        /*flist.par_iter().for_each(|feach| {
            // Check is needed to support if the file with nonexistant info was gotten from db
            if let Some(filestorage_obj) = self.file_get_id( feach) {
                let fileinfo = match filestorage_obj {
                    sharedtypes::DbFileStorage::Exist(file) => file,
                    _ => {
                        panic!("Pulled item that shouldnt exist: {:?}", filestorage_obj);
                    }
                };
                loop {
                    let location = self.storage_get_string( &fileinfo.storage_id).unwrap();
                    let temppath = &format!("{}/{}", location, fileinfo.hash);
                    if Path::new(temppath).exists() {
                        let fil = std::fs::read(temppath).unwrap();
                        let hinfo = download::hash_bytes(
                            &bytes::Bytes::from(fil),
                            &sharedtypes::HashesSupported::Sha512(fileinfo.hash.clone()),
                        );
                        if !hinfo.1 {
                            dbg!(format!(
                                "BAD HASH: ID: {}  HASH: {}   2ND HASH: {}",
                                fileinfo.id, fileinfo.hash, hinfo.0
                            ));
                        }
                    }
                }
            } else {
                dbg!(format!("File ID: {} Does not exist.", &feach));
            }
        });*/
    }

    ///
    ///Returns the max id of something inside of the db
    ///
    pub fn tags_max_id(&self) -> usize {
        match self._cache {
            CacheType::Bare => self.tags_max_return_sql(),
            _ => self._inmemdb.read().tags_max_return(),
        }
    }

    /// Returns next jobid from _inmemdb
    pub fn jobs_get_max(&self) -> usize {
        match self._cache {
            //  CacheType::Bare => self.jobs_return_count_sql(),
            _ => *self._inmemdb.read().jobs_get_max(),
        }
    }

    /// Sets up first database interaction. Makes tables and does first time setup.
    pub fn first_db(&self, tn: &mut Transaction) {
        {
            // Making Tags Table
            let mut name = "Tags".to_string();
            let mut keys = vec_of_strings!["id", "name", "namespace"];
            let mut vals = vec_of_strings![
                "INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL ",
                "TEXT NOT NULL",
                "INTEGER NOT NULL, UNIQUE(name, namespace)"
            ];
            self.table_create(tn, &name, &keys, &vals);

            {
                tn.execute(
                    "  CREATE UNIQUE INDEX IF NOT EXISTS tags_name_namespace_idx
        ON Tags(name, namespace);",
                    [],
                )
                .unwrap();
            }

            // Making Parents Table. Relates tags to tag parents.
            self.parents_create_v2(tn);

            // Making Namespace Table
            name = "Namespace".to_string();
            keys = vec_of_strings!["id", "name", "description"];
            vals = vec_of_strings![
                "INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL",
                "TEXT NOT NULL UNIQUE",
                "TEXT"
            ];
            self.table_create(tn, &name, &keys, &vals);

            // Making Settings Table
            name = "Settings".to_string();
            keys = vec_of_strings!["name", "pretty", "num", "param"];
            vals = vec_of_strings!["TEXT PRIMARY KEY", "TEXT", "INTEGER", "TEXT"];
            self.table_create(tn, &name, &keys, &vals);

            // Making Jobs Table
            name = "Jobs".to_string();
            keys = vec_of_strings!(
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
            vals = vec_of_strings!(
                "INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL",
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

            self.table_create(tn, &name, &keys, &vals);

            {
                tn.execute(
                "CREATE TABLE File 
            (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, hash TEXT, extension INTEGER, storage_id INTEGER, 
                CHECK (
                    (hash IS NOT NULL AND extension IS NOT NULL) OR
                    (hash IS NULL AND extension IS NULL)
                )
            )",
                [],
            )
            .unwrap();

                self.relationship_create_v2(tn);

                tn.execute("CREATE INDEX idx_namespace ON Namespace (name)", [])
                    .unwrap();
            }

            self.enclave_create_database_v5(tn);

            // Making dead urls Table
            name = "dead_source_urls".to_string();
            keys = vec_of_strings!["id", "dead_url"];
            vals = vec_of_strings!["INTEGER PRIMARY KEY", "TEXT NOT NULL"];
            self.table_create(tn, &name, &keys, &vals);
            {
                tn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_file_hash ON File (hash)",
                    [],
                )
                .unwrap();
            }
        }
        self.tag_count_create_v1(tn);
        self.tags_fts_create_v1(tn);
        self.namespace_properties_create_v1(tn);
    }

    pub fn updatedb(&self) {
        self.transaction_start();
        self.setting_add(
            "DBCOMMITNUM".to_string(),
            Some("Number of transactional items before pushing to db.".to_string()),
            Some(3000),
            None,
        );
        self.setting_add(
            "VERSION".to_string(),
            Some("Version that the database is currently on.".to_string()),
            Some(self._vers),
            None,
        );
        self.setting_add("DEFAULTRATELIMIT".to_string(), None, Some(5), None);
        self.setting_add(
            "FilesLoc".to_string(),
            None,
            None,
            Some("Files".to_string()),
        );
        self.setting_add(
            "DEFAULTUSERAGENT".to_string(),
            None,
            None,
            Some("DIYHydrus/1.0".to_string()),
        );
        self.setting_add(
            "pluginloadloc".to_string(),
            Some("Where plugins get loaded into.".to_string()),
            None,
            Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
        );

        self.setting_add(
            "scraperloadloc".to_string(),
            Some("Where scrapers get loaded into.".to_string()),
            None,
            Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
        );

        self.setup_default_cache();

        self.enclave_create_default_file_download(self.location_get());

        self.create_default_source_url_ns_id();
    }

    ///
    /// Default caching option for the db
    ///
    fn setup_default_cache(&self) {
        self.setting_add(
            "dbcachemode".to_string(),
            Some("The database caching options. Supports: Bare, InMemdb and InMemory".to_string()),
            None,
            Some("Bare".to_string()),
        );
    }

    ///
    /// Gets a scraper folder. If it doesn't exist then please create it in db
    ///
    pub fn loaded_scraper_folder(&self) -> PathBuf {
        match self.settings_get_name(&"scraperloadloc".to_string()) {
            Some(setting) => {
                if let Some(param) = &setting.param {
                    Path::new(param).to_path_buf()
                } else {
                    self.setting_add(
                        "scraperloadloc".to_string(),
                        Some("Where scrapers get loaded into.".to_string()),
                        None,
                        Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
                    );
                    Path::new(crate::DEFAULT_LOC_SCRAPER).to_path_buf()
                }
            }
            None => {
                self.setting_add(
                    "scraperloadloc".to_string(),
                    Some("Where scrapers get loaded into.".to_string()),
                    None,
                    Some(crate::DEFAULT_LOC_SCRAPER.to_string()),
                );
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
                    self.setting_add(
                        "pluginloadloc".to_string(),
                        Some("Where plugins get loaded into.".to_string()),
                        None,
                        Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
                    );
                    Path::new(crate::DEFAULT_LOC_PLUGIN).to_path_buf()
                }
            }
            None => {
                self.setting_add(
                    "pluginloadloc".to_string(),
                    Some("Where plugins get loaded into.".to_string()),
                    None,
                    Some(crate::DEFAULT_LOC_PLUGIN.to_string()),
                );
                Path::new(crate::DEFAULT_LOC_PLUGIN).to_path_buf()
            }
        }
    }

    pub fn check_default_source_urls(&self, action: &sharedtypes::CheckSourceUrlsEnum) {
        self.load_table(&sharedtypes::LoadDBTable::Namespace);
        self.load_table(&sharedtypes::LoadDBTable::Tags);
        let source_nsid = self.create_default_source_url_ns_id();

        for tag_id in self.namespace_get_tagids(&source_nsid).iter() {
            if let Some(tag) = self.tag_id_get(tag_id) {
                if !check_url(&tag.name) {
                    match action {
                        sharedtypes::CheckSourceUrlsEnum::Print => {
                            logging::info_log(format!(
                                "Tagid - {} name - {} doesn't look like a valid url",
                                tag_id, &tag.name
                            ));
                        }
                        sharedtypes::CheckSourceUrlsEnum::Delete => {
                            self.load_table(&sharedtypes::LoadDBTable::All);
                            self.tag_remove(tag_id);
                        }
                    }
                }
            }
        }
    }
    ///
    /// Gets a default namespace id if it doesn't exist
    ///
    pub fn create_default_source_url_ns_id(&self) -> usize {
        match self.namespace_get(&"source_url".to_string()) {
            None => self.namespace_add(
                &"source_url".to_string(),
                &Some("Source URL for a file.".to_string()),
            ),
            Some(id) => id,
        }
    }

    /// Creates a table name: The table name key: List of Collumn lables. dtype: List
    /// of Collumn types. NOTE Passed into SQLITE DIRECTLY THIS IS BAD :C
    pub fn table_create(
        &self,
        tn: &mut Transaction,
        name: &String,
        key: &[String],
        dtype: &[String],
    ) {
        // Sanity checking...
        assert_eq!(
            key.len(),
            dtype.len(),
            "Warning table create was 2 Vecs weren't balanced. Lengths: {} {}",
            key.len(),
            dtype.len()
        );

        // Not sure if theirs a better way to dynamically allocate a string based on two
        // vec strings at run time. Let me know if im doing something stupid.
        let mut concat = true;
        let mut c = 0;
        let mut stocat = "".to_string();
        while concat {
            let ke = &key[c];
            let dt = &dtype[c];
            stocat = [
                stocat,
                ke.to_string(),
                " ".to_string(),
                dt.to_string(),
                ", ".to_string(),
            ]
            .concat();
            c += 1;
            if c >= key.len() - 1 {
                concat = false;
            }
        }
        let ke = &key[key.len() - 1];
        let dt = &dtype[dtype.len() - 1];
        let endresult = [
            "CREATE TABLE IF NOT EXISTS ".to_string(),
            name.to_string(),
            " (".to_string(),
            stocat,
            ke.to_string(),
            " ".to_string(),
            dt.to_string(),
            ");".to_string(),
        ]
        .concat();
        dbg!(&endresult);
        info!("Creating table as: {}", endresult);
        tn.execute_batch(&endresult).unwrap();
        //stocat = endresult;
        //self.execute(stocat);
    }

    /// Alters a tables name
    pub(super) fn alter_table(&self, original_table: &String, new_table: &String) {
        self.transaction_start();
        let tn = self.write_conn.lock();
        tn.execute(
            "ALTER TABLE ? RENAME TO ?",
            params![original_table, new_table],
        )
        .unwrap();
    }

    /// Checks if table exists in DB if it do then delete.
    pub(super) fn check_table_exists(&self, table: String) -> bool {
        let mut out = false;
        let query_string = "SELECT * FROM sqlite_master WHERE type='table' AND name=? ;";
        dbg!("setup");
        let tn = self.get_database_connection();
        dbg!("connection");
        dbg!(&self._cache);
        dbg!(&table);
        dbg!(self.pool.state());
        //panic!();
        let mut toexec = tn.prepare(query_string).unwrap();
        dbg!("query");
        let mut rows = toexec.query(params![table]).unwrap();
        dbg!("rows");
        if let Some(_each) = rows.next().unwrap() {
            out = true;
        }
        out
    }

    /// Gets the names of a collumn in a table
    pub(super) fn db_table_collumn_getnames(&self, table: &String) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        {
            let tn = self.get_database_connection();
            let stmt = tn
                .prepare(&format!("SELECT * FROM {} LIMIT 1", table))
                .unwrap();
            for collumn in stmt.column_names() {
                out.push(collumn.to_owned());
            }
        }
        out
    }

    /// Sets DB Version
    pub(super) fn db_version_set(&mut self, version: usize) {
        logging::log(format!("Setting DB Version to: {}", &version));
        self._active_vers = version;
        self.setting_add(
            "VERSION".to_string(),
            Some("Version that the database is currently on.".to_string()),
            Some(version),
            None,
        );
    }

    pub(super) fn db_drop_table(&self, table: &String) {
        self.transaction_exclusive_start();
        let tn = self.write_conn.lock();
        let query_string = format!("DROP TABLE IF EXISTS {};", table);
        let mut toexec = tn.prepare(&query_string).unwrap();
        toexec.execute(params![]).unwrap();
    }

    /// Checks if db version is consistent. If this function returns false signifies
    /// that we shouldn't run.
    pub fn check_version(&mut self) -> bool {
        let mut query_string = "SELECT num FROM Settings WHERE name='VERSION';";
        let query_string_manual = "SELECT num FROM Settings_Old WHERE name='VERSION';";
        let mut g1 = self.quer_int(query_string.to_string());
        if g1.len() != 1 {
            error!(
                "Could not check_version due to length of recieved version being less then one. Trying manually!!!"
            );

            // let out = self.execute("SELECT num from Settings WHERE
            // name='VERSION';".to_string());
            let tn = self.get_database_connection();
            let mut toexec = tn.prepare(query_string).unwrap();
            let mut rows = toexec.query(params![]).unwrap();
            g1.clear();
            while let Some(each) = rows.next().unwrap() {
                let ver: Result<String> = each.get(0);
                let vers: Result<usize> = each.get(0);

                // let izce;
                let izce = match &ver {
                    Ok(_string_ver) => ver.unwrap().parse::<usize>().unwrap(),
                    Err(_unk_err) => vers.unwrap(),
                };
                g1.push(izce.try_into().unwrap())
            }
        }
        if g1.len() != 1 {
            error!("Manual loading failed. Trying from old table.");
            println!("Manual loading failed. Trying from old table.");
            query_string = query_string_manual;
            let tn = self.get_database_connection();
            let mut toexec = tn.prepare(query_string).unwrap();
            let mut rows = toexec.query(params![]).unwrap();
            g1.clear();
            while let Some(each) = rows.next().unwrap() {
                let ver: String = each.get(0).unwrap();

                // let vers = ver.try_into().unwrap();
                let izce = ver.parse().unwrap();
                g1.push(izce)
            }
            logging::panic_log("check_version: Could not load DB properly PANICING!!!".to_string());
        }
        let mut db_vers = g1[0] as usize;
        self._active_vers = db_vers;
        logging::info_log(format!("check_version: Loaded version {}", db_vers));
        if self._active_vers != self._vers {
            let mut conn = self.get_database_connection();
            conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            logging::info_log(format!(
                "Starting upgrade from V{} to V{}",
                db_vers,
                db_vers + 1
            ));

            // Resets the DB internal Cache if their is any.
            self.clear_cache();
            self.load_table(&sharedtypes::LoadDBTable::Settings);

            self.transaction_flush();
            if db_vers == 1 {
                panic!("How did you get here vers is 1 did you do something dumb??")
            } else if db_vers == 2 {
                dbg!(self._vers, self._active_vers);
                self.db_update_two_to_three();
                db_vers += 1;
            } else if db_vers == 3 {
                self.db_update_three_to_four();
            } else if db_vers == 4 {
                self.db_update_four_to_five();
            } else if db_vers == 5 {
                self.db_update_five_to_six();
            } else if db_vers == 6 {
                self.db_update_six_to_seven();
            } else if db_vers == 7 {
                self.db_update_seven_to_eight();
            } else if db_vers == 8 {
                self.db_update_eight_to_nine();
            } else if db_vers == 9 {
                self.db_update_nine_to_ten();
            }

            logging::info_log(format!("Finished upgrade to V{}.", db_vers));
            self.transaction_flush();
            if db_vers == self._vers {
                logging::info_log(format!("Successfully updated db to version {}", self._vers));
                return true;
            }
        } else {
            info!("Database Version is: {}", g1[0]);
            return true;
        }
        false
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

    /// Adds file into Memdb instance.
    pub fn file_add_db(&self, file: sharedtypes::DbFileStorage) -> usize {
        match self._cache {
            CacheType::Bare => self.file_add_sql(&file),
            _ => {
                self.file_add_sql(&file);

                self._inmemdb.write().file_put(file)
            }
        }
    }

    ///
    /// Adds all tags to a fileid
    /// If theirs no fileid then it just adds the tag
    ///
    pub fn add_tags_to_fileid(
        &self,
        file_id: Option<usize>,
        tag_actions: &[sharedtypes::FileTagAction],
    ) {
        for tag_action in tag_actions {
            match tag_action.operation {
                sharedtypes::TagOperation::Add => {
                    // Simple add operation
                    for tag in &tag_action.tags {
                        if matches!(
                            tag.tag_type,
                            sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex
                        ) {
                            if let Some(tag_id) = self.tag_add_tagobject(tag) {
                                if let Some(file_id) = file_id {
                                    self.relationship_add(file_id, tag_id);
                                }
                            }
                        }
                    }
                }

                sharedtypes::TagOperation::Set => {
                    // We need a valid file_id for Set
                    let file_id = match file_id {
                        Some(fid) => fid,
                        None => return,
                    };

                    // 1️⃣ Build parser tags grouped by namespace
                    let mut namespace_tags: HashMap<String, Vec<usize>> = HashMap::new();
                    for tag in &tag_action.tags {
                        // Ignore special tag types
                        if !matches!(
                            tag.tag_type,
                            sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex
                        ) {
                            continue;
                        }

                        if let Some(tag_id) = self.tag_add_tagobject(tag) {
                            namespace_tags
                                .entry(tag.namespace.name.clone())
                                .or_insert_with(Vec::new)
                                .push(tag_id);
                        }
                    }

                    // 2️⃣ Fetch current tags for file grouped by namespace
                    let mut namespace_file_tags: HashMap<String, Vec<usize>> = HashMap::new();
                    for tag_id in self.relationship_get_tagid(&file_id) {
                        if let Some(tag_obj) = self.tag_id_get(&tag_id) {
                            if let Some(namespace) = self.namespace_get_string(&tag_obj.namespace) {
                                namespace_file_tags
                                    .entry(namespace.name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(tag_id);
                            }
                        }
                    }

                    // 3️⃣ Remove ignored namespaces
                    for ignored in ["source_url", ""] {
                        namespace_tags.remove(ignored);
                        namespace_file_tags.remove(ignored);
                    }

                    // 4️⃣ Synchronize tags
                    for (namespace, parser_tags) in &namespace_tags {
                        let file_tags = namespace_file_tags.get_mut(namespace);

                        // Convert parser_tags to a HashSet for efficient lookup
                        let parser_tag_set: HashSet<_> = parser_tags.iter().copied().collect();

                        match file_tags {
                            Some(file_tags) => {
                                // a) Add new tags from parser that aren't already in file
                                for &tag_id in parser_tags {
                                    if !file_tags.contains(&tag_id) {
                                        logging::log(format!(
                                            "Adding tag_id {} to file_id {} per scraper",
                                            tag_id, file_id
                                        ));
                                        self.relationship_add(file_id, tag_id);
                                        file_tags.push(tag_id); // Update in-memory vector
                                    }
                                }

                                // b) Remove tags from file that are no longer in parser
                                let to_remove: Vec<_> = file_tags
                                    .iter()
                                    .filter(|&&tag_id| !parser_tag_set.contains(&tag_id))
                                    .copied()
                                    .collect();

                                for tag_id in to_remove {
                                    logging::log(format!(
                                        "Removing tag_id {} from file_id {} per scraper",
                                        tag_id, file_id
                                    ));
                                    self.relationship_remove(&file_id, &tag_id);
                                    file_tags.retain(|&id| id != tag_id); // Update in-memory vector
                                }
                            }

                            None => {
                                // Namespace doesn't exist for this file, just add all parser tags
                                for &tag_id in parser_tags {
                                    logging::log(format!(
                                        "Adding tag_id {} to file_id {} per scraper",
                                        tag_id, file_id
                                    ));
                                    self.relationship_add(file_id, tag_id);
                                }
                            }
                        }
                    }
                }

                sharedtypes::TagOperation::Del => {
                    let file_id = match file_id {
                        Some(fid) => fid,
                        None => return,
                    };
                    for tag in tag_action.tags.iter() {
                        if let Some(ns_id) = self.namespace_get(&tag.namespace.name) {
                            if let Some(tag_id) = self.tag_get_name(tag.tag.clone(), ns_id) {
                                logging::log(format!(
                                    "Removing tag_id {} to file_id {} per scraper",
                                    tag_id, file_id
                                ));

                                self.relationship_remove(&file_id, &tag_id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// NOTE USES PASSED tnECTION FROM FUNCTION NOT THE DB CONNECTION GETS ARROUND
    /// MEMROY SAFETY ISSUES WITH CLASSES IN RUST
    fn load_files(&self) {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }
        self.load_extensions();

        logging::info_log("Database is Loading: Files".to_string());
        let tn = self.get_database_connection();
        let temp = tn.prepare("SELECT * FROM File");
        if let Ok(mut con) = temp {
            let files = con
                .query_map([], |row| {
                    let id: Option<usize> = row.get(0).unwrap();
                    let hash: Option<String> = row.get(1).unwrap();
                    let ext: Option<usize> = row.get(2).unwrap();
                    let location: Option<usize> = row.get(3).unwrap();
                    if id.is_some() && hash.is_some() && ext.is_some() && location.is_some() {
                        Ok(sharedtypes::DbFileStorage::Exist(sharedtypes::DbFileObj {
                            id: row.get(0).unwrap(),
                            hash: row.get(1).unwrap(),
                            ext_id: row.get(2).unwrap(),
                            storage_id: row.get(3).unwrap(),
                        }))
                    } else if id.is_some() && hash.is_none() && ext.is_none() && location.is_none()
                    {
                        Ok(sharedtypes::DbFileStorage::NoExist(id.unwrap()))
                    } else {
                        panic!("Error on: {:?} {:?} {:?} {:?}", id, hash, ext, location);
                    }
                })
                .unwrap();
            for each in files {
                if let Ok(res) = each {
                    self.file_add_db(res);
                } else {
                    error!("Bad File cant load {:?}", each);
                }
            }
        }
    }

    ///
    /// Gets an extension id and creates it if it does not exist
    ///
    pub fn extension_put_string(&self, ext: &String) -> usize {
        match self.extension_get_id(ext) {
            Some(id) => id,
            None => self.extension_put_id_ext_sql(None, ext),
        }
    }

    /// Puts extension into mem cache
    pub fn extension_load(&self, id: usize, ext: String) {
        match self._cache {
            CacheType::Bare => {
                self.extension_put_id_ext_sql(Some(id), &ext);
            }
            _ => self._inmemdb.write().extension_load(id, ext),
        }
    }

    /// Loads extensions into db
    fn load_extensions(&self) {
        logging::info_log("Database is Loading: File Extensions".to_string());
        let tn = self.get_database_connection();
        let temp = tn.prepare("SELECT * FROM FileExtensions");

        if let Ok(mut con) = temp {
            let quer = con.query([]);
            if let Ok(mut rows) = quer {
                while let Ok(Some(row)) = rows.next() {
                    let id: Option<usize> = row.get(0).unwrap();
                    let extension: Option<String> = row.get(1).unwrap();

                    if let Some(ext) = extension
                        && let Some(id) = id
                    {
                        self.extension_load(id, ext);
                    }
                }
            }
        }
    }

    /// Same as above
    fn load_namespace(&self) {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }

        let mut nses: Vec<sharedtypes::DbNamespaceObj> = vec![];
        logging::info_log("Database is Loading: Namespace".to_string());
        {
            let tn = self.get_database_connection();
            let temp = tn.prepare("SELECT * FROM Namespace");
            if let Ok(mut con) = temp {
                let namespaces = con
                    .query_map([], |row| {
                        Ok(sharedtypes::DbNamespaceObj {
                            id: row.get(0).unwrap(),
                            name: row.get(1).unwrap(),
                            description: row.get(2).unwrap(),
                        })
                    })
                    .unwrap();
                for each in namespaces {
                    if let Ok(res) = each {
                        nses.push(res);
                    } else {
                        error!("Bad Namespace cant load {:?}", each);
                    }
                }
            }
        }

        for ns in nses {
            self.namespace_add_id_exists(ns);
            //self.namespace_add(res.name, res.description);
        }
    }

    /// Loads jobs in from DB tnection
    fn load_jobs(&self) {
        logging::info_log("Database is Loading: Jobs".to_string());
        let tn = self.get_database_connection();
        let temp = tn.prepare("SELECT * FROM Jobs");
        if let Ok(mut con) = temp {
            let jobs = con
                .query_map([], |row| {
                    let id = row.get(0).unwrap();
                    let time = row.get(1).unwrap();
                    let reptime = row.get(2).unwrap();
                    let priority = row.get(3).unwrap_or(sharedtypes::DEFAULT_PRIORITY);
                    let cachetime = row.get(4).unwrap_or_default();
                    //let cachechecktype: String = row.get(5).unwrap();
                    let cachechecktype = "TimeReptimeParam".to_string();
                    let manager: String = row.get(6).unwrap();
                    let man = serde_json::from_str(&manager).unwrap();
                    let site = row.get(7).unwrap();
                    let param: String = row.get(8).unwrap();
                    let system_data_string: String = row.get(9).unwrap();
                    let user_data_string: String = row.get(10).unwrap();
                    let system_data = serde_json::from_str(&system_data_string).unwrap();
                    let user_data = serde_json::from_str(&user_data_string).unwrap();
                    Ok(sharedtypes::DbJobsObj {
                        id,
                        time,
                        reptime,
                        priority,
                        cachetime,
                        cachechecktype: DEFAULT_CACHECHECK,
                        //cachechecktype: serde_json::from_str(&cachechecktype).unwrap(),
                        site,
                        param: serde_json::from_str(&param).unwrap(),
                        jobmanager: man,
                        isrunning: false,
                        user_data,
                        system_data,
                    })
                })
                .unwrap();
            for each in jobs {
                if let Ok(res) = each {
                    self.jobs_add_new_todb(res);
                } else {
                    error!("Bad Job cant load {:?}", each);
                }
            }
        }
    }

    /// Adds a file into the db sqlite. Do this first.
    pub fn file_add(&self, file: sharedtypes::DbFileStorage) -> usize {
        match file {
            sharedtypes::DbFileStorage::Exist(ref file_obj) => {
                if self.file_get_hash(&file_obj.hash).is_none() {
                    let id = self.file_add_db(file.clone());
                    id
                } else {
                    file_obj.id
                }
            }
            sharedtypes::DbFileStorage::NoIdExist(ref noid_obj) => {
                if self.file_get_hash(&noid_obj.hash).is_none() {
                    let id = self.file_add_db(file.clone());

                    id
                } else {
                    self.file_get_hash(&noid_obj.hash).unwrap()
                }
            }
            sharedtypes::DbFileStorage::NoExist(_) => {
                panic!();
                /*let id = self.file_add_db(file.clone());
                if addtodb {
                    self.file_add_sql(&None, &None, &None, &id);
                }
                id*/
            }
            sharedtypes::DbFileStorage::NoExistUnknown => {
                panic!();
                /*let id = self.file_add_db(file.clone());
                if addtodb {
                    self.file_add_sql(&None, &None, &None, &id);
                }
                id*/
            }
        }
    }

    ///
    /// Adds a namespace into the db if it may or may not exist
    ///
    pub fn namespace_add(&self, name: &String, description: &Option<String>) -> usize {
        if let Some(id) = self.namespace_get(name) {
            return id;
        }

        match self._cache {
            CacheType::Bare => {
                self.namespace_add_sql(name, description, None);
                self.transaction_flush();
                self.namespace_get(name).unwrap()
            }
            _ => self.namespace_add_inmemdb(name.clone(), description.clone()),
        }
    }

    ///
    /// Adds a ns into the db if the id already exists
    ///
    pub fn namespace_add_id_exists(&self, ns: sharedtypes::DbNamespaceObj) -> usize {
        match self._cache {
            CacheType::Bare => {
                self.namespace_add_sql(&ns.name, &ns.description, Some(ns.id));

                self.namespace_get(&ns.name).unwrap()
            }
            _ => self.namespace_add_inmemdb(ns.name, ns.description),
        }
    }

    /// Adds namespace into DB. Returns the ID of the namespace.
    pub(super) fn namespace_add_inmemdb(&self, name: String, description: Option<String>) -> usize {
        let namespace_grab = {
            let inmemdb = self._inmemdb.read();
            inmemdb.namespace_get(&name).copied()
        };
        match namespace_grab {
            None => {}
            Some(id) => return id.to_owned(),
        }

        let ns_id = self._inmemdb.read().namespace_get_max();
        let ns = sharedtypes::DbNamespaceObj {
            id: ns_id,
            name,
            description,
        };
        if namespace_grab.is_none() {
            self.namespace_add_sql(&ns.name, &ns.description, Some(ns_id));
        }
        //self.namespace_add_db(ns)
        self._inmemdb.write().namespace_put(ns)
    }

    /// Wrapper for inmemdb adding
    pub(super) fn parents_add_db(&self, parent: sharedtypes::DbParentsObj) -> usize {
        self._inmemdb.write().parents_put(parent)
    }

    /// Wrapper for inmemdb and parents_add_db
    pub fn parents_add(&self, par: sharedtypes::DbParentsObj) -> usize {
        match self._cache {
            CacheType::Bare => {
                let tagid = self.parents_get_id_list_sql(&par);

                if tagid.is_empty() {
                    self.parents_add_sql(&par)
                } else {
                    let tags: Vec<usize> = tagid.into_iter().collect();
                    tags[0]
                }
            }
            _ => {
                let inmemdb = self._inmemdb.read();
                let parent = inmemdb.parents_get(&par);
                if parent.is_none() {
                    self.parents_add_sql(&par);
                }
                self.parents_add_db(par)
            }
        }
    }

    /// Adds tag into inmemdb
    fn tag_add_db(&self, tag: &String, namespace: &usize, id: Option<usize>) -> usize {
        match id {
            None => {
                match self._inmemdb.read().tags_get_id(&sharedtypes::DbTagNNS {
                    name: tag.to_string(),
                    namespace: namespace.to_owned(),
                }) {
                    None => {
                        let tag_info = sharedtypes::DbTagNNS {
                            name: tag.to_string(),
                            namespace: *namespace,
                        };
                        self._inmemdb.write().tags_put(&tag_info, id)
                    }
                    Some(tag_id_max) => *tag_id_max,
                }
            }
            Some(out) => {
                let tag_info = sharedtypes::DbTagNNS {
                    name: tag.to_string(),
                    namespace: *namespace,
                };
                self._inmemdb.write().tags_put(&tag_info, Some(out))
            }
        }
    }

    /// Prints db info
    pub fn debugdb(&self) {
        self._inmemdb.read().dumpe_data();
    }

    pub fn setup_globalload(&mut self, globalload: GlobalLoad) {
        self.globalload = Some(globalload.into());
    }

    pub fn namespace_add_namespaceobject(
        &self,

        namespace_obj: sharedtypes::GenericNamespaceObj,
    ) -> usize {
        self.namespace_add(&namespace_obj.name, &namespace_obj.description)
    }

    ///
    /// Adds tags to a file id
    ///
    pub fn relationship_tag_add(&self, fid: usize, tags: Vec<sharedtypes::TagObject>) {
        match self._cache {
            CacheType::Bare => {
                self.file_tag_relationship(&fid, tags);
            }
            _ => {
                self.transaction_exclusive_start();
                for tag in tags.iter() {
                    if let Some(tid) = self.tag_add_tagobject(tag) {
                        self.relationship_add(fid, tid);
                    }
                }
                self.transaction_flush();
            }
        }
    }

    /// Runs regex mostly when a tag gets added
    pub fn tag_run_on_tag(&self, tag: &sharedtypes::TagObject) {
        if tag.tag_type != sharedtypes::TagType::NormalNoRegex {
            if let Some(ref globalload) = self.globalload {
                globalload.plugin_on_tag(tag);
            }
        }
    }

    ///
    /// More modern way to add a file into the db
    ///
    pub fn tag_add_tagobject(&self, tag: &sharedtypes::TagObject) -> Option<usize> {
        let mut limit_to = None;

        // If the tag is empty then dont push.
        // this is a stupid check but I don't trust myself
        if tag.tag.is_empty() || tag.namespace.name.is_empty() {
            return None;
        }

        self.tag_run_on_tag(tag);
        let nsid = self.namespace_add_namespaceobject(tag.namespace.clone());
        let tag_id = self.tag_add(&tag.tag, nsid, None);

        // Parent tag adding
        if let Some(subtag) = &tag.relates_to {
            if subtag.tag.is_empty() || subtag.namespace.name.is_empty() {
                return None;
            }

            let nsid = self.namespace_add_namespaceobject(subtag.namespace.clone());
            let relate_tag_id = self.tag_add(&subtag.tag, nsid, None);
            if let Some(limitto) = &subtag.limit_to {
                if limitto.tag.is_empty() || limitto.namespace.name.is_empty() {
                    return None;
                }

                let nsid = self.namespace_add_namespaceobject(limitto.namespace.clone());
                limit_to = Some(self.tag_add(&limitto.tag, nsid, None));
            }
            let par = sharedtypes::DbParentsObj {
                tag_id,
                relate_tag_id,
                limit_to,
            };

            self.parents_add(par);
        }
        Some(tag_id)
    }

    /// Adds tag into DB if it doesn't exist in the memdb.
    pub fn tag_add(&self, tags: &String, namespace: usize, id: Option<usize>) -> usize {
        match self._cache {
            CacheType::Bare => {
                let tag = self.tag_get_name(tags.to_string(), namespace);
                match tag {
                    None => self.tag_add_no_id_sql(tags, namespace),
                    Some(id) => id,
                }
            }
            _ => {
                let tag_id = match id {
                    None => self.tags_max_id(),
                    Some(id) => id,
                };

                let tagnns = sharedtypes::DbTagNNS {
                    name: tags.to_string(),
                    namespace,
                };
                let tags_grab = self._inmemdb.read().tags_get_id(&tagnns).copied();
                match tags_grab {
                    None => {
                        let tag_id = self.tag_add_db(tags, &namespace, id);
                        self.tag_add_sql(&tag_id, tags, &namespace);
                        self.transaction_flush();
                        tag_id
                    }
                    Some(tag_id) => tag_id,
                }
            }
        }
    }

    /// Wrapper for inmemdb relationship adding
    pub(super) fn relationship_add_db(&self, file: usize, tag: usize) {
        self._inmemdb.write().relationship_add(file, tag);
    }

    /// Adds relationship into DB. Inherently trusts user user to not duplicate stuff.
    pub fn relationship_add(&self, file: usize, tag: usize) {
        match self._cache {
            CacheType::Bare => {
                self.relationship_add_sql(&file, &tag);
            }
            _ => {
                let existcheck = self._inmemdb.read().relationship_get(&file, &tag);
                if !existcheck {
                    // println!("relationship a ");
                    self.relationship_add_sql(&file, &tag);
                }
                if !existcheck {
                    // println!("relationship b ");
                    self.relationship_add_db(file, tag);
                }
            }
        }
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

    ///
    /// Adds a job into a DB.
    /// Going to make a better job adding function
    ///
    pub fn jobs_add(
        &self,

        id: Option<usize>,
        time: usize,
        reptime: usize,
        priority: usize,
        cachetime: Option<usize>,
        cachechecktype: sharedtypes::JobCacheType,
        site: String,
        param: Vec<ScraperParam>,
        system_data: BTreeMap<String, String>,
        user_data: BTreeMap<String, String>,
        jobsmanager: sharedtypes::DbJobsManager,
    ) -> usize {
        let id = match id {
            None => self.jobs_get_max(),
            Some(id) => id,
        };

        for each in self.jobs_get_all() {
            if time == each.1.time
                && reptime == each.1.reptime
                && site == each.1.site
                && param == each.1.param
            {
                return id;
            }
        }

        let jobs_obj: sharedtypes::DbJobsObj = sharedtypes::DbJobsObj {
            id: Some(id),
            time,
            reptime,
            priority,
            cachetime,
            cachechecktype,
            site: site.clone(),
            param: param.clone(),
            jobmanager: jobsmanager.clone(),
            isrunning: false,
            system_data: system_data.clone(),
            user_data: user_data.clone(),
        };
        self.jobs_update_db(jobs_obj.clone());

        // let jobsmanager = sharedtypes::DbJobsManager { jobtype: *dbjobtype, recreation:
        // None, additionaldata: None, };
        id
    }

    pub fn jobs_add_new(&self, dbjobsobj: sharedtypes::DbJobsObj) -> usize {
        let mut dbjobsobj = dbjobsobj.clone();
        let id = match dbjobsobj.id {
            None => self.jobs_get_max(),
            Some(id) => id,
        };

        for each in self.jobs_get_all() {
            if dbjobsobj.time == each.1.time
                && dbjobsobj.reptime == each.1.reptime
                && dbjobsobj.site == each.1.site
                && dbjobsobj.param == each.1.param
            {
                dbg!("RET");
                return id;
            }
        }

        dbjobsobj.id = Some(id);

        self.jobs_update_db(dbjobsobj);

        id
    }

    /// Wrapper for inmemdb insert.
    pub(super) fn setting_add_db(
        &self,
        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        self._inmemdb.write().settings_add(name, pretty, num, param);
    }

    /// Adds a setting to the Settings Table. name: str   , Setting name pretty: str ,
    /// Fancy Flavor text optional num: u64    , unsigned u64 largest int is
    /// 18446744073709551615 smallest is 0 param: str  , Parameter to allow (value)
    pub fn setting_add(
        &self,

        name: String,
        pretty: Option<String>,
        num: Option<usize>,
        param: Option<String>,
    ) {
        self.setting_add_sql(name.to_string(), &pretty, num, &param);

        // Adds setting into memdbb
        self.setting_add_db(name, pretty, num, param);
    }

    /// Returns db location as String refernce.
    pub fn get_db_loc(&self) -> Option<String> {
        self._dbpath.clone()
    }

    /// Parses data from search query to return an id
    fn search_for_namespace(&self, search: &sharedtypes::DbSearchObject) -> Option<usize> {
        match &search.namespace {
            None => search.namespace_id,
            Some(id_string) => self.namespace_get(id_string),
        }
    }

    /*/// Raw Call to database. Try to only use internally to this file only. Doesn't
    /// support params nativly. Will not write changes to DB. Have to call write().
    /// Panics to help issues.
    fn execute(&self, inp: String) -> usize {
        let tn = self.conn.lock();
        let out = tn.execute(&inp, params![]);
        match out {
            Err(_out) => {
                println!("SQLITE STRING:: {}", inp);
                println!("BAD CALL {}", _out);
                error!("BAD CALL {}", _out);
                panic!("BAD CALL {}", _out);
            }
            Ok(_out) => _out,
        }
    }*/

    /*/// Deletes an item from jobs table. critera is the searchterm and collumn is the
    /// collumn to target. Doesn't remove from inmemory database
    pub fn del_from_jobs_table(&self, job: &sharedtypes::JobScraper) {
        self.del_from_jobs_table_sql(&job.site, &job.param);
    }*/

    /// Removes a job from the database by id. Removes from both memdb and sql.
    pub fn del_from_jobs_byid(&self, id: Option<&usize>) {
        if let Some(id) = id {
            self.del_from_jobs_inmemdb(id);
            self.del_from_jobs_table_sql_better(id);
        }
    }

    /// Removes job from inmemdb Removes by id
    fn del_from_jobs_inmemdb(&self, id: &usize) {
        self._inmemdb.write().jobref_remove(id)
    }

    ///
    /// Migrates a tag to a new tag from an old tag
    ///
    pub fn migrate_relationship_tag(&self, old_tag_id: &usize, new_tag_id: &usize) {
        match self._cache {
            CacheType::Bare => {
                self.migrate_relationship_tag_sql(old_tag_id, new_tag_id);
            }
            _ => {
                let fileids = self.relationship_get_fileid(old_tag_id);
                for file_id in fileids {
                    self.relationship_add(file_id, *new_tag_id);
                    self.relationship_remove(&file_id, old_tag_id);
                }
            }
        }
        self.parents_migration(old_tag_id, new_tag_id);
    }

    ///
    /// Migrates one tag per file to a new tagid
    ///
    pub fn migrate_relationship_file_tag(
        &self,
        file_id: &usize,
        old_tag_id: &usize,
        new_tag_id: &usize,
    ) {
        match self._cache {
            CacheType::Bare => {
                self.migrate_relationship_file_tag_sql(file_id, old_tag_id, new_tag_id);
            }
            _ => {
                self.relationship_add(*file_id, *new_tag_id);
                self.relationship_remove(file_id, old_tag_id);
            }
        }
        self.parents_migration(old_tag_id, new_tag_id);
    }

    ///
    /// Migrates a tag from one ID to another
    /// NOTE Make this an exclusive transaction otherwise we could drop data
    ///
    pub fn migrate_tag(&self, old_tag_id: &usize, new_tag_id: &usize, tn: &mut Transaction) {
        if !matches!(self._cache, CacheType::InMemdb) {
            tn.execute(
                "UPDATE Tags SET id = ? WHERE id = ?",
                params![new_tag_id, old_tag_id],
            )
            .unwrap();
            return;
        }
        let tag = match self.tag_id_get(old_tag_id) {
            Some(out) => out,
            None => {
                return;
            }
        };

        if self.tag_id_get(new_tag_id).is_none() {
            panic!(
                "Old tagid {},new tagid {} new tagid already exists cannot delete",
                old_tag_id, new_tag_id
            );
            return;
        } else {
            self.tag_add(&tag.name.clone(), tag.namespace, Some(*new_tag_id));
        }

        logging::log(format!("Moving tagid: {} to {}", old_tag_id, new_tag_id));
        self.parents_migration(old_tag_id, new_tag_id);
        self.migrate_relationship_tag(old_tag_id, new_tag_id);
        self.tag_remove(old_tag_id);
    }

    /// Removes tag & relationship from db.
    pub fn delete_tag_relationship(&self, tag_id: &usize) {
        let relationships = self.relationship_get_fileid(tag_id);

        // Gets list of fileids from internal db.

        for _fileids in relationships.iter() {
            logging::log(format!(
                "Found {} relationships's effected for tagid: {}.",
                relationships.len(),
                tag_id
            ));

            // let mut sql = String::new();
            for file_id in relationships.clone() {
                logging::log(format!(
                    "Removing file: {} tagid: {} from db.",
                    file_id, tag_id
                ));

                logging::log("Removing relationship sql".to_string());

                self.relationship_remove(&file_id, tag_id);
            }

            // self.tn.lock().execute_batch(&sql).unwrap();
            logging::log("Relationship Loop".to_string());
        }
    }

    ///
    /// Migrates a parent from a old tagid to a new id
    ///
    fn parents_migration(&self, old_tag_id: &usize, new_tag_id: &usize) {
        // Removes parent by ID and readds it with the new id
        for parent in self.parents_tagid_remove(old_tag_id) {
            logging::log(format!(
                "T Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                parent.tag_id,
                parent.relate_tag_id,
                parent.limit_to,
                new_tag_id,
                parent.relate_tag_id,
                parent.limit_to
            ));
            let par = sharedtypes::DbParentsObj {
                tag_id: *new_tag_id,
                relate_tag_id: parent.relate_tag_id,
                limit_to: parent.limit_to,
            };
            self.parents_add(par);
        }

        // Removes parent by ID and readds it with the new id
        for parent in self.parents_reltagid_remove(old_tag_id) {
            logging::log(format!(
                "R Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                parent.tag_id,
                parent.relate_tag_id,
                parent.limit_to,
                parent.tag_id,
                new_tag_id,
                parent.limit_to
            ));
            let par = sharedtypes::DbParentsObj {
                tag_id: parent.tag_id,
                relate_tag_id: *new_tag_id,
                limit_to: parent.limit_to,
            };
            self.parents_add(par);
        }
        // Kinda hacky but nothing bad will happen if we have nothing in the limit
        // slot
        for parent in self.parents_limitto_remove(Some(*old_tag_id)) {
            logging::log(format!(
                "L Deleting parent: {} {} {:?}  replacing with {} {} {:?}",
                parent.tag_id,
                parent.relate_tag_id,
                parent.limit_to,
                parent.tag_id,
                parent.relate_tag_id,
                new_tag_id
            ));
            let par = sharedtypes::DbParentsObj {
                tag_id: parent.tag_id,
                relate_tag_id: parent.relate_tag_id,
                limit_to: Some(*new_tag_id),
            };
            self.parents_add(par);
        }
    }

    ///
    /// Removes a relationship based on fileid and tagid
    ///
    pub fn relationship_remove(&self, file_id: &usize, tag_id: &usize) {
        self._inmemdb.write().relationship_remove(file_id, tag_id);
        self.delete_relationship_sql(file_id, tag_id);
    }

    /// Removes tag from inmemdb and sql database.
    pub fn tag_remove(&self, id: &usize) {
        for fid in self.relationship_get_fileid(id).iter() {
            self.relationship_remove(fid, id);
        }

        self._inmemdb.write().tag_remove(id);
        self.delete_tag_sql(id);
        let rel = &self._inmemdb.write().parents_remove(id);
        for each in rel {
            //println!("Removing Parent: {} {}", each.0, each.1);
            self.delete_parent_sql(&each.0, &each.1);
        }
    }

    ///
    /// Removes parent from db
    ///
    pub fn parents_tagid_remove(&self, tag_id: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        match self._cache {
            CacheType::Bare => {
                let out = self.parents_tagid_tag_get(tag_id);

                self.parents_delete_tag_id_sql(tag_id);
                out
            }
            _ => {
                self.parents_delete_tag_id_sql(tag_id);
                self._inmemdb.write().parents_tagid_remove(tag_id)
            }
        }
    }

    ///
    /// Wrapper for inmemdb
    ///
    pub fn parents_reltagid_remove(&self, reltag: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        match self._cache {
            CacheType::Bare => {
                let out = self.parents_relate_tag_get(reltag);

                self.parents_delete_relate_tag_id_sql(reltag);
                out
            }
            _ => {
                self.parents_delete_relate_tag_id_sql(reltag);
                self._inmemdb.write().parents_reltagid_remove(reltag)
            }
        }
    }

    pub fn parents_limitto_remove(
        &self,

        limit_to: Option<usize>,
    ) -> HashSet<sharedtypes::DbParentsObj> {
        match self._cache {
            CacheType::Bare => {
                if let Some(limit_to) = limit_to {
                    let temp = self.parents_limitto_tag_get(&limit_to);

                    self.parents_delete_limit_to_sql(&limit_to);
                    temp
                } else {
                    HashSet::new()
                }
            }
            _ => {
                if let Some(limit_to) = limit_to {
                    self.parents_delete_limit_to_sql(&limit_to);
                    self._inmemdb.write().parents_limitto_remove(&limit_to)
                } else {
                    HashSet::new()
                }
            }
        }
    }

    /// Removes a parent selectivly
    pub fn parents_selective_remove(&self, parentobj: &sharedtypes::DbParentsObj) {
        todo!("Need to fix this for sqlite");
        self._inmemdb.write().parents_selective_remove(parentobj);
    }

    /// Deletes namespace by id Removes tags & relationships assocated.
    pub fn namespace_delete_id(&self, id: &usize) {
        logging::info_log(format!("Starting deletion work on namespace id: {}", id));

        // self.vacuum(); self.tn.lock().execute("create index ffid on
        // Relationship(fileid);", []);
        if self.namespace_get_string(id).is_none() {
            logging::info_log("Stopping because I cannot get ns string.".to_string());
            return;
        }
        let tagids = self.namespace_get_tagids(id);
        for each in tagids.clone().iter() {
            self.tag_remove(each);

            // tag_sql += &format!("DELETE FROM Tags WHERE id = {}; ", each);
            self.delete_tag_relationship(each);
        }

        // elf.tn.lock().execute_batch(&tag_sql).unwrap();
        self._inmemdb.write().namespace_delete(id);
        self.delete_namespace_sql(id);
    }

    ///
    /// Condenses and corrects file locations
    ///
    pub fn condense_file_locations(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Files);

        let mut file_id_list: Vec<usize> = self.file_get_list_id().into_iter().collect();

        file_id_list.par_sort_unstable();

        for (ref cnt, key) in file_id_list.iter().enumerate() {
            if cnt != key {
                dbg!("mismatch", &key, &cnt);
            }
        }
    }

    fn get_starting_tag_id(&self) -> usize {
        let mut out = None;

        for id in 0..self.tags_max_id() {
            if self.tag_id_get(&id).is_some() {
                out = Some(id);
                break;
            }
        }

        if out.is_none() {
            out = Some(0);
        }

        out.unwrap()
    }

    /// Removes all empty tags in the db
    fn clear_empty_tags(&self) {
        logging::info_log("Starting to clear any unlinked tags from the db");
        let empty_tags = self.get_empty_tagids();
        logging::info_log(format!("Found {} empty tags will clear.", empty_tags.len()));
        for tag_id in empty_tags.iter() {
            self.tag_remove(tag_id);
        }
    }

    ///
    /// Condenses tag ids into a solid column
    /// NOTE Make this an exclusive transaction otherwise we could drop data
    ///
    pub fn condense_tags(&self, tn: &mut Transaction) {
        self.load_table(&sharedtypes::LoadDBTable::Tags);

        // Stopping automagically updating the db

        let tag_max = self.tags_max_id();

        let mut flag = false;

        logging::info_log("Starting preliminary tags scanning".to_string());
        for id in self.get_starting_tag_id()..tag_max - 1 {
            if self.tag_id_get(&id).is_none() {
                logging::info_log(format!(
                    "Disjointed tags detected. initting fixing badid: {}",
                    &id
                ));
                flag = true;
                break;
            }
        }

        if flag {
            logging::info_log("Started loading files, relationship and parents table".to_string());
            self.load_table(&sharedtypes::LoadDBTable::Files);
            self.load_table(&sharedtypes::LoadDBTable::Relationship);
            self.load_table(&sharedtypes::LoadDBTable::Parents);
            flag = false;
        }

        let mut cnt: usize = self.get_starting_tag_id();
        let mut last_highest: usize = cnt;
        let mut eta = Eta::new(tag_max - self.get_starting_tag_id(), TimeAcc::MILLI);
        let count_percent = tag_max.div_ceil(100 / 2);
        for id in self.get_starting_tag_id()..tag_max + 1 {
            let tagnns = self.tag_id_get(&id);
            match tagnns {
                None => {
                    flag = true;
                }
                Some(tag) => {
                    if flag {
                        logging::log(format!("Migrating tag id {id} to {cnt}"));
                        self.migrate_tag(&id, &cnt, tn);
                        last_highest = cnt;
                    }
                    cnt += 1;
                }
            }
            eta.step();
            if (cnt % count_percent) == 0 || cnt == 5 || cnt == tag_max {
                logging::info_log(format!("{}", &eta));
            }
        }

        // Updating internal caches highest tag value
        match self._cache {
            CacheType::Bare => {}
            _ => {
                self._inmemdb.write().tag_max_set(last_highest + 1);
            }
        }
    }

    pub fn condense_namespace(&self) {
        self.load_table(&sharedtypes::LoadDBTable::Namespace);
        self.load_table(&sharedtypes::LoadDBTable::Tags);

        //self.namespace_delete_id(&2);

        for namespace_id in self.namespace_keys() {
            dbg!(&namespace_id);
            // if self.namespace_get_tagids(&namespace_id).is_empty() {
            //     dbg!(&namespace_id);
            // }
        }
    }

    /// Condesnes relationships between tags & files. Changes tag id's removes spaces
    /// inbetween tag id's and their relationships.
    /// NOTE Make this an exclusive transaction otherwise we could drop data
    pub fn condense_db_all(&self) {
        self.clear_empty_tags();
        let mut write_conn = self.write_conn.lock();
        let mut tn = write_conn.transaction().unwrap();
        //self.condense_namespace();
        self.condense_tags(&mut tn);

        tn.commit().unwrap();
        //self.condense_file_locations();
        //self.vacuum();
    }

    /*/// Recreates the db with only one ns in it.
    pub fn drop_recreate_ns(&self, id: &usize) {
        // self.load_table(&sharedtypes::LoadDBTable::Relationship);
        // self.load_table(&sharedtypes::LoadDBTable::Parents);
        // self.load_table(&sharedtypes::LoadDBTable::Tags);
        self.db_drop_table(&"Relationship".to_string());
        self.db_drop_table(&"Tags".to_string());
        self.db_drop_table(&"Parents".to_string());

        // Recreates tables with new defaults
        self.first_db();
        self.transaction_flush();

        // let tag_max = self._inmemdb.tags_max_return().clone();
        self._inmemdb.tags_max_reset();
        let tida = self.namespace_get_tagids(id);
        for (cnt, tid) in tida.iter().enumerate() {
            let file_listop = self._inmemdb.relationship_get_fileid(tid).clone();
            let tag = self._inmemdb.tags_get_data(tid).unwrap().clone();
            self.tag_add_sql(&cnt, &tag.name, &tag.namespace);
            for file in file_listop {
                self.relationship_add_sql(file, cnt);
            }
        }
        self._inmemdb.tags_clear();
        self._inmemdb.parents_clear();
        self._inmemdb.relationships_clear();
        self.vacuum();
        self.transaction_flush();

        // self.condese_relationships_tags();
        self.transaction_flush();
    }*/

    fn print_all_tables(&self) -> Result<()> {
        use rusqlite::Row;
        let conn = self.write_conn.lock();
        // Step 1: Get all table names
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';",
        )?;

        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        for table in table_names {
            println!("Table: {}", table);
            println!("Columns and constraints:");

            // Step 2: Get columns and constraints
            let mut col_stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
            let columns = col_stmt.query_map([], |row: &Row| {
                let cid: i32 = row.get(0)?;
                let name: String = row.get(1)?;
                let dtype: String = row.get(2)?;
                let notnull: i32 = row.get(3)?; // 1 if NOT NULL
                let dflt_value: Option<String> = row.get(4)?;
                let pk: i32 = row.get(5)?; // 1 if part of PK
                Ok((name, dtype, notnull != 0, dflt_value, pk != 0))
            })?;

            for col in columns {
                let (name, dtype, notnull, dflt, pk) = col?;
                println!(
                    "- {} {}{}{}",
                    name,
                    dtype,
                    if notnull { " NOT NULL" } else { "" },
                    if pk { " PRIMARY KEY" } else { "" }
                );
                if let Some(d) = dflt {
                    println!("  Default: {}", d);
                }
            }

            // Step 3: Get unique indexes
            let mut idx_stmt = conn.prepare(&format!("PRAGMA index_list({})", table))?;

            let indexes: Vec<(String, bool)> = idx_stmt
                .query_map([], |row: &Row| {
                    let name: String = row.get(1)?; // index name
                    let unique: i32 = row.get(2)?; // 1 if UNIQUE
                    Ok((name, unique != 0))
                })?
                .collect::<Result<_, _>>()?; // collect into Vec, propagating any errors

            for (name, unique) in indexes {
                if unique {
                    let mut info_stmt = conn.prepare(&format!("PRAGMA index_info({})", name))?;
                    let cols: Vec<String> = info_stmt
                        .query_map([], |row: &Row| row.get(2))?
                        .collect::<Result<_, _>>()?;
                    println!("- UNIQUE index on columns: {:?}", cols);
                }
            }

            // Step 4: Print table data
            let mut data_stmt = conn.prepare(&format!("SELECT * FROM {}", table))?;
            let column_names: Vec<String> = data_stmt
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            println!("Data columns: {:?}", column_names);

            let rows = data_stmt.query_map([], |row: &Row| {
                let mut values = Vec::new();
                for i in 0..column_names.len() {
                    let val: Result<String, _> = row.get(i);
                    values.push(val.unwrap_or_else(|_| "NULL".to_string()));
                }
                Ok(values)
            })?;

            for row in rows {
                println!("{:?}", row?);
            }
            println!("-----------------------------------");
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test_database {
    use super::*;
    use crate::{VERS, client::relationship_get_tagid};

    pub fn setup_default_db() -> Vec<Main> {
        let mut out = Vec::new();
        for (cnt, cachetype) in [CacheType::Bare].iter().enumerate() {
            let mut db = Main::new(Some(format!("test{}.db", cnt)), VERS);
            db._cache = cachetype.clone();

            db.tag_add(&"test".to_string(), 1, None);
            db.tag_add(&"test1".to_string(), 1, None);
            db.tag_add(&"test2".to_string(), 1, None);
            db.transaction_flush();

            out.push(db);
        }
        out
    }

    #[test]
    fn db_relationship() {
        for mut main in setup_default_db() {
            main.relationship_add(0, 0);
            dbg!(&main._cache, main.relationship_get_fileid(&0));
            assert_eq!(main.relationship_get_fileid(&0).len(), 1);
            assert_eq!(main.relationship_get_tagid(&0).len(), 1);
            let mut test_hashset: HashSet<usize> = HashSet::new();
            test_hashset.insert(0);
            assert_eq!(main.relationship_get_fileid(&0), test_hashset);
            assert_eq!(main.relationship_get_tagid(&0), test_hashset);
        }
    }

    #[test]
    fn db_parents_tagid_remove() {
        for mut main in setup_default_db() {
            main.parents_tagid_remove(&1);
            assert_eq!(main.parents_rel_get(&1), HashSet::new());
            assert_eq!(main.parents_tag_get(&1), HashSet::new());
        }
    }

    #[test]
    fn db_test_load_tags() {
        for mut main in setup_default_db() {
            dbg!(&main._cache);
            dbg!(main.tags_max_id());
            dbg!(main.tag_id_get(&0));
            dbg!(main.tag_id_get(&1));
            dbg!(main.tag_id_get(&2));
            dbg!(main.tag_id_get(&3));
            dbg!(main.tag_id_get(&4));
            assert_eq!(main.tags_max_id(), 4);
            let id = main.tag_add(&"test3".to_string(), 3, None);
            assert_eq!(id, 4);

            assert!(main.tag_id_get(&2).is_some());
        }
    }
    /* #[test]
    fn condense_tags_test() {
        let test_id = 10;
        for mut main in setup_default_db() {
            let max_tag = main.tags_max_id();
            let id = main.tag_add(&"test3".to_string(), 3,  Some(test_id));
            let parents_id = main.parents_add(sharedtypes::DbParentsObj {
                tag_id: id,
                relate_tag_id: 3,
                limit_to: None,
            });
            for i in 0..test_id + 1 {
                dbg!(&i, main.tag_id_get(&i));
            }

            main.parents_add(sharedtypes::DbParentsObj {
                tag_id: id,
                relate_tag_id: 1,
                limit_to: None,
            });
            main.parents_add(sharedtypes::DbParentsObj {
                tag_id: 2,
                relate_tag_id: id,
                limit_to: None,
            });
            main.parents_add(sharedtypes::DbParentsObj {
                tag_id: 3,
                relate_tag_id: 1,
                limit_to: Some(id),
            });
            main.transaction_flush();
            dbg!(&main._cache, &max_tag, &id);
            main.condense_tags();

            assert!(
                main.parents_get(&sharedtypes::DbParentsObj {
                    tag_id: 3,
                    relate_tag_id: 1,
                    limit_to: Some(max_tag),
                })
                .is_some(),
            );
            assert!(
                main.parents_get(&sharedtypes::DbParentsObj {
                    tag_id: 2,
                    relate_tag_id: max_tag,
                    limit_to: None,
                })
                .is_some(),
            );
            assert!(
                main.parents_get(&sharedtypes::DbParentsObj {
                    tag_id: max_tag,
                    relate_tag_id: 1,
                    limit_to: None,
                })
                .is_some(),
            );

            // Check to see if the parents actually migrated.
            if let Some(parentid) = main.parents_get(&sharedtypes::DbParentsObj {
                tag_id: main.tags_max_id(),
                relate_tag_id: 3,
                limit_to: None,
            }) {
                assert_eq!(parentid, parents_id);
            }

            assert_eq!(main.tags_max_id(), max_tag + 1);
            assert!(main.tag_id_get(&id).is_none());

            for i in 0..max_tag + 1 {
                dbg!(&i, main.tag_id_get(&i));
            }

            assert!(main.tag_id_get(&(max_tag)).is_some());

            if let Some(tag) = main.tag_id_get(&(max_tag)) {
                assert_eq!(tag.name, "test3");
            }
        }
    }*/
    #[test]
    fn db_namespace() {
        for mut main in setup_default_db() {
            dbg!(&main._cache);
            let testid = main.namespace_add(&"test".to_string(), &Some("woohoo".into()));
            let descid = main.namespace_add(&"desc".to_string(), &None);

            dbg!(testid, descid);
            assert!(main.namespace_get(&"test".into()).is_some());
            assert!(main.namespace_get(&"desc".into()).is_some());
        }
    }

    ///
    /// This test does not deduplicate data. We rely on the jobs obj to deduplicate
    ///
    #[test]
    fn db_jobs_check_id() {
        for mut main in setup_default_db() {
            dbg!(&main._cache);
            dbg!(main.jobs_get_max());
            let current_job_max = main.jobs_get_max();
            main.jobs_add(
                None,
                0,
                0,
                sharedtypes::DEFAULT_PRIORITY,
                sharedtypes::DEFAULT_CACHETIME,
                sharedtypes::DEFAULT_CACHECHECK,
                "".to_string(),
                vec![],
                BTreeMap::new(),
                BTreeMap::new(),
                sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::NoScrape,
                    recreation: None,
                },
            );
            dbg!(main.jobs_get_max());
            dbg!(main.jobs_get(&0));
            assert_eq!(main.jobs_get_max(), current_job_max + 1);
            main.jobs_add(
                None,
                0,
                0,
                sharedtypes::DEFAULT_PRIORITY,
                sharedtypes::DEFAULT_CACHETIME,
                sharedtypes::DEFAULT_CACHECHECK,
                "yeet".to_string(),
                vec![],
                BTreeMap::new(),
                BTreeMap::new(),
                sharedtypes::DbJobsManager {
                    jobtype: sharedtypes::DbJobType::NoScrape,
                    recreation: None,
                },
            );
            dbg!(main.jobs_get_max());
            dbg!(main.jobs_get_all());
            assert_eq!(main.jobs_get_max(), current_job_max + 2);

            // Checks if all jobs exist at current time
            for i in current_job_max..main.jobs_get_max() {
                assert!(main.jobs_get(&i).is_some());
            }
        }
    }

    #[test]
    fn db_dead_jobs() {
        let mut mains = setup_default_db();
        mains.push(Main::new(Some("test1.db".to_string()), VERS));

        for mut main in mains {
            main.add_dead_url(&"test".to_string());

            assert!(main.check_dead_url(&"test".to_string()));
            assert!(!main.check_dead_url(&"Null".to_string()));
        }
    }

    #[test]
    fn file_add_test() {
        let mut mains = setup_default_db();

        for mut main in mains {
            dbg!(&main._cache);
            let id = main.file_add_db(sharedtypes::DbFileStorage::NoIdExist(
                sharedtypes::DbFileObjNoId {
                    hash: "yeet".to_string(),
                    ext_id: 1,
                    storage_id: 2,
                },
            ));
            let id1 = main.file_add_db(sharedtypes::DbFileStorage::NoIdExist(
                sharedtypes::DbFileObjNoId {
                    hash: "yeet".to_string(),
                    ext_id: 1,
                    storage_id: 2,
                },
            ));

            assert_eq!(id, id1);
        }
    }
}
