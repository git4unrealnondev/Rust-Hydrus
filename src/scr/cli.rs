extern crate clap;

use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::{collections::HashSet, io::Write};
use strfmt::Format;

// use std::str::pattern::Searcher;
use crate::download;
use crate::{
    database, logging, pause, scraper,
    sharedtypes::{self},
};
use clap::Parser;
use file_format::FileFormat;
use std::sync::{Arc, Mutex};

mod cli_structs;

fn return_jobtypemanager(
    jobtype: Option<sharedtypes::DbJobType>,
    recursion: Option<&cli_structs::DbJobRecreationClap>,
) -> (sharedtypes::DbJobType, sharedtypes::DbJobsManager) {
    let jobtype = match jobtype {
        None => sharedtypes::DbJobType::Params,
        Some(out) => out,
    };
    let jobsmanager = match recursion {
        None => sharedtypes::DbJobsManager {
            jobtype,
            recreation: None,
        },
        Some(recursion) => match recursion {
            cli_structs::DbJobRecreationClap::OnTagId(id) => sharedtypes::DbJobsManager {
                jobtype,
                recreation: Some(sharedtypes::DbJobRecreation::OnTagId(id.id, id.timestamp)),
            },
            cli_structs::DbJobRecreationClap::OnTagExist(tagclap) => sharedtypes::DbJobsManager {
                jobtype,
                recreation: Some(sharedtypes::DbJobRecreation::OnTag(
                    tagclap.name.clone(),
                    tagclap.namespace,
                    tagclap.timestamp,
                )),
            },
            cli_structs::DbJobRecreationClap::AlwaysTime(timestamp) => sharedtypes::DbJobsManager {
                jobtype,
                recreation: Some(sharedtypes::DbJobRecreation::AlwaysTime(
                    timestamp.timestamp,
                    timestamp.count,
                )),
            },
        },
    };
    (jobtype, jobsmanager)
}

fn return_jobtypemanager_old(
    jobtype: Option<sharedtypes::DbJobType>,
    recursion: &cli_structs::DbJobRecreationClap,
) -> (sharedtypes::DbJobType, sharedtypes::DbJobsManager) {
    let jobtype = match jobtype {
        None => sharedtypes::DbJobType::Params,
        Some(out) => out,
    };
    let jobsmanager = match recursion {
        cli_structs::DbJobRecreationClap::OnTagId(id) => sharedtypes::DbJobsManager {
            jobtype,
            recreation: Some(sharedtypes::DbJobRecreation::OnTagId(id.id, id.timestamp)),
        },
        cli_structs::DbJobRecreationClap::OnTagExist(tagclap) => sharedtypes::DbJobsManager {
            jobtype,
            recreation: Some(sharedtypes::DbJobRecreation::OnTag(
                tagclap.name.clone(),
                tagclap.namespace,
                tagclap.timestamp,
            )),
        },
        cli_structs::DbJobRecreationClap::AlwaysTime(timestamp) => sharedtypes::DbJobsManager {
            jobtype,
            recreation: Some(sharedtypes::DbJobRecreation::AlwaysTime(
                timestamp.timestamp,
                timestamp.count,
            )),
        },
    };
    (jobtype, jobsmanager)
}

