use crate::Arc;
use crate::Mutex;
use crate::sharedtypes;
use nohash::IntMap;
use roaring::bitmap::RoaringBitmap;
use std::ops::BitAndAssign;

use crate::database::database::Main;

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
            //  db,
        }
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
                let tag_id: u64 = tag_id.try_into().unwrap();
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
                let file_id: u64 = file_id.try_into().unwrap();
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
    }
}
