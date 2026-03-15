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
use rusqlite::OptionalExtension;
use rusqlite::Transaction;
use rusqlite::params;
use rusqlite::params_from_iter;
use std::cmp::Reverse;
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
    // Keeps popular tags loaded in memory and other tags in sqlite
    Popular(u64),
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
            InternalCacheType::Full | InternalCacheType::Popular(_) => {
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

        logging::info_log("Starting to Recache everything inside of roaring cache".to_string());

        let mut processed: u64 = 0;
        tn.execute("DELETE FROM RelationshipRoaringTagid", [])
            .unwrap();
        tn.execute("DELETE FROM RelationshipRoaringFileid", [])
            .unwrap();
        let mut stmt = tn.prepare("SELECT CAST(fileid AS INTEGER), CAST(tagid AS INTEGER) FROM Relationship ORDER BY fileid")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, u64>(0).unwrap(), row.get::<_, u64>(1).unwrap()))
            })
            .unwrap();

        let mut current_fileid: Option<u64> = None;
        let mut bitmap = RoaringBitmap::new();

        for row in rows {
            let (fileid, tagid) = row.unwrap();

            if Some(fileid) != current_fileid {
                if let Some(prev_fileid) = current_fileid {
                    self.relationship_cache_add_fileid_sql(tn, &prev_fileid, &bitmap);
                    processed += 1;
                    if processed.is_multiple_of(10_000) {
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

        let mut stmt = tn.prepare("SELECT CAST(fileid AS INTEGER), CAST(tagid AS INTEGER) FROM Relationship ORDER BY tagid")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?)))?;

        processed = 0;

        let mut current_tagid: Option<u64> = None;
        let mut bitmap = RoaringBitmap::new();

        for row in rows {
            let (fileid, tagid) = row.unwrap();

            if Some(tagid) != current_tagid {
                if let Some(prev_tagid) = current_tagid {
                    self.relationship_cache_add_tagid_sql(tn, &prev_tagid, &bitmap);
                    processed += 1;
                    if processed.is_multiple_of(10_000) {
                        println!("Processed {} tagids...", processed);
                    }
                }
                bitmap.clear();
                current_tagid = Some(tagid);
            }

            bitmap.insert(fileid.try_into().unwrap());
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

        self.internal_cache = cachetype.clone();

        // No need to load this
        if let InternalCacheType::Table = self.internal_cache {
            return;
        }

        let params;
        let sql = match cachetype {
            InternalCacheType::Popular(ref popular_count) => {
                params = vec![popular_count];
                "SELECT tagid, fileid_bitmap FROM RelationshipRoaringTagid WHERE tagid IN (SELECT id FROM Tags WHERE count >= ?)"
            }
            _ => {
                params = vec![];
                "SELECT tagid, fileid_bitmap FROM RelationshipRoaringTagid"
            }
        };

        let mut stmt = conn.prepare(sql).unwrap();
        let rows = stmt
            .query_map(params_from_iter(params), |row| {
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

        logging::info_log("Finished storing tag_id maps for roaring");
        if let InternalCacheType::Popular(_) = self.internal_cache {
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
    }

    ///
    /// Checks if a tagid exists in the cache
    ///
    pub fn relationship_cache_tagid_exists(&self, tag_id: &u64) -> bool {
        match self.internal_cache {
            InternalCacheType::Popular(_) => {
                if self.tag_id.contains_key(tag_id) {
                    return true;
                }
                self.relationship_cache_tagid_get(&self.db.read().get_database_connection(), tag_id)
                    .is_some()
            }
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
            InternalCacheType::Popular(_) => {
                if let Some(bitmap) = self.tag_id.get(tag_id) {
                    return Some(bitmap.clone());
                } else {
                    if let Ok(Some(raw_bitmap)) = tn
                        .query_row(
                            "SELECT fileid_bitmap FROM RelationshipRoaringTagid WHERE tagid = ?",
                            params![tag_id],
                            |row| row.get::<_, Vec<u8>>(0),
                        )
                        .optional()
                    {
                        if let Ok(out) = RoaringBitmap::deserialize_unchecked_from(&raw_bitmap[..])
                        {
                            return Some(out);
                        }
                    }
                }
            }
            InternalCacheType::Full => {
                return self.tag_id.get(tag_id).cloned();
            }
            InternalCacheType::Table => {
                if let Ok(Some(raw_bitmap)) = tn
                    .query_row(
                        "SELECT fileid_bitmap FROM RelationshipRoaringTagid WHERE tagid = ?",
                        params![tag_id],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .optional()
                {
                    if let Ok(out) = RoaringBitmap::deserialize_unchecked_from(&raw_bitmap[..]) {
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
            InternalCacheType::Table | InternalCacheType::Popular(_) => {
                if let Ok(Some(raw_bitmap)) = tn
                    .query_row(
                        "SELECT tagid_bitmap FROM RelationshipRoaringFileid WHERE fileid = ?",
                        params![file_id],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .optional()
                {
                    if let Ok(out) = RoaringBitmap::deserialize_unchecked_from(&raw_bitmap[..]) {
                        return Some(out);
                    }
                }
            }
        }
        None
    }

    fn relationship_cache_add_sql(&self, tn: &Transaction, file_id: &u64, tag_id: &u64) {
        if let Some(mut tag_bitmap) = self.relationship_cache_fileid_get(tn, file_id) {
            tag_bitmap.insert((*tag_id).try_into().unwrap());
            self.relationship_cache_add_fileid_sql(tn, file_id, &tag_bitmap);
        } else {
            let mut tag_bitmap = RoaringBitmap::new();
            tag_bitmap.insert((*tag_id).try_into().unwrap());
            self.relationship_cache_add_fileid_sql(tn, file_id, &tag_bitmap);
        }
        if let Some(mut file_bitmap) = self.relationship_cache_tagid_get(tn, tag_id) {
            file_bitmap.insert((*file_id).try_into().unwrap());
            self.relationship_cache_add_tagid_sql(tn, tag_id, &file_bitmap);
        } else {
            let mut file_bitmap = RoaringBitmap::new();
            file_bitmap.insert((*file_id).try_into().unwrap());
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
    /// Loads the relationships into the internal memory
    ///
    pub fn relationship_roaring_add(&mut self, tn: &Transaction, file_id: u64, tag_id: u64) {
        match self.internal_cache {
            InternalCacheType::Table => {}
            InternalCacheType::Popular(popular_count) => {
                if let Some(tagid_count) = self.db.read().get_count_for_tagid(tn, &tag_id) {
                    if popular_count <= tagid_count {
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
        self.relationship_cache_add_sql(tn, &file_id, &tag_id);
    }

    fn internal_search_item(
        &self,
        tag_id_list: &[u64],
        searchtype: sharedtypes::DbSearchTypeEnum,
    ) -> Option<RoaringBitmap> {
        if tag_id_list.is_empty() {
            return None;
        }

        let conn = self.db.read().get_database_connection();

        // iterator over all non-empty bitmaps
        let mut bitmaps_iter = tag_id_list
            .iter()
            .filter_map(|tag| self.relationship_cache_tagid_get(&conn, tag));

        match searchtype {
            sharedtypes::DbSearchTypeEnum::Or => {
                // fold all into one bitmap
                let result = bitmaps_iter.fold(RoaringBitmap::new(), |mut acc, b| {
                    acc |= b; // in-place union
                    acc
                });

                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }

            sharedtypes::DbSearchTypeEnum::And => {
                // for AND, we need at least one bitmap to start
                let first = bitmaps_iter.next()?;
                // sort not needed here; we could optionally do a single pass if desired
                let result = bitmaps_iter.fold(first, |mut acc, b| {
                    if acc.is_empty() {
                        acc // short-circuit
                    } else {
                        acc &= b; // in-place intersection
                        acc
                    }
                });

                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }
        }
    }
    /*fn internal_search_item(
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

                let mut result = sorted_bitmaps[0];
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
    }*/

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

pub struct SearchQuery<'a> {
    engine: &'a RelationshipStorage,
    offset: Option<u64>,
    limit: Option<u64>,
    and_search: Option<(sharedtypes::DbSearchTypeEnum, &'a [u64])>,
    or_search: Option<(sharedtypes::DbSearchTypeEnum, Vec<u64>)>,
    sort: bool,
}

impl<'a> SearchQuery<'a> {
    pub fn new(engine: &'a RelationshipStorage) -> Self {
        Self {
            engine,
            offset: None,
            limit: None,
            and_search: None,
            or_search: None,
            sort: false,
        }
    }
    pub fn sort(mut self) -> Self {
        self.sort = true;
        self
    }

    pub fn limit(mut self, limit: Option<u64>) -> Self {
        self.limit = limit;
        self
    }
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn and_search(mut self, tag_ids: &'a [u64]) -> Self {
        self.and_search = Some((sharedtypes::DbSearchTypeEnum::And, tag_ids));

        self
    }
    pub fn or_search(mut self, tag_ids: &'a [u64]) -> Self {
        self.and_search = Some((sharedtypes::DbSearchTypeEnum::Or, tag_ids));

        self
    }

    /// Finalizes the search returns applicable fileids
    pub fn build(self) -> Vec<u64> {
        if let Some((searchtype, tag_id_list)) = self.and_search
            && let Some(bitmap) = self.engine.internal_search_item(tag_id_list, searchtype)
        {
            let offset = self.offset.unwrap_or(0) as usize;
            let limit = self.limit.unwrap_or(bitmap.len()) as usize;

            return bitmap
                .iter()
                .rev()
                .skip(offset)
                .take(limit)
                .map(|v| v as u64)
                .collect();
        }

        Vec::new()
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
                SearchQuery::new(&storage).and_search(&[]).build(),
                Vec::<u64>::new()
            );
            assert_eq!(
                SearchQuery::new(&storage).and_search(&[9999]).build(),
                Vec::<u64>::new()
            );
            assert_eq!(SearchQuery::new(&storage).and_search(&[2]).build(), vec![1]);
            assert_eq!(
                SearchQuery::new(&storage).and_search(&[2, 1]).build(),
                vec![1]
            );
            assert_eq!(SearchQuery::new(&storage).or_search(&[2]).build(), vec![1]);

            assert_eq!(storage.relationship_search_tagid_roaring(&1), &[1, 2]);
            assert_eq!(storage.relationship_search_tagid_roaring(&2), &[1]);
        }
    }
}
