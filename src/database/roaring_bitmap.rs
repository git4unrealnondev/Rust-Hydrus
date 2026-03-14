use crate::Arc;
use crate::RwLock;
use crate::file;
use crate::logging;
use crate::logging::error_log;
use crate::sharedtypes;
use nohash::IntMap;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use roaring::bitmap::RoaringBitmap;
use rusqlite::Error;
use rusqlite::Transaction;
use rusqlite::params;
use std::io::Cursor;
use std::ops::BitAndAssign;

use crate::Connection;
use crate::database::database::Main;
use std::ops::Deref;
/// Gets the cache type
#[derive(Clone, Debug)]
pub enum InternalCacheType {
    // Will load everything into memory
    Full,
    // Relies on sqlite table for pulls
    Table,
}

pub struct RelationshipStorage {
    file_id: IntMap<u64, RoaringBitmap>,
    tag_id: IntMap<u64, RoaringBitmap>,
    internal_cache: InternalCacheType,
    db: Arc<RwLock<Main>>,
}

impl RelationshipStorage {
    /// Creates a stored object
    pub fn new(db: Arc<RwLock<Main>>) -> Self {
        RelationshipStorage {
            file_id: IntMap::default(),
            tag_id: IntMap::default(),
            internal_cache: InternalCacheType::Table,
            db,
        }
    }

    ///
    /// Removes a relationship
    ///
    pub fn remove_roaring(&mut self, tn: &Transaction, tag_id: &u64, file_id: &u64) {
        match self.internal_cache {
            InternalCacheType::Full => {
                if let Some(tagid_bitmap) = self.file_id.get_mut(file_id) {
                    tagid_bitmap.remove(*tag_id as u32);
                }
                if let Some(fileid_bitmap) = self.tag_id.get_mut(tag_id) {
                    fileid_bitmap.remove(*file_id as u32);
                }
            }
            InternalCacheType::Table => {}
        }

        if let Some(mut tag_bitmap) = self.relationship_cache_fileid_get(tn, file_id) {
            tag_bitmap.remove(*tag_id as u32);

            self.relationship_cache_add_fileid_sql(tn, file_id, &tag_bitmap);
        }
        if let Some(mut file_bitmap) = self.relationship_cache_tagid_get(tn, tag_id) {
            file_bitmap.remove(*file_id as u32);

            self.relationship_cache_add_tagid_sql(tn, tag_id, &file_bitmap);
        }
    }

