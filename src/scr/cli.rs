extern crate clap;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::Path;
//use std::str::pattern::Searcher;

use std::{str::FromStr, task::Wake};

use crate::download;
use crate::{
    database, logging, pause, scraper,
    sharedtypes::{self, AllFields, JobsAdd, JobsRemove},
};
use clap::{Arg, Parser};
use log::{error, info};
//use super::sharedtypes::;

use strum::IntoEnumIterator;

mod cli_structs;

///
/// Returns the main argument and parses data.
///
pub fn main(data: &mut database::Main, scraper: &mut scraper::ScraperManager) {
    let args = cli_structs::MainWrapper::parse();

    if let None = &args.a {
        return;
    }

    // Loads settings into DB.
    data.load_table(&sharedtypes::LoadDBTable::Settings);

    match &args.a.as_ref().unwrap() {
        cli_structs::test::Job(jobstruct) => match jobstruct {
            cli_structs::JobStruct::Add(addstruct) => {
                data.load_table(&sharedtypes::LoadDBTable::Jobs);
                let comtype = sharedtypes::CommitType::from_str(&addstruct.committype);
                match comtype {
                    Ok(comfinal) => {
                        let jobs_add = sharedtypes::JobsAdd {
                            site: addstruct.site.to_string(),
                            query: addstruct.query.to_string(),
                            time: addstruct.time.to_string(),
                            committype: comfinal,
                        };

                        data.jobs_add_new(
                            &jobs_add.site,
                            &jobs_add.query,
                            &jobs_add.time,
                            Some(jobs_add.committype),
                            true,
                            sharedtypes::DbJobType::Params,
                        );
                    }
                    Err(_) => {
                        let enum_vec = sharedtypes::CommitType::iter().collect::<Vec<_>>();
                        println!(
                            "Could not parse commit type. Expected one of {:?}",
                            enum_vec
                        );
                        //return sharedtypes::AllFields::Nothing;
                    }
                }
            }
            cli_structs::JobStruct::Remove(remove) => {
                /*return sharedtypes::AllFields::JobsRemove(sharedtypes::JobsRemove {
                    site: remove.site.to_string(),
                    query: remove.query.to_string(),
                    time: remove.time.to_string(),
                })*/
            }
        },
        cli_structs::test::Search(searchstruct) => match searchstruct {
            cli_structs::SearchStruct::fid(id) => {}
            cli_structs::SearchStruct::tid(id) => {}
            cli_structs::SearchStruct::tag(tag) => {}
            cli_structs::SearchStruct::hash(hash) => {
                data.load_table(&sharedtypes::LoadDBTable::Files);
                data.load_table(&sharedtypes::LoadDBTable::Relationship);
                data.load_table(&sharedtypes::LoadDBTable::Tags);
                let file_id = data.file_get_hash(&hash.hash);
                match file_id {
                    None => {
                        println!("Cannot find hash in db: {}", &hash.hash);
                    }
                    Some(fid) => {
                        let hstags = data.relationship_get_tagid(fid);
                        match hstags {
                            None => {
                                println!(
                                    "Cannot find any loaded relationships for fileid: {}",
                                    &fid
                                );
                            }
                            Some(tags) => {
                                for tid in tags.iter() {
                                    let tag = data.tag_id_get(tid);
                                    match tag {
                                        None => {
                                            println!(
                                                "WANRING CORRUPTION DETECTED for tagid: {}",
                                                &tid
                                            );
                                        }
                                        Some(tagnns) => {
                                            println!(
                                                "Tag: {} namespace: {}",
                                                tagnns.name, tagnns.namespace
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                panic!();
            }
        },
        cli_structs::test::Tasks(taskstruct) => match taskstruct {
            cli_structs::TasksStruct::Reimport(reimp) => match reimp {
                cli_structs::Reimport::DirectoryLocation(loc) => {
                    if !Path::new(&loc.location).exists() {
                        println!("Couldn't find location: {}", &loc.location);
                        return;
                    }
                    // Loads the scraper info for parsing.
                    let scraperlibrary = scraper.return_libloading_string(&loc.site);
                    let libload = match scraperlibrary {
                        None => {
                            println!("Cannot find a loaded scraper. {}", &loc.site);
                            return;
                        }
                        Some(load) => load,
                    };
                    data.load_table(&sharedtypes::LoadDBTable::Tags);
                    data.load_table(&sharedtypes::LoadDBTable::Files);
                    data.load_table(&sharedtypes::LoadDBTable::Relationship);

                    let mut failedtoparse: HashSet<String> = HashSet::new();

                    let FileRegen = crate::scraper::ScraperFileRegen(libload);

                    std::env::set_var("RAYON_NUM_THREADS", "50");

                    println!("Found location: {} Starting to process.", &loc.location);
                    //dbg!(&loc.site, &loc.location);
                    let mut cnt = 0;
                    'walkloop: for each in jwalk::WalkDir::new(&loc.location)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|z| z.file_type().is_file())
                    {
                        //println!("{}", each.path().display());
                        //println!("On file: {}", cnt);
                        let (fhist, b) = download::hash_file(
                            &each.path().display().to_string(),
                            &FileRegen.hash,
                        );

                        println!("File Hash: {}", &fhist);
                        // Tries to infer the type from the ext.
                        let ext = infer::get(&b);

                        // Error handling if we can't parse the filetyp
                        let ext = match ext {
                            None => {
                                failedtoparse.insert(each.path().display().to_string());

                                continue 'walkloop;
                            }
                            Some(ex) => ex,
                        };
                        // parses the info into something the we can use for the scraper
                        let scraperinput = sharedtypes::ScraperFileInput {
                            hash: Some(fhist),
                            ext: Some(ext.extension().to_string()),
                        };

                        let tag = crate::scraper::ScraperFileRetrun(libload, &scraperinput);
                        // gets sha 256 from the file.
                        let (sha2, _a) = download::hash_bytes(
                            &b,
                            &sharedtypes::HashesSupported::Sha256("".to_string()),
                        );
                        let filesloc = data
                            .settings_get_name(&"FilesLoc".to_string())
                            .unwrap()
                            .param
                            .as_ref()
                            .unwrap()
                            .to_owned();
                        // Adds data into db
                        let fid = data.file_add(
                            None,
                            &sha2,
                            &ext.extension().to_string(),
                            &filesloc,
                            true,
                        );
                        let nid =
                            data.namespace_add(tag.namespace.name, tag.namespace.description, true);
                        let tid = data.tag_add(tag.tag, nid, true, None);
                        data.relationship_add(fid, tid, true);
                        cnt += 1;
                        //println!("FIle: {}", each.path().display());
                    }
                    data.transaction_flush();
                    println!("done");
                    if failedtoparse.len() >= 1 {
                        println!("We've got failed items.: {}", failedtoparse.len());
                        for ke in failedtoparse.iter() {
                            println!("{}", ke);
                        }
                    }
                }
            },
            cli_structs::TasksStruct::Database(db) => {
                match db {
                    cli_structs::Database::CompressDatabase => {
                        data.condese_relationships_tags();
                    }

                    cli_structs::Database::RemoveWhereNot(db_n_rmv) => {
                        let ns_id = match db_n_rmv {
                            cli_structs::NamespaceInfo::NamespaceString(ns) => {
                                data.load_table(&sharedtypes::LoadDBTable::Namespace);
                                let db_id = match data.namespace_get(&ns.namespace_string).cloned()
                                {
                                    None => {
                                        logging::info_log(&format!(
                                            "Cannot find the tasks remove string in namespace {}",
                                            &ns.namespace_string
                                        ));
                                        return;
                                    }
                                    Some(id) => id,
                                };
                                db_id
                            }
                            cli_structs::NamespaceInfo::NamespaceId(ns) => ns.namespace_id,
                        };
                        logging::info_log(&format!(
                            "Found Namespace: {} Removing all but id...",
                            &ns_id
                        ));
                        data.load_table(&sharedtypes::LoadDBTable::Tags);
                        data.load_table(&sharedtypes::LoadDBTable::Relationship);
                        data.load_table(&sharedtypes::LoadDBTable::Parents);
                        //data.namespace_get(inp)

                        let mut key = data.namespace_keys();
                        key.retain(|x| *x != ns_id);
                        for each in key {
                            data.delete_namespace_sql(&each);
                        }

                        data.drop_recreate_ns(&ns_id);

                        panic!();
                    }

                    // Removing db namespace. Will get id to remove then remove it.
                    cli_structs::Database::Remove(db_rmv) => {
                        let ns_id = match db_rmv {
                            cli_structs::NamespaceInfo::NamespaceString(ns) => {
                                data.load_table(&sharedtypes::LoadDBTable::Namespace);
                                let db_id = match data.namespace_get(&ns.namespace_string).cloned()
                                {
                                    None => {
                                        logging::info_log(&format!(
                                            "Cannot find the tasks remove string in namespace {}",
                                            &ns.namespace_string
                                        ));
                                        return;
                                    }
                                    Some(id) => id,
                                };
                                db_id
                            }
                            cli_structs::NamespaceInfo::NamespaceId(ns) => ns.namespace_id,
                        };
                        logging::info_log(&format!("Found Namespace: {} Removing...", &ns_id));
                        data.load_table(&sharedtypes::LoadDBTable::Tags);
                        data.load_table(&sharedtypes::LoadDBTable::Relationship);
                        data.namespace_delete_id(&ns_id);
                    }
                }
            }
            cli_structs::TasksStruct::Csv(csvstruct) => {}
        },
    }
    //AllFields::Nothing
}
