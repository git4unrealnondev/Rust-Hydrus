use crate::Arc;
use crate::Mutex;
use crate::logging;
use crate::logging::error_log;
use crate::sharedtypes;
use nohash::IntMap;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use roaring::bitmap::RoaringBitmap;
use rusqlite::Transaction;
use rusqlite::params;
use std::io::Cursor;
use std::ops::BitAndAssign;

use crate::database::database::Main;

/// Gets the cache type
#[derive(Clone, Debug)]
pub enum InternalCacheType {
    Full,
    Popular,
}

pub struct RelationshipStorage {
    file_id: IntMap<u64, RoaringBitmap>,
    tag_id: IntMap<u64, RoaringBitmap>,
    // db: Arc<Mutex<Main>>,
}

impl RelationshipStorage {
    /// Creates a stored object
    pub fn new() -> Self {
        RelationshipStorage {
            file_id: IntMap::default(),
            tag_id: IntMap::default(),
        }
    }

    pub fn recache_roaring(&mut self, tn: &Transaction) {
        self.file_id.clear();
        self.tag_id.clear();

        logging::info_log(format!(
            "Starting to Recache everything inside of roaring cache"
        ));

        tn.execute("DELETE FROM RelationshipRoaringTagid", [])
            .unwrap();
        tn.execute("DELETE FROM RelationshipRoaringFileid", [])
            .unwrap();

        let temp = tn.prepare("SELECT fileid, tagid FROM Relationship");
        if let Ok(mut con) = temp {
            let relationship = con
                .query_map([], |row| {
                    Ok(sharedtypes::DbRelationshipObj {
                        fileid: row.get(0).unwrap(),
                        tagid: row.get(1).unwrap(),
                    })
                })
                .unwrap();
            for each in relationship {
                match each {
                    Ok(res) => {
                        self.relationship_roaring_add(res.fileid, res.tagid);
                    }

                    Err(err) => {}
                }
            }
        }

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
        }
    }

    /// Loads entire relationships into db
    pub fn load_relationship_cache(
        &mut self,
        conn: PooledConnection<SqliteConnectionManager>,
        cachetype: &InternalCacheType,
    ) {
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
        self.tag_id.contains_key(tag_id)
    }

    fn relationship_cache_add_sql(&self, tn: &Transaction, file_id: &u64, tag_id: &u64) {
        if let Some(tag_bitmap) = self.file_id.get(file_id) {
            let mut bytes = vec![];
            tag_bitmap.serialize_into(&mut bytes).unwrap();
            tn.execute("INSERT OR REPLACE INTO RelationshipRoaringFileid (tagid_bitmap) VALUES (?) WHERE fileid = ?", params![bytes, file_id]).unwrap();
        }
        if let Some(file_bitmap) = self.tag_id.get(tag_id) {
            let mut bytes = vec![];
            file_bitmap.serialize_into(&mut bytes).unwrap();
            tn.execute("INSERT OR REPLACE INTO RelationshipRoaringTagid (fileid_bitmap) VALUES (?) WHERE tagid = ?", params![bytes, tag_id]).unwrap();
        }
    }

    ///
    /// Adds a relationship to into cache and adds it to the db aswell
    ///
    pub fn relationship_roaring_add_sql(&mut self, tn: &Transaction, file_id: u64, tag_id: u64) {
        self.relationship_roaring_add(file_id, tag_id);

        self.relationship_cache_add_sql(tn, &file_id, &tag_id);
    }

    ///
    /// Loads the relationships into the internal memory
    ///
    pub fn relationship_roaring_add(&mut self, file_id: u64, tag_id: u64) {
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

    fn internal_search_item(
        &self,
        tag_id_list: &[u64],
        searchtype: sharedtypes::DbSearchTypeEnum,
    ) -> Vec<u64> {
        if tag_id_list.is_empty() {
            return Vec::new();
        }

        // Collect all bitmaps that exist for the given tag IDs
        let bitmaps: Vec<&RoaringBitmap> = tag_id_list
            .iter()
            .filter_map(|tag| self.tag_id.get(tag))
            .collect();

        if bitmaps.is_empty() {
            return Vec::new();
        }

        match searchtype {
            sharedtypes::DbSearchTypeEnum::And => {
                // AND search: smallest bitmap first
                let mut sorted_bitmaps = bitmaps;
                sorted_bitmaps.sort_by_key(|b| b.len());
                let smallest = sorted_bitmaps[0];
                let others = &sorted_bitmaps[1..];

                let mut result = Vec::new();
                for file_id in smallest.iter() {
                    if others.iter().all(|bitmap| bitmap.contains(file_id)) {
                        result.push(file_id.into());
                    }
                }
                result
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

        if let Some(tags) = self.file_id.get(file_id) {
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

    #[test]
    fn roaring_cache_test() {
        let mut storage = RelationshipStorage::new();
        storage.relationship_roaring_add(1, 5);
        storage.relationship_roaring_add(5, 5);
        storage.relationship_roaring_add(5, 1);
        storage.relationship_roaring_add(5, 8);

        assert_eq!(
            storage.relationship_search_fileid_roaring_and(&[]),
            Vec::<u64>::new()
        );
        assert_eq!(
            storage.relationship_search_fileid_roaring_and(&[9999]),
            Vec::<u64>::new()
        );
        assert_eq!(
            storage.relationship_search_fileid_roaring_and(&[5]),
            vec![1, 5]
        );
        assert_eq!(
            storage.relationship_search_fileid_roaring_and(&[5, 1]),
            vec![5]
        );
        assert_eq!(
            storage.relationship_search_fileid_roaring_and(&[8]),
            vec![5]
        );
        assert_eq!(
            storage.relationship_search_fileid_roaring_or(&[8, 5]),
            vec![1, 5]
        );

        assert_eq!(storage.relationship_search_tagid_roaring(&1), &[5]);
        assert_eq!(storage.relationship_search_tagid_roaring(&5), &[1, 5, 8]);
    }
}
