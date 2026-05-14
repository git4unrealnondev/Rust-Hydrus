use crate::database::database::Main;
use crate::sharedtypes;
use std::collections::HashMap;

impl Main {
    fn add_tags_to_fileid_smart(&self, file_id: u64, tag_actions: &[sharedtypes::FileTagAction]) {
        let mut tags_to_add = Vec::new();
        let mut tag_relationships_to_add = Vec::new();
        let mut namespaces_to_add = Vec::new();

        let mut namespaces_cache = HashMap::new();
        let mut tag_ids_cache = HashMap::new();

        for tag_action in tag_actions {
            match tag_action.operation {
                sharedtypes::TagOperation::Add => {
                    // Simple add operation
                    for tag in &tag_action.tags {
                        if matches!(
                            tag.tag_type,
                            sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex
                        ) {
                            let nsid = if let Some(id) = namespaces_cache.get(&tag.namespace) {
                                *id
                            } else if let Some(id) = self.namespace_get(&tag.namespace.name) {
                                namespaces_cache.insert(tag.namespace.clone(), id);
                                id
                            } else {
                                namespaces_to_add.push(tag.namespace.clone());
                                tags_to_add.push(tag);
                                continue;
                            };

                            let tag_id = if let Some(id) = tag_ids_cache.get(tag) {
                                *id
                            } else if let Some(id) = self.tag_get_name(tag.tag.clone(), nsid) {
                                tag_ids_cache.insert(tag, id);
                                id
                            } else {
                                tags_to_add.push(tag);
                                continue;
                            };

                            tag_relationships_to_add.push(tag_id);
                        }
                    }
                }

                sharedtypes::TagOperation::Set => {
                    /*        // 1️⃣ Build parser tags grouped by namespace
                    let mut namespace_tags: HashMap<String, Vec<u64>> = HashMap::new();
                    for tag in &tag_action.tags {
                        // Ignore special tag types
                        if !matches!(
                            tag.tag_type,
                            sharedtypes::TagType::Normal | sharedtypes::TagType::NormalNoRegex
                        ) {
                            continue;
                        }

                        if let Some(tag_id) = self.tag_add_tagobject_internal(tn, tag) {
                            namespace_tags
                                .entry(tag.namespace.name.clone())
                                .or_default()
                                .push(tag_id);
                        }
                    }

                    // 2️⃣ Fetch current tags for file grouped by namespace
                    let mut namespace_file_tags: HashMap<String, Vec<u64>> = HashMap::new();
                    for tag_id in self.relationship_get_tagid(&file_id) {
                        if let Some(tag_obj) = self.tag_id_get(&tag_id) {
                            if let Some(namespace) = self.namespace_get_string(&tag_obj.namespace) {
                                namespace_file_tags
                                    .entry(namespace.name.clone())
                                    .or_default()
                                    .push(tag_id);
                            }
                        }
                    }

                    // 3️⃣ Remove ignored namespaces
                    for ignored in ["source_url", ""] {
                        namespace_tags.remove(ignored);
                        namespace_file_tags.remove(ignored);
                    }

                    // 4️⃣ Synchronize tags
                    for (namespace, parser_tags) in &namespace_tags {
                        let file_tags = namespace_file_tags.get_mut(namespace);

                        // Convert parser_tags to a HashSet for efficient lookup
                        let parser_tag_set: HashSet<_> = parser_tags.iter().copied().collect();

                        match file_tags {
                            Some(file_tags) => {
                                // a) Add new tags from parser that aren't already in file
                                for &tag_id in parser_tags {
                                    if !file_tags.contains(&tag_id) {
                                        /*  logging::log(format!(
                                            "Adding tag_id {} to file_id {} per scraper",
                                            tag_id, file_id
                                        ));*/
                                        self.add_relationship_sql(tn, &file_id, &tag_id);
                                        file_tags.push(tag_id); // Update in-memory vector
                                    }
                                }

                                // b) Remove tags from file that are no longer in parser
                                let to_remove: Vec<_> = file_tags
                                    .iter()
                                    .filter(|&&tag_id| !parser_tag_set.contains(&tag_id))
                                    .copied()
                                    .collect();

                                for tag_id in to_remove {
                                    /*  logging::log(format!(
                                        "Removing tag_id {} from file_id {} per scraper",
                                        tag_id, file_id
                                    ));*/
                                    self.delete_relationship_sql(tn, &file_id, &tag_id);
                                    file_tags.retain(|&id| id != tag_id); // Update in-memory vector
                                }
                            }

                            None => {
                                // Namespace doesn't exist for this file, just add all parser tags
                                for &tag_id in parser_tags {
                                    /*  logging::log(format!(
                                        "Adding tag_id {} to file_id {} per scraper",
                                        tag_id, file_id
                                    ));*/
                                    self.add_relationship_sql(tn, &file_id, &tag_id);
                                }
                            }
                        }
                    }*/
                }

                sharedtypes::TagOperation::Del => {
                    /*       let file_id = match file_id {
                        Some(fid) => fid,
                        None => return,
                    };
                    for tag in tag_action.tags.iter() {
                        if let Some(ns_id) = self.namespace_get(&tag.namespace.name) {
                            if let Some(tag_id) = self.tag_get_name(tag.tag.clone(), ns_id) {
                                logging::log(format!(
                                    "Removing tag_id {} to file_id {} per scraper",
                                    tag_id, file_id
                                ));

                                self.delete_relationship_sql(tn, &file_id, &tag_id);
                            }
                        }
                    }*/
                }
            }
        }
    }
}
