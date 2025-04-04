use crate::database::Main;
use rusqlite::params;

impl Main {
    pub fn parents_delete_sql(&mut self, id: &usize) {
        self.parents_delete_tag_id_sql(id);
        self.parents_delete_relate_tag_id_sql(id);
        self.parents_delete_limit_to_sql(id);
    }

    ///
    /// Removes ALL of a tag_id from the parents collumn
    ///
    pub fn parents_delete_tag_id_sql(&mut self, tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM Parents WHERE tag_id = ?", params![tag_id]);
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    pub fn parents_delete_relate_tag_id_sql(&mut self, relate_tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM Parents WHERE relate_tag_id = ?",
            params![relate_tag_id],
        );
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    pub fn parents_delete_limit_to_sql(&mut self, limit_to: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM Parents WHERE limit_to = ?", params![limit_to]);
    }
}

#[cfg(test)]
mod tests {
    use crate::VERS;
    

    use super::*;

    fn setup_default_db() -> Main {
        let mut db = Main::new(None, VERS);
        db.parents_add(1, 2, Some(3), true);
        db.parents_add(2, 3, Some(4), true);
        db.parents_add(3, 4, Some(5), true);
        db
    }

    #[test]
    fn sql_parents_add() {}
    #[test]
    fn sql_parents_del_tag_id() {}
    #[test]
    fn sql_parents_del_relate_tag_id() {}
    #[test]
    fn sql_parents_del_limit_to() {}
}
