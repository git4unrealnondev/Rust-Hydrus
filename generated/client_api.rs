use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use crate::sharedtypes;
#[derive(Debug)]
pub struct RustHydrusApiClient {
    pub base_url: String,
}
#[allow(dead_code)]
impl RustHydrusApiClient {
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        let base_url_str = base_url.into();
        let base_url_temp = if !base_url_str.starts_with("http") {
            format!("http://{}", base_url_str)
        } else {
            base_url_str
        };
        RustHydrusApiClient {
            base_url: base_url_temp,
        }
    }
    /// Gets a scraper folder. If it doesn't exist then please create it in db
    pub fn loaded_scraper_folder(&self) -> Result<PathBuf, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "loaded_scraper_folder");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<PathBuf>()?;
        Ok(res)
    }
    /// Gets a plugin folder. If it doesn't exist then please create it in db
    pub fn loaded_plugin_folder(&self) -> Result<PathBuf, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "loaded_plugin_folder");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<PathBuf>()?;
        Ok(res)
    }
    /// Deletes a namespace by id
    pub fn delete_namespace_id(&self, nsid: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "delete_namespace_id");
        let res = ureq::post(url)
            .send_json(&(nsid))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn check_default_source_urls(
        &self,
        action: &sharedtypes::CheckSourceUrlsEnum,
    ) -> Result<(), ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "check_default_source_urls"
        );
        let res = ureq::post(url)
            .send_json(&(action))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Checks relationships with table for any dead tagids
    pub fn check_relationship_tag_relations(&self) -> Result<(), ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "check_relationship_tag_relations"
        );
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Removes a job from the database by id. Removes from both memdb and sql.
    pub fn del_from_jobs_byid(&self, id: Option<u64>) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "del_from_jobs_byid");
        let res = ureq::post(url)
            .send_json(&(id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn file_add(
        &self,
        file: sharedtypes::DbFileStorage,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_add");
        let res = ureq::post(url)
            .send_json(&(file))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    ///
    pub fn storage_put(&self, location: &String) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "storage_put");
        let res = ureq::post(url)
            .send_json(&(location))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    /// Adds tags to fileid  commits to db
    pub fn add_tags_to_fileid(
        &self,
        file_id: Option<u64>,
        tag_actions: &Vec<sharedtypes::FileTagAction>,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "add_tags_to_fileid");
        let res = ureq::post(url)
            .send_json(&(file_id, tag_actions))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn delete_tag(&self, tag: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "delete_tag");
        let res = ureq::post(url)
            .send_json(&(tag))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn parents_tagid_remove(
        &self,
        tagid: &u64,
    ) -> Result<HashSet<sharedtypes::DbParentsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_tagid_remove");
        let res = ureq::post(url)
            .send_json(&(tagid))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<sharedtypes::DbParentsObj>>()?;
        Ok(res)
    }
    /// Adds relationship into db
    pub fn add_relationship(&self, file: &u64, tag: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "add_relationship");
        let res = ureq::post(url)
            .send_json(&(file, tag))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn delete_relationship(&self, file: &u64, tag: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "delete_relationship");
        let res = ureq::post(url)
            .send_json(&(file, tag))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Checks if a relationship exists in the db
    pub fn check_relationship_exists(
        &self,
        file_id: &u64,
        tag_id: &u64,
    ) -> Result<bool, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "check_relationship_exists"
        );
        let res = ureq::post(url)
            .send_json(&(file_id, tag_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<bool>()?;
        Ok(res)
    }
    /// Adds the tag to the db. commits on finish
    pub fn tag_add_tagobject(
        &self,
        tag: &sharedtypes::TagObject,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_add_tagobject");
        let res = ureq::post(url)
            .send_json(&(tag))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// Adds multiple tags to the db. commits on finish
    pub fn tag_add_tagobject_multiple(
        &self,
        tag_list: &HashSet<sharedtypes::TagObject>,
    ) -> Result<(), ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "tag_add_tagobject_multiple"
        );
        let res = ureq::post(url)
            .send_json(&(tag_list))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// condesnes everything in db
    pub fn condense_db_all(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "condense_db_all");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Sets a relationship between a fileid old and new tagid
    pub fn condense_tags(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "condense_tags");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Sets a relationship between a fileid old and new tagid
    pub fn migrate_tag(
        &self,
        old_tag_id: &u64,
        new_tag_id: &u64,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "migrate_tag");
        let res = ureq::post(url)
            .send_json(&(old_tag_id, new_tag_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Sets a relationship between a fileid old and new tagid
    pub fn migrate_relationship_file_tag(
        &self,
        file_id: &u64,
        old_tag_id: &u64,
        new_tag_id: &u64,
    ) -> Result<(), ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "migrate_relationship_file_tag"
        );
        let res = ureq::post(url)
            .send_json(&(file_id, old_tag_id, new_tag_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Updates the database for inmemdb and sql
    pub fn jobs_update_db(
        &self,
        jobs_obj: sharedtypes::DbJobsObj,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_update_db");
        let res = ureq::post(url)
            .send_json(&(jobs_obj))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Removes a parent selectivly
    pub fn parents_selective_remove(
        &self,
        parentobj: &sharedtypes::DbParentsObj,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_selective_remove");
        let res = ureq::post(url)
            .send_json(&(parentobj))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Adds a parent into the db
    pub fn parents_add(
        &self,
        par: sharedtypes::DbParentsObj,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_add");
        let res = ureq::post(url)
            .send_json(&(par))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    /// Adds tag into db
    pub fn tag_add(
        &self,
        tags: &String,
        namespace: u64,
        id: Option<u64>,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_add");
        let res = ureq::post(url)
            .send_json(&(tags, namespace, id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    /// Checks if table is loaded in mem and if not then loads it.
    pub fn load_table(
        &self,
        table: &sharedtypes::LoadDBTable,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "load_table");
        let res = ureq::post(url)
            .send_json(&(table))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn setting_add(
        &self,
        name: String,
        pretty: Option<String>,
        num: Option<u64>,
        param: Option<String>,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "setting_add");
        let res = ureq::post(url)
            .send_json(&(name, pretty, num, param))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Adds a dead url into the db
    pub fn add_dead_url(&self, url_string: &String) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "add_dead_url");
        let res = ureq::post(url)
            .send_json(&(url_string))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /** Searches the database using FTS5 allows getting a list of tags and their count based on a

 search string and a limit of tagids to get*/
    pub fn search_tags(
        &self,
        search_string: &String,
        limit_to: &u64,
        fts_or_count: sharedtypes::TagPartialSearchType,
    ) -> Result<Vec<(sharedtypes::Tag, u64, u64)>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "search_tags");
        let res = ureq::post(url)
            .send_json(&(search_string, limit_to, fts_or_count))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Vec<(sharedtypes::Tag, u64, u64)>>()?;
        Ok(res)
    }
    /** Searches the database using FTS5 allows getting a list of tagids and their count based on a

 search string and a limit of tagids to get*/
    pub fn search_tags_ids(
        &self,
        search_string: &String,
        limit_to: &u64,
        fts_or_count: sharedtypes::TagPartialSearchType,
    ) -> Result<Vec<(u64, u64)>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "search_tags_ids");
        let res = ureq::post(url)
            .send_json(&(search_string, limit_to, fts_or_count))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Vec<(u64, u64)>>()?;
        Ok(res)
    }
    /// A test function to return 1
    pub fn test(&self) -> Result<u32, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "test");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u32>()?;
        Ok(res)
    }
    /// Returns the db version number
    pub fn db_vers_get(&self) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "db_vers_get");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    /// Returns a list of loaded tag ids
    pub fn tags_get_list_id(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tags_get_list_id");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(
        &self,
        tag: &u64,
    ) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "relationship_get_fileid");
        let res = ureq::post(url)
            .send_json(&(tag))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// Gets one fileid from one tagid
    pub fn relationship_get_one_fileid(
        &self,
        tag: &u64,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "relationship_get_one_fileid"
        );
        let res = ureq::post(url)
            .send_json(&(tag))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(
        &self,
        file_id: &u64,
    ) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "relationship_get_tagid");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    ///
    pub fn settings_get_name(
        &self,
        name: &String,
    ) -> Result<Option<sharedtypes::DbSettingObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "settings_get_name");
        let res = ureq::post(url)
            .send_json(&(name))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<sharedtypes::DbSettingObj>>()?;
        Ok(res)
    }
    /// Correct any weird paths existing inside of the db.
    pub fn check_db_paths(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "check_db_paths");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Backs up the DB file.
    pub fn backup_db(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "backup_db");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /** Returns a files bytes if the file exists. Note if called from intcom then this

 locks the DB while getting the file. One workaround it to use get_file and read

 bytes in manually in seperate thread. that way minimal locking happens.*/
    pub fn get_file_bytes(&self, file_id: &u64) -> Result<Option<Vec<u8>>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "get_file_bytes");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<Vec<u8>>>()?;
        Ok(res)
    }
    /// Gets the location of a file in the file system
    pub fn get_file(&self, file_id: &u64) -> Result<Option<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "get_file");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<String>>()?;
        Ok(res)
    }
    ///Checks if a url is dead
    pub fn check_dead_url(&self, url_to_check: &String) -> Result<bool, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "check_dead_url");
        let res = ureq::post(url)
            .send_json(&(url_to_check))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<bool>()?;
        Ok(res)
    }
    /// Gets all running jobs in the db
    pub fn jobs_get_isrunning(
        &self,
    ) -> Result<HashSet<sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get_isrunning");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<sharedtypes::DbJobsObj>>()?;
        Ok(res)
    }
    /// Returns all locations currently inside of the db.
    pub fn storage_get_all(&self) -> Result<Vec<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "storage_get_all");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Vec<String>>()?;
        Ok(res)
    }
    /** Handles the searching of the DB dynamically. Returns the file id's associated

 with the search.

 Returns file IDs matching the search.

 Supports AND, OR, NOT operations.*/
    pub fn search_db_files(
        &self,
        search: sharedtypes::SearchObj,
        limit: Option<u64>,
    ) -> Result<Option<Vec<u64>>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "search_db_files");
        let res = ureq::post(url)
            .send_json(&(search, limit))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<Vec<u64>>>()?;
        Ok(res)
    }
    /// Gets all jobs loaded in the db
    pub fn jobs_get_all(
        &self,
    ) -> Result<HashMap<u64, sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get_all");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashMap<u64, sharedtypes::DbJobsObj>>()?;
        Ok(res)
    }
    /// Pull job by id TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    pub fn jobs_get(
        &self,
        id: &u64,
    ) -> Result<Option<sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get");
        let res = ureq::post(url)
            .send_json(&(id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<sharedtypes::DbJobsObj>>()?;
        Ok(res)
    }
    /// Gets a tag by id
    pub fn tag_id_get(
        &self,
        uid: &u64,
    ) -> Result<Option<sharedtypes::DbTagNNS>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_id_get");
        let res = ureq::post(url)
            .send_json(&(uid))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<sharedtypes::DbTagNNS>>()?;
        Ok(res)
    }
    /// Vacuums database. cleans everything.
    pub fn vacuum(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "vacuum");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Analyzes the sqlite database. Shouldn't need this but will be nice for indexes
    pub fn analyze(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "analyze");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    /// Convience function to get a list of files that are images
    pub fn extensions_images_get_fileid(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "extensions_images_get_fileid"
        );
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// Convience function to get a list of files that are videos
    pub fn extensions_videos_get_fileid(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "extensions_videos_get_fileid"
        );
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// Gets an ID if a extension string exists
    pub fn extension_get_string(
        &self,
        ext_id: &u64,
    ) -> Result<Option<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "extension_get_string");
        let res = ureq::post(url)
            .send_json(&(ext_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<String>>()?;
        Ok(res)
    }
    /// Gets a fileid from a hash
    pub fn file_get_hash(&self, hash: &String) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_hash");
        let res = ureq::post(url)
            .send_json(&(hash))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// Gets a file from storage from its id
    pub fn file_get_id(
        &self,
        file_id: &u64,
    ) -> Result<Option<sharedtypes::DbFileStorage>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_id");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<sharedtypes::DbFileStorage>>()?;
        Ok(res)
    }
    /// Returns all file id's loaded in db
    pub fn file_get_list_id(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_list_id");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    ///
    pub fn file_get_list_all(
        &self,
    ) -> Result<HashMap<u64, sharedtypes::DbFileStorage>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_list_all");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashMap<u64, sharedtypes::DbFileStorage>>()?;
        Ok(res)
    }
    /// Gets a tagid from a unique tag and namespace combo
    pub fn tag_get_name(
        &self,
        tag: String,
        namespace: u64,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_get_name");
        let res = ureq::post(url)
            .send_json(&(tag, namespace))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// Gets a tagid from a tagobject
    pub fn tag_get_name_tagobject(
        &self,
        tagobj: &sharedtypes::DbTagNNS,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_get_name_tagobject");
        let res = ureq::post(url)
            .send_json(&(tagobj))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// db get namespace wrapper
    pub fn namespace_get(&self, namespace: &String) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get");
        let res = ureq::post(url)
            .send_json(&(namespace))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// Returns namespace as a string from an ID returns None if it doesn't exist.
    pub fn namespace_get_string(
        &self,
        ns_id: &u64,
    ) -> Result<Option<sharedtypes::DbNamespaceObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get_string");
        let res = ureq::post(url)
            .send_json(&(ns_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<sharedtypes::DbNamespaceObj>>()?;
        Ok(res)
    }
    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(&self, id: &u64) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get_tagids");
        let res = ureq::post(url)
            .send_json(&(id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// Returns the tags object for each namesapce  for the fileid
    pub fn namespace_get_tags_from_fileid(
        &self,
        ns_id: &u64,
        file_id: &u64,
    ) -> Result<Vec<sharedtypes::DbTagNNS>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "namespace_get_tags_from_fileid"
        );
        let res = ureq::post(url)
            .send_json(&(ns_id, file_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Vec<sharedtypes::DbTagNNS>>()?;
        Ok(res)
    }
    /// Gets all tagids that are in a namespace from a fileid
    pub fn namespace_get_tagids_from_fileid(
        &self,
        ns_id: &u64,
        file_id: &u64,
    ) -> Result<Vec<u64>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "namespace_get_tagids_from_fileid"
        );
        let res = ureq::post(url)
            .send_json(&(ns_id, file_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Vec<u64>>()?;
        Ok(res)
    }
    /// Checks if a tag exists in a namespace
    pub fn namespace_contains_id(
        &self,
        namespace_id: &u64,
        tag_id: &u64,
    ) -> Result<bool, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_contains_id");
        let res = ureq::post(url)
            .send_json(&(namespace_id, tag_id))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<bool>()?;
        Ok(res)
    }
    /// Retuns namespace id's
    pub fn namespace_keys(&self) -> Result<Vec<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_keys");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Vec<u64>>()?;
        Ok(res)
    }
    /// Gets a parent id if they exist
    pub fn parents_get(
        &self,
        parent: &sharedtypes::DbParentsObj,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_get");
        let res = ureq::post(url)
            .send_json(&(parent))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<Option<u64>>()?;
        Ok(res)
    }
    /// Relates the list of relationships assoicated with tag
    pub fn parents_rel_get(&self, relid: &u64) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_rel_get");
        let res = ureq::post(url)
            .send_json(&(relid))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// Relates the list of tags assoicated with relations
    pub fn parents_tag_get(&self, tagid: &u64) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_tag_get");
        let res = ureq::post(url)
            .send_json(&(tagid))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<HashSet<u64>>()?;
        Ok(res)
    }
    /// Returns the location of the file storage path. Helper function
    pub fn location_get(&self) -> Result<String, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "location_get");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<String>()?;
        Ok(res)
    }
    /// commits an exclusive write transaction
    pub fn transaction_flush(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "transaction_flush");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<()>()?;
        Ok(res)
    }
    ///
    pub fn namespace_add(
        &self,
        name: &String,
        description: &Option<String>,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_add");
        let res = ureq::post(url)
            .send_json(&(name, description))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    /// Adds a ns into the db if the id already exists
    pub fn namespace_add_id_exists(
        &self,
        ns: sharedtypes::DbNamespaceObj,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_add_id_exists");
        let res = ureq::post(url)
            .send_json(&(ns))?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
    /// Gets a default namespace id if it doesn't exist
    pub fn create_default_source_url_ns_id(&self) -> Result<u64, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "create_default_source_url_ns_id"
        );
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json::<u64>()?;
        Ok(res)
    }
}