    pub fn recache_roaring(&mut self, tn: &Transaction) -> Result<(), Error> {
        self.file_id.clear();
        self.tag_id.clear();

        logging::info_log(format!(
            "Starting to Recache everything inside of roaring cache"
        ));

        tn.execute("DELETE FROM RelationshipRoaringTagid", [])
            .unwrap();
        tn.execute("DELETE FROM RelationshipRoaringFileid", [])
            .unwrap();
        let mut processed: u64 = 0;
        let mut stmt = tn.prepare("SELECT CAST(fileid AS INTEGER), CAST(tagid AS INTEGER) FROM Relationship ORDER BY fileid")?;
        let mut rows =
            stmt.query_map([], |row| Ok((row.get::<_, u64>(0).unwrap(), row.get::<_, u64>(1).unwrap()))).unwrap();

        let mut current_fileid: Option<u64> = None;
        let mut bitmap = RoaringBitmap::new();

        while let Some(row) = rows.next() {
            let (fileid, tagid) = row.unwrap();

            if Some(fileid) != current_fileid {
                if let Some(prev_fileid) = current_fileid {
                    self.relationship_cache_add_fileid_sql(tn, &prev_fileid, &bitmap);
                    processed += 1;
                    if processed % 10_000 == 0 {
                        println!("Processed {} fileids...", processed);
                    }
                }
                bitmap.clear();
                current_fileid = Some(fileid);
            }

            bitmap.insert(tagid.try_into().unwrap());
        }

        // Flush last fileid
        if let Some(fileid) = current_fileid {
            self.relationship_cache_add_fileid_sql(tn, &fileid, &bitmap);
        }

        let mut stmt = tn.prepare("SELECT tagid, fileid FROM Relationship ORDER BY tagid")?;
        let mut rows =
            stmt.query_map([], |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?)))?;

        processed = 0;

        let mut current_tagid: Option<u64> = None;
        let mut bitmap = RoaringBitmap::new();

        while let Some(row) = rows.next() {
            let (tagid, fileid) = row.unwrap();

            if Some(tagid) != current_tagid {
                if let Some(prev_tagid) = current_tagid {
                    self.relationship_cache_add_tagid_sql(tn, &prev_tagid, &bitmap);
                    processed += 1;
                    if processed % 10_000 == 0 {
                        println!("Processed {} tagids...", processed);
                    }
                }
                bitmap.clear();
                current_tagid = Some(tagid);
            }

            bitmap.insert(fileid as u32);
        }

        // Flush last tagid
        if let Some(tagid) = current_tagid {
            self.relationship_cache_add_tagid_sql(tn, &tagid, &bitmap);
        }

        Ok(())

        /*
        // Loads int sqlite
        let mut temp = tn
            .prepare("INSERT INTO RelationshipRoaringFileid(fileid, tagid_bitmap) VALUES (?, ?) ")
            .unwrap();
        for fileid in self.file_id.keys() {
            if let Some(bitmap) = self.file_id.get(fileid) {
                let mut bytes: Vec<u8> = Vec::new();
                bitmap.serialize_into(&mut bytes).unwrap();

                temp.execute(params![fileid, bytes]).unwrap();
            }
        }
        let mut temp = tn
            .prepare("INSERT INTO RelationshipRoaringTagid(Tagid, fileid_bitmap) VALUES (?, ?) ")
            .unwrap();
        for fileid in self.tag_id.keys() {
            if let Some(bitmap) = self.tag_id.get(fileid) {
                let mut bytes: Vec<u8> = Vec::new();
                bitmap.serialize_into(&mut bytes).unwrap();

                temp.execute(params![fileid, bytes]).unwrap();
            }
        }*/
    }

    /// Loads entire relationships into db
    pub fn load_relationship_cache(&mut self, cachetype: InternalCacheType) {
        let conn = self.db.read().get_database_connection();

        self.internal_cache = cachetype;

        // No need to load this
        if let InternalCacheType::Table = self.internal_cache {
            return;
        }

        let mut stmt = conn
            .prepare("SELECT fileid, tagid_bitmap FROM RelationshipRoaringFileid")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, u64>(0).unwrap(),     // fileid
                    row.get::<_, Vec<u8>>(1).unwrap(), // tagid_bitmap
                ))
            })
            .unwrap();

        for (fileid, tagid_bitmap) in rows.flatten() {
            if let Ok(bitmap) = RoaringBitmap::deserialize_unchecked_from(Cursor::new(tagid_bitmap))
            {
                self.file_id.insert(fileid, bitmap);
            }
        }
        let mut stmt = conn
            .prepare("SELECT tagid, fileid_bitmap FROM RelationshipRoaringTagid")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, u64>(0).unwrap(),     // tagid
                    row.get::<_, Vec<u8>>(1).unwrap(), // tagid_bitmap
                ))
            })
            .unwrap();

        for (tagid, fileid_bitmap) in rows.flatten() {
            if let Ok(bitmap) =
                RoaringBitmap::deserialize_unchecked_from(Cursor::new(fileid_bitmap))
            {
                self.tag_id.insert(tagid, bitmap);
            }
        }
    }

    ///
    /// Checks if a tagid exists in the cache
    ///
    pub fn relationship_cache_tagid_exists(&self, tag_id: &u64) -> bool {
        match self.internal_cache {
            InternalCacheType::Full => self.tag_id.contains_key(tag_id),
            InternalCacheType::Table => self
                .relationship_cache_tagid_get(&self.db.read().get_database_connection(), tag_id)
                .is_some(),
        }
    }

    fn relationship_cache_tagid_get<C>(&self, tn: &C, tag_id: &u64) -> Option<RoaringBitmap>
    where
        C: Deref<Target = Connection>,
    {
        match self.internal_cache {
            InternalCacheType::Full => {
                return self.tag_id.get(tag_id).cloned();
            }
            InternalCacheType::Table => {
                if let Some(raw_bitmap) = tn
                    .query_row(
                        "SELECT fileid_bitmap FROM RelationshipRoaringTagid WHERE tagid = ?",
                        params![tag_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(None)
                {
                    if let Ok(out) = RoaringBitmap::deserialize_unchecked_from(
                        Cursor::<Vec<u8>>::new(raw_bitmap),
                    ) {
                        return Some(out);
                    }
                }
            }
        }
        None
    }
    fn relationship_cache_fileid_get<C>(&self, tn: &C, file_id: &u64) -> Option<RoaringBitmap>
    where
        C: Deref<Target = Connection>,
    {
        match self.internal_cache {
            InternalCacheType::Full => {
                return self.file_id.get(file_id).cloned();
            }
            InternalCacheType::Table => {
                if let Some(raw_bitmap) = tn
                    .query_row(
                        "SELECT tagid_bitmap FROM RelationshipRoaringFileid WHERE fileid = ?",
                        params![file_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(None)
                {
                    if let Ok(out) = RoaringBitmap::deserialize_unchecked_from(
                        Cursor::<Vec<u8>>::new(raw_bitmap),
                    ) {
                        return Some(out);
                    }
                }
            }
        }
        None
    }

    fn relationship_cache_add_sql(&self, tn: &Transaction, file_id: &u64, tag_id: &u64) {
        if let Some(mut tag_bitmap) = self.relationship_cache_fileid_get(tn, file_id) {
            tag_bitmap.insert(tag_id.clone().try_into().unwrap());
            self.relationship_cache_add_fileid_sql(tn, file_id, &tag_bitmap);
        } else {
            let mut tag_bitmap = RoaringBitmap::new();
            tag_bitmap.insert(tag_id.clone().try_into().unwrap());
            self.relationship_cache_add_fileid_sql(tn, file_id, &tag_bitmap);
        }
        if let Some(mut file_bitmap) = self.relationship_cache_tagid_get(tn, tag_id) {
            file_bitmap.insert(file_id.clone().try_into().unwrap());
            self.relationship_cache_add_tagid_sql(tn, tag_id, &file_bitmap);
        } else {
            let mut file_bitmap = RoaringBitmap::new();
            file_bitmap.insert(file_id.clone().try_into().unwrap());
            self.relationship_cache_add_tagid_sql(tn, tag_id, &file_bitmap);
        }
    }

    fn relationship_cache_add_fileid_sql(
        &self,
        tn: &Transaction,
        file_id: &u64,
        tag_bitmap: &RoaringBitmap,
    ) {
        let mut bytes = vec![];
        tag_bitmap.serialize_into(&mut bytes).unwrap();
        tn.execute(
            "INSERT INTO RelationshipRoaringFileid (fileid, tagid_bitmap) VALUES (?, ?) ON CONFLICT(fileid) DO UPDATE SET tagid_bitmap = excluded.tagid_bitmap",
            params![file_id, bytes],
        )
        .unwrap();
    }

    fn relationship_cache_add_tagid_sql(
        &self,
        tn: &Transaction,
        tag_id: &u64,
        file_bitmap: &RoaringBitmap,
    ) {
        let mut bytes = vec![];
        file_bitmap.serialize_into(&mut bytes).unwrap();
        tn.execute(
            "INSERT INTO RelationshipRoaringTagid (tagid, fileid_bitmap)
     VALUES (?, ?)
     ON CONFLICT(tagid) DO UPDATE SET fileid_bitmap = excluded.fileid_bitmap",
            params![tag_id, bytes],
        )
        .unwrap();
    }

    ///
    /// Adds a relationship to into cache and adds it to the db aswell
    ///
    pub fn relationship_roaring_add_sql(&mut self, tn: &Transaction, file_id: u64, tag_id: u64) {
        self.relationship_roaring_add(tn, file_id, tag_id);

        //self.relationship_cache_add_sql(tn, &file_id, &tag_id);
    }

    ///
    /// Loads the relationships into the internal memory
    ///
    pub fn relationship_roaring_add(&mut self, tn: &Transaction, file_id: u64, tag_id: u64) {
        match self.internal_cache {
            InternalCacheType::Table => {
                self.relationship_cache_add_sql(tn, &file_id, &tag_id);
            }
            InternalCacheType::Full => {
                match self.file_id.get_mut(&file_id) {
                    None => {
                        let mut bitmap = RoaringBitmap::new();
                        bitmap.insert(tag_id.try_into().unwrap());
                        self.file_id.insert(file_id, bitmap);
                    }
                    Some(bitmap) => {
                        bitmap.insert(tag_id.try_into().unwrap());
                    }
                }
                match self.tag_id.get_mut(&tag_id) {
                    None => {
                        let mut bitmap = RoaringBitmap::new();
                        bitmap.insert(file_id.try_into().unwrap());
                        self.tag_id.insert(tag_id, bitmap);
                    }
                    Some(bitmap) => {
                        bitmap.insert(file_id.try_into().unwrap());
                    }
                }
            }
        }
    }

    fn internal_search_item(
        &self,
        tag_id_list: &[u64],
        searchtype: sharedtypes::DbSearchTypeEnum,
    ) -> Vec<u64> {
        if tag_id_list.is_empty() {
            return Vec::new();
        }

        // Collect all bitmaps that exist for the given tag IDs
        let bitmaps: Vec<RoaringBitmap> = tag_id_list
            .iter()
            .filter_map(|tag| {
                self.relationship_cache_tagid_get(&self.db.read().get_database_connection(), tag)
            })
            .collect();

        if bitmaps.is_empty() {
            return Vec::new();
        }

        match searchtype {
            sharedtypes::DbSearchTypeEnum::And => {
                let mut sorted_bitmaps = bitmaps;
                sorted_bitmaps.sort_by_key(|b| b.len());

                let mut result = sorted_bitmaps[0].clone();
                for bitmap in &sorted_bitmaps[1..] {
                    result &= bitmap;
                }

                result.iter().map(|v| v.into()).collect()
            }
            sharedtypes::DbSearchTypeEnum::Or => {
                // OR search: union all bitmaps
                let mut union_bitmap = RoaringBitmap::new();
                for bitmap in bitmaps {
                    union_bitmap |= bitmap;
                }

                let mut out = Vec::new();
                for item in union_bitmap.iter() {
                    out.push(item.into());
                }
                out
            }
        }
    }
    ///
    /// Gets fileids from a list of tagids should be pretty fast
    ///
    pub fn relationship_search_fileid_roaring_and(&self, tag_id_list: &[u64]) -> Vec<u64> {
        self.internal_search_item(tag_id_list, sharedtypes::DbSearchTypeEnum::And)
    }
    ///
    /// Gets fileids from a list of tagids should be pretty fast
    /// Gets tagid OR tagid
    ///
    pub fn relationship_search_fileid_roaring_or(&self, tag_id_list: &[u64]) -> Vec<u64> {
        self.internal_search_item(tag_id_list, sharedtypes::DbSearchTypeEnum::Or)
    }

    ///
    /// Returns the tagids associated with a fileid
    ///
    pub fn relationship_search_tagid_roaring(&self, file_id: &u64) -> Vec<u64> {
        let mut out = Vec::new();

        if let Some(tags) =
            self.relationship_cache_fileid_get(&self.db.read().get_database_connection(), file_id)
        {
            for tag in tags {
                out.push(tag.into());
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import functions from the outer module
    use crate::database::database::test_database::setup_default_db;

    #[test]
    fn roaring_cache_test() {
        for db in setup_default_db() {
            let mut storage = RelationshipStorage::new(Arc::new(RwLock::new(db)));
            //storage.relationship_roaring_add(1, 5);
            //storage.relationship_roaring_add(2, 5);
            //storage.relationship_roaring_add(5, 1);
            //storage.relationship_roaring_add(5, 8);

            dbg!(&storage.file_id, &storage.tag_id);

            assert_eq!(
                storage.relationship_search_fileid_roaring_and(&[]),
                Vec::<u64>::new()
            );
            assert_eq!(
                storage.relationship_search_fileid_roaring_and(&[9999]),
                Vec::<u64>::new()
            );
            assert_eq!(
                storage.relationship_search_fileid_roaring_and(&[2]),
                vec![1]
            );
            assert_eq!(
                storage.relationship_search_fileid_roaring_and(&[2, 1]),
                vec![1]
            );
            assert_eq!(storage.relationship_search_fileid_roaring_or(&[2]), vec![1]);

            assert_eq!(storage.relationship_search_tagid_roaring(&1), &[1]);
            assert_eq!(storage.relationship_search_tagid_roaring(&2), &[1]);
        }
    }
}
