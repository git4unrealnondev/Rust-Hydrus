use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::collections::BTreeMap;
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
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: PathBuf = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a plugin folder. If it doesn't exist then please create it in db
    pub fn loaded_plugin_folder(&self) -> Result<PathBuf, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "loaded_plugin_folder");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: PathBuf = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Deletes a namespace by id
    pub fn delete_namespace_id(&self, nsid: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "delete_namespace_id");
        let payload = bitcode::serialize(&(nsid))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(action))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Checks relationships with table for any dead tagids
    pub fn check_relationship_tag_relations(&self) -> Result<(), ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "check_relationship_tag_relations"
        );
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Removes a job from the database by id. Removes from both memdb and sql.
    pub fn del_from_jobs_byid(&self, id: Option<u64>) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "del_from_jobs_byid");
        let payload = bitcode::serialize(&(id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn file_add(
        &self,
        file: sharedtypes::DbFileStorage,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_add");
        let payload = bitcode::serialize(&(file))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn storage_put(&self, location: &String) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "storage_put");
        let payload = bitcode::serialize(&(location))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Adds tags to fileid  commits to db
    pub fn add_tags_to_fileid(
        &self,
        file_id: Option<u64>,
        tag_actions: &Vec<sharedtypes::FileTagAction>,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "add_tags_to_fileid");
        let payload = bitcode::serialize(&(file_id, tag_actions))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn delete_tag(&self, tag: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "delete_tag");
        let payload = bitcode::serialize(&(tag))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn parents_tagid_remove(
        &self,
        tagid: &u64,
    ) -> Result<HashSet<sharedtypes::DbParentsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_tagid_remove");
        let payload = bitcode::serialize(&(tagid))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<sharedtypes::DbParentsObj> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Adds relationship into db
    pub fn add_relationship(&self, file: &u64, tag: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "add_relationship");
        let payload = bitcode::serialize(&(file, tag))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn delete_relationship(&self, file: &u64, tag: &u64) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "delete_relationship");
        let payload = bitcode::serialize(&(file, tag))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(file_id, tag_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: bool = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Adds the tag to the db. commits on finish
    pub fn tag_add_tagobject(
        &self,
        tag: &sharedtypes::TagObject,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_add_tagobject");
        let payload = bitcode::serialize(&(tag))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(tag_list))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// condesnes everything in db
    pub fn condense_db_all(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "condense_db_all");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Sets a relationship between a fileid old and new tagid
    pub fn condense_tags(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "condense_tags");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Sets a relationship between a fileid old and new tagid
    pub fn migrate_tag(
        &self,
        old_tag_id: &u64,
        new_tag_id: &u64,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "migrate_tag");
        let payload = bitcode::serialize(&(old_tag_id, new_tag_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(file_id, old_tag_id, new_tag_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Removes a parent selectivly
    pub fn parents_selective_remove(
        &self,
        parentobj: &sharedtypes::DbParentsObj,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_selective_remove");
        let payload = bitcode::serialize(&(parentobj))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Adds a parent into the db
    pub fn parents_add(
        &self,
        par: sharedtypes::DbParentsObj,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_add");
        let payload = bitcode::serialize(&(par))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(tags, namespace, id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Checks if table is loaded in mem and if not then loads it.
    pub fn load_table(
        &self,
        table: &sharedtypes::LoadDBTable,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "load_table");
        let payload = bitcode::serialize(&(table))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(name, pretty, num, param))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Adds a dead url into the db
    pub fn add_dead_url(&self, url_string: &String) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "add_dead_url");
        let payload = bitcode::serialize(&(url_string))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(search_string, limit_to, fts_or_count))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Vec<(sharedtypes::Tag, u64, u64)> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(search_string, limit_to, fts_or_count))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Vec<(u64, u64)> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// A test function to return 1
    pub fn test(&self) -> Result<u32, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "test");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: u32 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns the db version number
    pub fn db_vers_get(&self) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "db_vers_get");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns a list of loaded tag ids
    pub fn tags_get_list_id(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tags_get_list_id");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// returns file id's based on relationships with a tag
    pub fn relationship_get_fileid(
        &self,
        tag: &u64,
    ) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "relationship_get_fileid");
        let payload = bitcode::serialize(&(tag))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(tag))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns tagid's based on relationship with a fileid.
    pub fn relationship_get_tagid(
        &self,
        file_id: &u64,
    ) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "relationship_get_tagid");
        let payload = bitcode::serialize(&(file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn settings_get_name(
        &self,
        name: &String,
    ) -> Result<Option<sharedtypes::DbSettingObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "settings_get_name");
        let payload = bitcode::serialize(&(name))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<sharedtypes::DbSettingObj> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Correct any weird paths existing inside of the db.
    pub fn check_db_paths(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "check_db_paths");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Backs up the DB file.
    pub fn backup_db(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "backup_db");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /** Returns a files bytes if the file exists. Note if called from intcom then this

 locks the DB while getting the file. One workaround it to use get_file and read

 bytes in manually in seperate thread. that way minimal locking happens.*/
    pub fn get_file_bytes(&self, file_id: &u64) -> Result<Option<Vec<u8>>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "get_file_bytes");
        let payload = bitcode::serialize(&(file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<Vec<u8>> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets the location of a file in the file system
    pub fn get_file(&self, file_id: &u64) -> Result<Option<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "get_file");
        let payload = bitcode::serialize(&(file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<String> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn jobs_update_db(
        &self,
        jobs_obj: sharedtypes::DbJobsObj,
    ) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_update_db");
        let payload = bitcode::serialize(&(jobs_obj))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn jobs_add_new(
        &self,
        jobs_obj: sharedtypes::DbJobsObj,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_add_new");
        let payload = bitcode::serialize(&(jobs_obj))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn jobs_add(
        &self,
        id: Option<u64>,
        time: u64,
        reptime: u64,
        priority: u64,
        cachetime: Option<u64>,
        cachechecktype: sharedtypes::JobCacheType,
        site: String,
        param: Vec<sharedtypes::ScraperParam>,
        system_data: BTreeMap<String, String>,
        user_data: BTreeMap<String, String>,
        jobmanager: sharedtypes::DbJobsManager,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_add");
        let payload = bitcode::serialize(
                &(
                    id,
                    time,
                    reptime,
                    priority,
                    cachetime,
                    cachechecktype,
                    site,
                    param,
                    system_data,
                    user_data,
                    jobmanager,
                ),
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///Checks if a url is dead
    pub fn check_dead_url(&self, url_to_check: &String) -> Result<bool, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "check_dead_url");
        let payload = bitcode::serialize(&(url_to_check))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: bool = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets all running jobs in the db
    pub fn jobs_get_isrunning(
        &self,
    ) -> Result<HashSet<sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get_isrunning");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<sharedtypes::DbJobsObj> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns the most likely locations for a file to be at
    pub fn storage_get_likely(&self, file_id: &u64) -> Result<Vec<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "storage_get_likely");
        let payload = bitcode::serialize(&(file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Vec<String> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns all locations currently inside of the db.
    pub fn storage_get_all(&self) -> Result<Vec<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "storage_get_all");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: Vec<String> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(search, limit))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<Vec<u64>> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets all jobs loaded in the db
    pub fn jobs_get_all(
        &self,
    ) -> Result<HashMap<u64, sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get_all");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashMap<u64, sharedtypes::DbJobsObj> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Pull job by id TODO NEEDS TO ADD IN PROPER POLLING FROM DB.
    pub fn jobs_get(
        &self,
        id: &u64,
    ) -> Result<Option<sharedtypes::DbJobsObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "jobs_get");
        let payload = bitcode::serialize(&(id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<sharedtypes::DbJobsObj> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a tag by id
    pub fn tag_id_get(
        &self,
        uid: &u64,
    ) -> Result<Option<sharedtypes::DbTagNNS>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_id_get");
        let payload = bitcode::serialize(&(uid))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<sharedtypes::DbTagNNS> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Vacuums database. cleans everything.
    pub fn vacuum(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "vacuum");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Analyzes the sqlite database. Shouldn't need this but will be nice for indexes
    pub fn analyze(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "analyze");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Convience function to get a list of files that are images
    pub fn extensions_images_get_fileid(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "extensions_images_get_fileid"
        );
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Convience function to get a list of files that are videos
    pub fn extensions_videos_get_fileid(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "extensions_videos_get_fileid"
        );
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets an ID if a extension string exists
    pub fn extension_get_string(
        &self,
        ext_id: &u64,
    ) -> Result<Option<String>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "extension_get_string");
        let payload = bitcode::serialize(&(ext_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<String> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a fileid from a hash
    pub fn file_get_hash(&self, hash: &String) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_hash");
        let payload = bitcode::serialize(&(hash))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a file from storage from its id
    pub fn file_get_id(
        &self,
        file_id: &u64,
    ) -> Result<Option<sharedtypes::DbFileStorage>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_id");
        let payload = bitcode::serialize(&(file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<sharedtypes::DbFileStorage> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns all file id's loaded in db
    pub fn file_get_list_id(&self) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_list_id");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn file_get_list_all(
        &self,
    ) -> Result<HashMap<u64, sharedtypes::DbFileStorage>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "file_get_list_all");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: HashMap<u64, sharedtypes::DbFileStorage> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a tagid from a unique tag and namespace combo
    pub fn tag_get_name(
        &self,
        tag: String,
        namespace: u64,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_get_name");
        let payload = bitcode::serialize(&(tag, namespace))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a tagid from a tagobject
    pub fn tag_get_name_tagobject(
        &self,
        tagobj: &sharedtypes::DbTagNNS,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "tag_get_name_tagobject");
        let payload = bitcode::serialize(&(tagobj))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// db get namespace wrapper
    pub fn namespace_get(&self, namespace: &String) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get");
        let payload = bitcode::serialize(&(namespace))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns namespace as a string from an ID returns None if it doesn't exist.
    pub fn namespace_get_string(
        &self,
        ns_id: &u64,
    ) -> Result<Option<sharedtypes::DbNamespaceObj>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get_string");
        let payload = bitcode::serialize(&(ns_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<sharedtypes::DbNamespaceObj> = bitcode::deserialize(
                &response_bytes,
            )
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets all tag's assocated a singular namespace
    pub fn namespace_get_tagids(&self, id: &u64) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_get_tagids");
        let payload = bitcode::serialize(&(id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(ns_id, file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Vec<sharedtypes::DbTagNNS> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
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
        let payload = bitcode::serialize(&(ns_id, file_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Vec<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Checks if a tag exists in a namespace
    pub fn namespace_contains_id(
        &self,
        namespace_id: &u64,
        tag_id: &u64,
    ) -> Result<bool, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_contains_id");
        let payload = bitcode::serialize(&(namespace_id, tag_id))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: bool = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Retuns namespace id's
    pub fn namespace_keys(&self) -> Result<Vec<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_keys");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: Vec<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a parent id if they exist
    pub fn parents_get(
        &self,
        parent: &sharedtypes::DbParentsObj,
    ) -> Result<Option<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_get");
        let payload = bitcode::serialize(&(parent))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: Option<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Relates the list of relationships assoicated with tag
    pub fn parents_rel_get(&self, relid: &u64) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_rel_get");
        let payload = bitcode::serialize(&(relid))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Relates the list of tags assoicated with relations
    pub fn parents_tag_get(&self, tagid: &u64) -> Result<HashSet<u64>, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "parents_tag_get");
        let payload = bitcode::serialize(&(tagid))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: HashSet<u64> = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Returns the location of the file storage path. Helper function
    pub fn location_get(&self) -> Result<String, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "location_get");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: String = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// commits an exclusive write transaction
    pub fn transaction_flush(&self) -> Result<(), ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "transaction_flush");
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: () = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    ///
    pub fn namespace_add(
        &self,
        name: &String,
        description: &Option<String>,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_add");
        let payload = bitcode::serialize(&(name, description))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Adds a ns into the db if the id already exists
    pub fn namespace_add_id_exists(
        &self,
        ns: sharedtypes::DbNamespaceObj,
    ) -> Result<u64, ureq::Error> {
        let url = format!("{}/{}/{}", self.base_url, "main", "namespace_add_id_exists");
        let payload = bitcode::serialize(&(ns))
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        let response_bytes = ureq::post(url)
            .header("content-type", "application/bitcode")
            .header("accept", "application/bitcode")
            .send(payload)?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
    /// Gets a default namespace id if it doesn't exist
    pub fn create_default_source_url_ns_id(&self) -> Result<u64, ureq::Error> {
        let url = format!(
            "{}/{}/{}", self.base_url, "main", "create_default_source_url_ns_id"
        );
        let response_bytes = ureq::get(url)
            .header("accept", "application/bitcode")
            .call()?
            .into_body()
            .read_to_vec()?;
        let res: u64 = bitcode::deserialize(&response_bytes)
            .map_err(|e| ureq::Error::Other(Box::new(e)))?;
        Ok(res)
    }
}
