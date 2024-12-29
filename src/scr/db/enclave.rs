use crate::database::Main;
use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::EnclaveCondition;
use crate::vec_of_strings;
use rusqlite::params;
use rusqlite::OptionalExtension;
use std::collections::BTreeMap;
impl Main {
    ///
    /// Creates a enclave that is for downloading files
    /// Default behaviour is to download to the specified folder unless the input is false
    ///
    pub fn enclave_create_default_file_download(&mut self, location: String) {
        let default_file_enclave = format!("File_Download_location_{}", location);
        self.enclave_name_put(default_file_enclave.clone());
        self.enclave_condition_put("DownloadToDiskAny", sharedtypes::EnclaveCondition::Any);
        self.enclave_action_put(
            &"DownloadToDiskDefault".to_string(),
            sharedtypes::EnclaveAction::DownloadToDefault,
        );

        let enclave_id = self.enclave_name_get_id(&default_file_enclave).unwrap();
        let condition_id = self.enclave_condition_get_id("DownloadToDiskAny").unwrap();
        let action_id = self
            .enclave_action_get_id(&"DownloadToDiskDefault".to_string())
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
    pub fn enclave_name_put(&mut self, name: String) {
        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO Enclave (enclave_name) VALUES (?)")
            .unwrap();
        let _ = prep.insert(params![name]);
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
    /// Adds a condition to the database
    ///
    pub fn enclave_condition_put(&mut self, name: &str, condition: sharedtypes::EnclaveCondition) {
        let conn = self._conn.lock().unwrap();
        let mut prep = conn
            .prepare("INSERT OR REPLACE INTO EnclaveCondition (action_name, action_condition) VALUES (?, ?)")
            .unwrap();
        let _ = prep.insert(params![name, serde_json::to_string(&condition).unwrap()]);
    }
    pub fn enclave_condition_edit() {}
    pub fn enclave_condition_get_id(&self, name: &str) -> Option<usize> {
        let conn = self._conn.lock().unwrap();
        if let Ok(out) = conn
            .query_row(
                "SELECT id from EnclaveCondition where action_name = ?",
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
    /// Adds a conditional action to the list of enclave actions
    ///
    pub fn enclave_action_order_link_put(
        &mut self,
        enclave_id: &usize,
        enclave_conditional_list_id: &usize,
        enclave_action_position: &usize,
    ) {
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
    /// Adds the conditional link between for a condition and it's priority in the enclave list
    ///
    pub fn enclave_condition_link_put(
        &mut self,
        condition_id: &usize,
        action_id: &usize,
        failed_enclave_id: Option<&usize>,
    ) {
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
    /// Inserts the name and action into the database
    ///
    pub fn enclave_action_put(&mut self, name: &String, action: sharedtypes::EnclaveAction) {
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
