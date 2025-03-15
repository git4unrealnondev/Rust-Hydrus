use crate::database::CacheType;
use crate::database::Main;
use log::error;
use rusqlite::params;
pub use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;

impl Main {
    pub fn parents_delete_sql(&mut self, id: &usize) {
        self.parents_delete_tag_id_sql(id);
        self.parents_delete_relate_tag_id_sql(id);
        self.parents_delete_limit_to_sql(id);
    }

    ///
    /// Removes ALL of a tag_id from the parents collumn
    ///
    fn parents_delete_tag_id_sql(&mut self, tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM Parents WHERE tag_id = ?", params![tag_id]);
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    fn parents_delete_relate_tag_id_sql(&mut self, relate_tag_id: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM Parents WHERE relate_tag_id = ?",
            params![relate_tag_id],
        );
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    fn parents_delete_limit_to_sql(&mut self, limit_to: &usize) {
        let conn = self._conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM Parents WHERE limit_to = ?", params![limit_to]);
    }
}

#[cfg(test)]
mod tests {
    use crate::VERS;
    use std::collections::HashSet;

    use super::*;

    fn setup_default_db() -> Main {
        let mut db = Main::new(None, VERS);
        db.parents_add(1, 2, Some(3), true);
        db
    }

    #[test]
    fn sql_parents_add() {
        let db = setup_default_db();
        let mut hs: HashSet<usize> = HashSet::new();
        hs.insert(2);
        let mut ts: HashSet<usize> = HashSet::new();
        ts.insert(1);

        let mut hs: HashSet<usize> = HashSet::new();
        hs.insert(2);
        assert_eq!(db.parents_rel_get(&1), Some(hs));
        assert_eq!(db.parents_tag_get(&2), Some(ts));
    }
    #[test]
    fn sql_parents_del_tag_id() {}
    #[test]
    fn sql_parents_del_relate_tag_id() {}
    #[test]
    fn sql_parents_del_limit_to() {}
}
