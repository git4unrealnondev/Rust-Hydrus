use std::collections::HashMap;
use std::collections::HashSet;
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
    /** Searches the database using FTS5 allows getting a list of tags and their count based on a

 search string and a limit of tagids to get*/
    pub fn search_tags(
        &self,
        search_string: &String,
        limit_to: &usize,
        fts_or_count: sharedtypes::TagPartialSearchType,
    ) -> Result<Vec<(sharedtypes::Tag, usize, usize)>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "search_tags");
        let res = ureq::post(url)
            .send_json(&(search_string, limit_to, fts_or_count))?
            .body_mut()
            .read_json::<Vec<(sharedtypes::Tag, usize, usize)>>()?;
        Ok(res)
    }
    /** Searches the database using FTS5 allows getting a list of tagids and their count based on a

 search string and a limit of tagids to get*/
    pub fn search_tags_ids(
        &self,
        search_string: &String,
        limit_to: &usize,
        fts_or_count: sharedtypes::TagPartialSearchType,
    ) -> Result<Vec<(usize, usize)>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "search_tags_ids");
        let res = ureq::post(url)
            .send_json(&(search_string, limit_to, fts_or_count))?
            .body_mut()
            .read_json::<Vec<(usize, usize)>>()?;
        Ok(res)
    }
    /// A test function to return 1
    pub fn test(&self) -> Result<u32, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "test");
        let res = ureq::get(url).call()?.body_mut().read_json::<u32>()?;
        Ok(res)
    }
    /// Returns the db version number
    pub fn db_vers_get(&self) -> Result<usize, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "db_vers_get");
        let res = ureq::get(url).call()?.body_mut().read_json::<usize>()?;
        Ok(res)
    }
    /// Returns a list of loaded tag ids
    pub fn tags_get_list_id(&self) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tags_get_list_id");
        let res = ureq::get(url).call()?.body_mut().read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(
        &self,
        tag: &usize,
    ) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "relationship_get_fileid");
        let res = ureq::post(url)
            .send_json(&(tag))?
            .body_mut()
            .read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// Gets one fileid from one tagid
    pub fn relationship_get_one_fileid(
        &self,
        tag: &usize,
    ) -> Result<Option<usize>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "relationship_get_one_fileid"
        );
        let res = ureq::post(url)
            .send_json(&(tag))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(
        &self,
        file_id: &usize,
    ) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "relationship_get_tagid");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .read_json::<HashSet<usize>>()?;
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
            .read_json::<Option<sharedtypes::DbSettingObj>>()?;
        Ok(res)
    }
    /// Correct any weird paths existing inside of the db.
    pub fn check_db_paths(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "check_db_paths");
        let res = ureq::get(url).call()?.body_mut().read_json::<()>()?;
        Ok(res)
    }
    /// Backs up the DB file.
    pub fn backup_db(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "backup_db");
        let res = ureq::get(url).call()?.body_mut().read_json::<()>()?;
        Ok(res)
    }
    /** Returns a files bytes if the file exists. Note if called from intcom then this

 locks the DB while getting the file. One workaround it to use get_file and read

 bytes in manually in seperate thread. that way minimal locking happens.*/
    pub fn get_file_bytes(
        &self,
        file_id: &usize,
    ) -> Result<Option<Vec<u8>>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "get_file_bytes");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .read_json::<Option<Vec<u8>>>()?;
        Ok(res)
    }
    /// Gets the location of a file in the file system
    pub fn get_file(&self, file_id: &usize) -> Result<Option<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "get_file");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .read_json::<Option<String>>()?;
        Ok(res)
    }
    ///Checks if a url is dead
    pub fn check_dead_url(&self, url_to_check: &String) -> Result<bool, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "check_dead_url");
        let res = ureq::post(url)
            .send_json(&(url_to_check))?
            .body_mut()
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
            .read_json::<HashSet<sharedtypes::DbJobsObj>>()?;
        Ok(res)
    }
    /// Returns all locations currently inside of the db.
    pub fn storage_get_all(&self) -> Result<Vec<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "storage_get_all");
        let res = ureq::get(url).call()?.body_mut().read_json::<Vec<String>>()?;
        Ok(res)
    }
    /** Handles the searching of the DB dynamically. Returns the file id's associated

 with the search.

 Returns file IDs matching the search.

 Supports AND, OR, NOT operations.*/
    pub fn search_db_files(
        &self,
        search: sharedtypes::SearchObj,
        limit: Option<usize>,
    ) -> Result<Option<Vec<usize>>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "search_db_files");
        let res = ureq::post(url)
            .send_json(&(search, limit))?
            .body_mut()
            .read_json::<Option<Vec<usize>>>()?;
        Ok(res)
    }
    /// Gets all jobs loaded in the db
    pub fn jobs_get_all(
        &self,
    ) -> Result<HashMap<usize, sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get_all");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .read_json::<HashMap<usize, sharedtypes::DbJobsObj>>()?;
        Ok(res)
    }
    /// Pull job by id TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    pub fn jobs_get(
        &self,
        id: &usize,
    ) -> Result<Option<sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get");
        let res = ureq::post(url)
            .send_json(&(id))?
            .body_mut()
            .read_json::<Option<sharedtypes::DbJobsObj>>()?;
        Ok(res)
    }
    /// Gets a tag by id
    pub fn tag_id_get(
        &self,
        uid: &usize,
    ) -> Result<Option<sharedtypes::DbTagNNS>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_id_get");
        let res = ureq::post(url)
            .send_json(&(uid))?
            .body_mut()
            .read_json::<Option<sharedtypes::DbTagNNS>>()?;
        Ok(res)
    }
    /// Vacuums database. cleans everything.
    pub fn vacuum(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "vacuum");
        let res = ureq::get(url).call()?.body_mut().read_json::<()>()?;
        Ok(res)
    }
    /// Analyzes the sqlite database. Shouldn't need this but will be nice for indexes
    pub fn analyze(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "analyze");
        let res = ureq::get(url).call()?.body_mut().read_json::<()>()?;
        Ok(res)
    }
    /// Convience function to get a list of files that are images
    pub fn extensions_images_get_fileid(&self) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "extensions_images_get_fileid"
        );
        let res = ureq::get(url).call()?.body_mut().read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// Convience function to get a list of files that are videos
    pub fn extensions_videos_get_fileid(&self) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "extensions_videos_get_fileid"
        );
        let res = ureq::get(url).call()?.body_mut().read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// Gets an ID if a extension string exists
    pub fn extension_get_id(&self, ext: &String) -> Result<Option<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "extension_get_id");
        let res = ureq::post(url)
            .send_json(&(ext))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// Gets an ID if a extension string exists
    pub fn extension_get_string(
        &self,
        ext_id: &usize,
    ) -> Result<Option<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "extension_get_string");
        let res = ureq::post(url)
            .send_json(&(ext_id))?
            .body_mut()
            .read_json::<Option<String>>()?;
        Ok(res)
    }
    /// Gets a fileid from a hash
    pub fn file_get_hash(&self, hash: &String) -> Result<Option<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_hash");
        let res = ureq::post(url)
            .send_json(&(hash))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// Gets a file from storage from its id
    pub fn file_get_id(
        &self,
        file_id: &usize,
    ) -> Result<Option<sharedtypes::DbFileStorage>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_id");
        let res = ureq::post(url)
            .send_json(&(file_id))?
            .body_mut()
            .read_json::<Option<sharedtypes::DbFileStorage>>()?;
        Ok(res)
    }
    /// Returns all file id's loaded in db
    pub fn file_get_list_id(&self) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_list_id");
        let res = ureq::get(url).call()?.body_mut().read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    ///
    pub fn file_get_list_all(
        &self,
    ) -> Result<HashMap<usize, sharedtypes::DbFileStorage>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_list_all");
        let res = ureq::get(url)
            .call()?
            .body_mut()
            .read_json::<HashMap<usize, sharedtypes::DbFileStorage>>()?;
        Ok(res)
    }
    /// Gets a tagid from a unique tag and namespace combo
    pub fn tag_get_name(
        &self,
        tag: String,
        namespace: usize,
    ) -> Result<Option<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_get_name");
        let res = ureq::post(url)
            .send_json(&(tag, namespace))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// Gets a tagid from a tagobject
    pub fn tag_get_name_tagobject(
        &self,
        tagobj: &sharedtypes::DbTagNNS,
    ) -> Result<Option<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_get_name_tagobject");
        let res = ureq::post(url)
            .send_json(&(tagobj))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// db get namespace wrapper
    pub fn namespace_get(
        &self,
        namespace: &String,
    ) -> Result<Option<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get");
        let res = ureq::post(url)
            .send_json(&(namespace))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// Returns namespace as a string from an ID returns None if it doesn't exist.
    pub fn namespace_get_string(
        &self,
        ns_id: &usize,
    ) -> Result<Option<sharedtypes::DbNamespaceObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get_string");
        let res = ureq::post(url)
            .send_json(&(ns_id))?
            .body_mut()
            .read_json::<Option<sharedtypes::DbNamespaceObj>>()?;
        Ok(res)
    }
    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(
        &self,
        id: &usize,
    ) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get_tagids");
        let res = ureq::post(url)
            .send_json(&(id))?
            .body_mut()
            .read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// Checks if a tag exists in a namespace
    pub fn namespace_contains_id(
        &self,
        namespace_id: &usize,
        tag_id: &usize,
    ) -> Result<bool, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_contains_id");
        let res = ureq::post(url)
            .send_json(&(namespace_id, tag_id))?
            .body_mut()
            .read_json::<bool>()?;
        Ok(res)
    }
    /// Retuns namespace id's
    pub fn namespace_keys(&self) -> Result<Vec<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_keys");
        let res = ureq::get(url).call()?.body_mut().read_json::<Vec<usize>>()?;
        Ok(res)
    }
    /// Gets a parent id if they exist
    pub fn parents_get(
        &self,
        parent: &sharedtypes::DbParentsObj,
    ) -> Result<Option<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_get");
        let res = ureq::post(url)
            .send_json(&(parent))?
            .body_mut()
            .read_json::<Option<usize>>()?;
        Ok(res)
    }
    /// Relates the list of relationships assoicated with tag
    pub fn parents_rel_get(&self, relid: &usize) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_rel_get");
        let res = ureq::post(url)
            .send_json(&(relid))?
            .body_mut()
            .read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// Relates the list of tags assoicated with relations
    pub fn parents_tag_get(&self, tagid: &usize) -> Result<HashSet<usize>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_tag_get");
        let res = ureq::post(url)
            .send_json(&(tagid))?
            .body_mut()
            .read_json::<HashSet<usize>>()?;
        Ok(res)
    }
    /// Returns the location of the file storage path. Helper function
    pub fn location_get(&self) -> Result<String, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "location_get");
        let res = ureq::get(url).call()?.body_mut().read_json::<String>()?;
        Ok(res)
    }
}