/// Returns the main argument and parses data.
pub fn main(data: Arc<Mutex<database::Main>>, scraper: Arc<Mutex<scraper::ScraperManager>>) {
    let args = cli_structs::MainWrapper::parse();
    if args.a.is_none() {
        return;
    }

    // Loads settings into DB.
    {
        let mut data = data.lock().unwrap();
        data.load_table(&sharedtypes::LoadDBTable::Settings);
    }
    match &args.a.as_ref().unwrap() {
        cli_structs::test::Job(jobstruct) => {
            match jobstruct {
                cli_structs::JobStruct::Add(addstruct) => {
                    dbg!(&addstruct);
                    let mut system_data = BTreeMap::new();
                    for each in addstruct.system_data.chunks(2) {
                        system_data.insert(each[0].clone(), each[1].clone());
                    }
                    let (jobtype, jobsmanager) =
                        return_jobtypemanager(addstruct.jobtype, addstruct.recursion.as_ref());
                    let mut data = data.lock().unwrap();
                    data.load_table(&sharedtypes::LoadDBTable::Jobs);
                    data.jobs_add(
                        None,
                        crate::time_func::time_secs(),
                        crate::time_func::time_conv(&addstruct.time),
                        addstruct.site.clone(),
                        addstruct.query.clone(),
                        true,
                        addstruct.committype,
                        &jobtype,
                        system_data,
                        BTreeMap::new(),
                        jobsmanager.clone(),
                    );
                }
                cli_structs::JobStruct::AddBulk(addstruct) => {
                    let (jobtype, jobsmanager) =
                        return_jobtypemanager(addstruct.jobtype, addstruct.recursion.as_ref());
                    let mut data = data.lock().unwrap();
                    data.load_table(&sharedtypes::LoadDBTable::Jobs);
                    for bulk in addstruct.bulkadd.iter() {
                        let mut vars = HashMap::new();
                        vars.insert("inject".to_string(), bulk.to_string());
                        let temp = addstruct.query.format(&vars);
                        if let Ok(ins) = temp {
                            data.jobs_add(
                                None,
                                crate::time_func::time_secs(),
                                crate::time_func::time_conv(&addstruct.time),
                                addstruct.site.clone(),
                                ins,
                                true,
                                addstruct.committype,
                                &jobtype,
                                BTreeMap::new(),
                                BTreeMap::new(),
                                jobsmanager.clone(),
                            );
                        }
                    }
                }
                cli_structs::JobStruct::Remove(_remove) => {
                    // return sharedtypes::AllFields::JobsRemove(sharedtypes::JobsRemove { site:
                    // remove.site.to_string(), query: remove.query.to_string(), time:
                    // remove.time.to_string(), })
                }
            }
        }
        cli_structs::test::Search(searchstruct) => match searchstruct {
            cli_structs::SearchStruct::Parent(parent) => {
                let mut data = data.lock().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::Parents);
                data.load_table(&sharedtypes::LoadDBTable::Tags);
                match data.tag_get_name(parent.tag.clone(), parent.namespace) {
                    None => {
                        dbg!("Cannot find tag.");
                    }
                    Some(tid) => {
                        dbg!("rel_get");

                        // let mut col = Vec::new(); let mut ucol = Vec::new();
                        if let Some(rel) = data.parents_rel_get(tid) {
                            for each in rel.iter() {
                                dbg!(each, data.tag_id_get(each).unwrap());
                            }
                        }
                        dbg!("tag_get");
                        if let Some(rel) = data.parents_tag_get(tid) {
                            for each in rel.iter() {
                                dbg!(each, data.tag_id_get(each).unwrap());
                            }
                        }
                    }
                }
            }
            cli_structs::SearchStruct::Fid(id) => {
                let mut data = data.lock().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let hstags = data.relationship_get_tagid(&id.id);
                match hstags {
                    None => {
                        println!(
                            "Cannot find any loaded relationships for fileid: {}",
                            &id.id
                        );
                    }
                    Some(tags) => {
                        let mut itvec: Vec<usize> = tags.into_iter().collect();
                        itvec.sort();
                        for tid in itvec {
                            let tag = data.tag_id_get(&tid);
                            match tag {
                                None => {
                                    println!("WANRING CORRUPTION DETECTED for tagid: {}", &tid);
                                }
                                Some(tagnns) => {
                                    let ns = data.namespace_get_string(&tagnns.namespace).unwrap();
                                    println!(
                                        "ID {} Tag: {} namespace: {}",
                                        tid, tagnns.name, ns.name
                                    );
                                }
                            }
                        }
                    }
                }
            }
            cli_structs::SearchStruct::Tid(id) => {
                let mut data = data.lock().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let fids = data.relationship_get_fileid(&id.id);
                if let Some(goodfid) = fids {
                    logging::info_log(&"Found Fids:".to_string());
                    for each in goodfid {
                        logging::info_log(&format!("{}", &each));
                    }
                }
            }
            cli_structs::SearchStruct::Tag(tag) => {
                let mut data = data.lock().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let nsid = data.namespace_get(&tag.namespace);
                if let Some(nsid) = nsid {
                    let tid = data.tag_get_name(tag.tag.clone(), *nsid);
                    if let Some(tid) = tid {
                        let fids = data.relationship_get_fileid(tid);
                        if let Some(goodfid) = fids {
                            logging::info_log(&"Found Fids:".to_string());
                            for each in goodfid {
                                logging::info_log(&format!("{}", &each));
                            }
                        } else {
                            logging::info_log(&format!(
                                "Cannot find any relationships for tag id: {}",
                                &tid
                            ));
                        }
                    } else {
                        logging::info_log(&"Cannot find tag :C".to_string());
                    }
                } else {
                    logging::info_log(&"Namespace isn't correct or cannot find it".to_string());
                }
            }
            cli_structs::SearchStruct::Hash(hash) => {
                let mut data = data.lock().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
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
                                let mut tvec = Vec::new();
                                for tid in tags.iter() {
                                    if let Some(tag) = data.tag_id_get(tid) {
                                        tvec.push(tid)
                                    }
                                }
                                tvec.sort();
                                for tid in tvec.iter() {
                                    let tag = data.tag_id_get(tid);
                                    match tag {
                                        None => {
                                            println!(
                                                "WANRING CORRUPTION DETECTED for tagid: {}",
                                                &tid
                                            );
                                        }
                                        Some(tagnns) => {
                                            let ns = data
                                                .namespace_get_string(&tagnns.namespace)
                                                .unwrap();
                                            println!(
                                                "ID {} Tag: {} namespace: {}",
                                                tid, tagnns.name, ns.name
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        cli_structs::test::Tasks(taskstruct) => match taskstruct {
            cli_structs::TasksStruct::Scraper(action) => match action {
                cli_structs::ScraperAction::Test(inp) => {
                    dbg!(&inp);
                }
            },
            cli_structs::TasksStruct::Reimport(reimp) => match reimp {
                cli_structs::Reimport::DirectoryLocation(loc) => {
                    let mut data = data.lock().unwrap();
                    if !Path::new(&loc.location).exists() {
                        println!("Couldn't find location: {}", &loc.location);
                        return;
                    }
                    let scraper = scraper.lock().unwrap();
                    // Loads the scraper info for parsing.
                    let scraperlibrary = scraper.return_libloading_string(&loc.site);
                    let libload = match scraperlibrary {
                        None => {
                            println!("Cannot find a loaded scraper. {}", &loc.site);
                            return;
                        }
                        Some(load) => load,
                    };
                    data.load_table(&sharedtypes::LoadDBTable::All);
                    let failedtoparse: HashSet<String> = HashSet::new();
                    let file_regen = crate::scraper::scraper_file_regen(libload);
                    std::env::set_var("RAYON_NUM_THREADS", "50");
                    println!("Found location: {} Starting to process.", &loc.location);

                    // dbg!(&loc.site, &loc.location);
                    for each in jwalk::WalkDir::new(&loc.location)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|z| z.file_type().is_file())
                    {
                        // println!("{}", each.path().display()); println!("On file: {}", cnt);
                        let (fhist, b) = match download::hash_file(
                            &each.path().display().to_string(),
                            &file_regen.hash,
                        ) {
                            Ok(out) => out,
                            Err(err) => {
                                logging::info_log(&format!(
                                    "Cannot hash file {} err: {:?}",
                                    &each.path().display().to_string(),
                                    err
                                ));
                                continue;
                            }
                        };
                        println!("File Hash: {}", &fhist);

                        // Tries to infer the type from the ext.
                        let ext = FileFormat::from_bytes(&b).extension().to_string();

                        // Error handling if we can't parse the filetyp parses the info into something the
                        // we can use for the scraper
                        let scraperinput = sharedtypes::ScraperFileInput {
                            hash: Some(fhist),
                            ext: Some(ext.clone()),
                        };
                        let tag = crate::scraper::scraper_file_return(libload, &scraperinput);

                        // gets sha 256 from the file.
                        let (sha2, _a) = download::hash_bytes(
                            &b,
                            &sharedtypes::HashesSupported::Sha512("".to_string()),
                        );
                        let filesloc = data.location_get();
                        data.storage_put(&filesloc);
                        let storage_id = data.storage_get_id(&filesloc).unwrap();

                        let ext_id = data.extension_put_string(&ext);

                        // Adds data into db
                        let file =
                            sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                                hash: sha2,
                                ext_id,
                                storage_id,
                            });
                        let fid = data.file_add(file, true);
                        let nid =
                            data.namespace_add(tag.namespace.name, tag.namespace.description, true);
                        let tid = data.tag_add(&tag.tag, nid, true, None);
                        data.relationship_add(fid, tid, true);
                        // println!("FIle: {}", each.path().display());
                    }
                    data.transaction_flush();
                    println!("done");
                    if !failedtoparse.is_empty() {
                        println!("We've got failed items.: {}", failedtoparse.len());
                        for ke in failedtoparse.iter() {
                            println!("{}", ke);
                        }
                    }
                }
            },
            cli_structs::TasksStruct::Database(db) => {
                use crate::helpers;

                let dbstore = data.clone();
                match db {
                    cli_structs::Database::BackupDB => {
                        // backs up the db. check the location in setting or code if I change anything lol
                        let mut data = data.lock().unwrap();
                        data.backup_db();
                    }
                    cli_structs::Database::CheckFiles => {
                        let mut data = data.lock().unwrap();

                        // This will check files in the database and will see if they even exist.
                        let db_location = data.location_get();
                        let cnt: std::sync::Arc<std::sync::Mutex<usize>> =
                            std::sync::Arc::new(std::sync::Mutex::new(0));
                        data.load_table(&sharedtypes::LoadDBTable::All);
                        if !Path::new("fileexists.txt").exists() {
                            let _ = std::fs::File::create("fileexists.txt");
                        }
                        let fiexist: std::sync::Arc<std::sync::Mutex<HashSet<usize>>> =
                            std::sync::Arc::new(std::sync::Mutex::new(
                                std::fs::read_to_string("fileexists.txt")
                                    // panic on possible file-reading errors
                                    .unwrap()
                                    // split the string into an iterator of string slices
                                    .lines()
                                    // make each slice into a string
                                    .map(|x| x.parse::<usize>().unwrap())
                                    .collect(),
                            ));
                        let f = std::sync::Arc::new(std::sync::Mutex::new(
                            std::fs::File::options()
                                .append(true)
                                .open("fileexists.txt")
                                .unwrap(),
                        ));
                        let lis = data.file_get_list_all();
                        println!("Files do not exist:");
                        let mut nsid: Option<usize> = None;
                        {
                            let nso = data.namespace_get(&"source_url".to_owned());
                            if let Some(ns) = nso {
                                nsid = Some(*ns);
                            }
                        }

                        // Spawn default ratelimiter of 1 item per second
                        let ratelimiter_obj = Arc::new(Mutex::new(download::ratelimiter_create(
                            1,
                            std::time::Duration::from_secs(1),
                        )));
                        lis.par_iter().for_each(|(fid, storage)| {
                            if fiexist.lock().unwrap().contains(fid) {
                                return;
                            }
                            let file = match storage {
                                sharedtypes::DbFileStorage::NoExistUnknown => return,
                                sharedtypes::DbFileStorage::NoExist(_) => return,
                                sharedtypes::DbFileStorage::NoIdExist(_) => return,
                                sharedtypes::DbFileStorage::Exist(file) => file,
                            };
                            let loc = helpers::getfinpath(&db_location, &file.hash);
                            let lispa = format!("{}/{}", loc, file.hash);
                            *cnt.lock().unwrap() += 1;
                            if *cnt.lock().unwrap() == 1000 {
                                let _ = f.lock().unwrap().flush();
                                *cnt.lock().unwrap() = 0;
                            }
                            if !Path::new(&lispa).exists() {
                                println!("{}", &file.hash);
                                if nsid.is_some() {
                                    if let Some(rel) = data.relationship_get_tagid(fid) {
                                        for eachs in rel.iter() {
                                            let dat = data.tag_id_get(eachs).unwrap();
                                            logging::info_log(&format!(
                                                "Got Tag: {} for fileid: {}",
                                                dat.name, fid
                                            ));
                                            if dat.namespace == nsid.unwrap() {
                                                let client = download::client_create();
                                                let mut file = sharedtypes::FileObject {
                                                    source_url: Some(dat.name.clone()),
                                                    hash: sharedtypes::HashesSupported::Sha512(
                                                        file.hash.clone(),
                                                    ),
                                                    tag_list: Vec::new(),
                                                    skip_if: Vec::new(),
                                                };
                                                download::dlfile_new(
                                                    &client,
                                                    dbstore.clone(),
                                                    &mut file,
                                                    &data.location_get(),
                                                    None,
                                                    &ratelimiter_obj,
                                                    &dat.name.clone(),
                                                );
                                            }
                                        }
                                    }
                                }
                            } else {
                                let fil = std::fs::read(lispa).unwrap();
                                let hinfo = download::hash_bytes(
                                    &bytes::Bytes::from(fil),
                                    &sharedtypes::HashesSupported::Sha512(file.hash.clone()),
                                );
                                if !hinfo.1 {
                                    logging::error_log(&format!(
                                        "BAD HASH: ID: {}  HASH: {}   2ND HASH: {}",
                                        &file.id, &file.hash, hinfo.0
                                    ));
                                    if nsid.is_some() {
                                        if let Some(rel) = data.relationship_get_tagid(fid) {
                                            for eachs in rel.iter() {
                                                let dat = data.tag_id_get(eachs).unwrap();
                                                logging::info_log(&format!(
                                                    "Got Tag: {} for fileid: {}",
                                                    dat.name, fid
                                                ));
                                                if dat.namespace == nsid.unwrap() {
                                                    let client = download::client_create();
                                                    let mut file = sharedtypes::FileObject {
                                                        source_url: Some(dat.name.clone()),
                                                        hash: sharedtypes::HashesSupported::Sha512(
                                                            file.hash.clone(),
                                                        ),
                                                        tag_list: Vec::new(),
                                                        skip_if: Vec::new(),
                                                    };
                                                    download::dlfile_new(
                                                        &client,
                                                        dbstore.clone(),
                                                        &mut file,
                                                        &data.location_get(),
                                                        None,
                                                        &ratelimiter_obj,
                                                        &dat.name.clone(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            fiexist.lock().unwrap().insert(*fid);
                            let fout = format!("{}\n", fid).into_bytes();
                            f.lock().unwrap().write_all(&fout).unwrap();
                        });
                        let _ = std::fs::remove_file("fileexists.txt");
                    }
                    cli_structs::Database::CheckInMemdb => {
                        let mut data = data.lock().unwrap();
                        data.load_table(&sharedtypes::LoadDBTable::Tags);
                        pause();
                    }
                    cli_structs::Database::CompressDatabase => {
                        let mut data = data.lock().unwrap();
                        data.condense_db_all();
                    }
                    cli_structs::Database::RemoveWhereNot(db_n_rmv) => {
                        let mut data = data.lock().unwrap();
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

                        // data.namespace_get(inp)
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
                        let mut data = data.lock().unwrap();
                        data.load_table(&sharedtypes::LoadDBTable::Namespace);
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
            cli_structs::TasksStruct::Csv(_csvstruct) => {}
        },
    }
    // AllFields::Nothing
}
