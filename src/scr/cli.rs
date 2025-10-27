extern crate clap;

use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::{collections::HashSet, io::Write};
use strfmt::Format;
use walkdir::WalkDir;

use crate::globalload::GlobalLoad;
// use std::str::pattern::Searcher;
use crate::Mutex;
use crate::RwLock;
use crate::download;
use crate::file::{find_sidecar, parse_file};
use crate::sharedtypes::{DEFAULT_CACHECHECK, DEFAULT_CACHETIME, DEFAULT_PRIORITY};
use crate::{
    database, logging, pause,
    sharedtypes::{self},
};
use clap::Parser;
use std::sync::Arc;

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

///
/// Parses strings into NORMAL inputs as scraperparam
///
fn parse_string_to_scraperparam(input: &str) -> Vec<sharedtypes::ScraperParam> {
    let mut out = Vec::new();

    for item in input.split(' ') {
        out.push(sharedtypes::ScraperParam::Normal(item.to_string()));
    }

    out
}

/// Returns the main argument and parses data.
pub fn main(data: Arc<RwLock<database::Main>>, globalload: Arc<RwLock<GlobalLoad>>) {
    //pub fn main(data: Arc<RwLock<database::Main>>, scraper: Arc<RwLock<GlobalLoad>>) {
    let args = cli_structs::MainWrapper::parse();
    if args.a.is_none() {
        return;
    }

    // Loads settings into DB.
    {
        let mut data = data.write().unwrap();
        data.load_table(&sharedtypes::LoadDBTable::Settings);
    }
    match &args.a.as_ref().unwrap() {
        cli_structs::Test::Job(jobstruct) => {
            match jobstruct {
                cli_structs::JobStruct::Add(addstruct) => {
                    dbg!(&addstruct);
                    let mut system_data = BTreeMap::new();
                    for each in addstruct.system_data.chunks(2) {
                        system_data.insert(each[0].clone(), each[1].clone());
                    }
                    let (_jobtype, jobsmanager) =
                        return_jobtypemanager(addstruct.jobtype, addstruct.recursion.as_ref());
                    let mut data = data.write().unwrap();
                    data.load_table(&sharedtypes::LoadDBTable::Jobs);
                    data.jobs_add(
                        None,
                        crate::time_func::time_secs(),
                        crate::time_func::time_conv(&addstruct.time),
                        DEFAULT_PRIORITY,
                        DEFAULT_CACHETIME,
                        DEFAULT_CACHECHECK,
                        addstruct.site.clone(),
                        parse_string_to_scraperparam(&addstruct.query),
                        system_data,
                        BTreeMap::new(),
                        jobsmanager.clone(),
                    );
                }
                cli_structs::JobStruct::AddBulk(addstruct) => {
                    let (_jobtype, jobsmanager) =
                        return_jobtypemanager(addstruct.jobtype, addstruct.recursion.as_ref());
                    let mut data = data.write().unwrap();
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
                                DEFAULT_PRIORITY,
                                DEFAULT_CACHETIME,
                                DEFAULT_CACHECHECK,
                                addstruct.site.clone(),
                                parse_string_to_scraperparam(&ins),
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
        cli_structs::Test::Search(searchstruct) => match searchstruct {
            cli_structs::SearchStruct::Parent(parent) => {
                let mut data = data.write().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::Parents);
                data.load_table(&sharedtypes::LoadDBTable::Tags);
                match &data.tag_get_name(parent.tag.clone(), parent.namespace) {
                    None => {
                        dbg!("Cannot find tag.");
                    }
                    Some(tid) => {
                        dbg!("rel_get");

                        // let mut col = Vec::new(); let mut ucol = Vec::new();
                        for each in data.parents_rel_get(tid).iter() {
                            dbg!(each, data.tag_id_get(each).unwrap());
                        }
                        dbg!("tag_get");
                        for each in data.parents_tag_get(tid).iter() {
                            dbg!(each, data.tag_id_get(each).unwrap());
                        }
                    }
                }
            }
            cli_structs::SearchStruct::Fid(id) => {
                let mut data = data.write().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let hstags = data.relationship_get_tagid(&id.id);
                if hstags.is_empty() {
                    println!(
                        "Cannot find any loaded relationships for fileid: {}",
                        &id.id
                    );
                } else {
                    let mut itvec: Vec<usize> = hstags.into_iter().collect();
                    itvec.sort();
                    for tid in itvec {
                        let tag = data.tag_id_get(&tid);
                        match tag {
                            None => {
                                println!("WANRING CORRUPTION DETECTED for tagid: {}", &tid);
                            }
                            Some(tagnns) => {
                                let ns = data.namespace_get_string(&tagnns.namespace).unwrap();
                                println!("ID {} Tag: {} namespace: {}", tid, tagnns.name, ns.name);
                            }
                        }
                    }
                }
            }
            cli_structs::SearchStruct::Tid(id) => {
                let mut data = data.write().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let fids = data.relationship_get_fileid(&id.id);
                if !fids.is_empty() {
                    logging::info_log("Found Fids:".to_string());
                    for each in fids {
                        logging::info_log(format!("{}", &each));
                    }
                }
            }
            cli_structs::SearchStruct::Tag(tag) => {
                let mut data = data.write().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let nsid = data.namespace_get(&tag.namespace);
                if let Some(nsid) = nsid {
                    let tid = &data.tag_get_name(tag.tag.clone(), nsid);
                    if let Some(tid) = tid {
                        let fids = data.relationship_get_fileid(tid);

                        if fids.is_empty() {
                            logging::info_log(format!(
                                "Cannot find any relationships for tag id: {}",
                                &tid
                            ));
                        } else {
                            logging::info_log("Found Fids:".to_string());
                            for each in fids {
                                logging::info_log(format!("{}", &each));
                            }
                        }
                    } else {
                        logging::info_log("Cannot find tag :C".to_string());
                    }
                } else {
                    logging::info_log("Namespace isn't correct or cannot find it".to_string());
                    logging::info_log("Please use a namespace below:".to_string());
                }
            }
            cli_structs::SearchStruct::Hash(hash) => {
                let mut data = data.write().unwrap();
                data.load_table(&sharedtypes::LoadDBTable::All);
                let file_id = data.file_get_hash(&hash.hash);
                match file_id {
                    None => {
                        println!("Cannot find hash in db: {}", &hash.hash);
                    }
                    Some(fid) => {
                        let hstags = data.relationship_get_tagid(&fid);
                        if hstags.is_empty() {
                            println!("Cannot find any loaded relationships for fileid: {}", &fid);
                        } else {
                            let mut tvec = Vec::new();
                            for tid in hstags.iter() {
                                if data.tag_id_get(tid).is_some() {
                                    tvec.push(tid)
                                }
                            }
                            tvec.sort();
                            for tid in tvec.iter() {
                                let tag = data.tag_id_get(tid);
                                match tag {
                                    None => {
                                        println!("WANRING CORRUPTION DETECTED for tagid: {}", &tid);
                                    }
                                    Some(tagnns) => {
                                        let ns =
                                            data.namespace_get_string(&tagnns.namespace).unwrap();
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
        },
        cli_structs::Test::Tasks(taskstruct) => match taskstruct {
            cli_structs::TasksStruct::Import(directory) => {
                {
                    let mut unwrappy = data.write().unwrap();
                    unwrappy.load_table(&sharedtypes::LoadDBTable::Files);
                    unwrappy.load_table(&sharedtypes::LoadDBTable::Relationship);
                    unwrappy.load_table(&sharedtypes::LoadDBTable::Tags);
                    unwrappy.load_table(&sharedtypes::LoadDBTable::Namespace);
                    unwrappy.load_table(&sharedtypes::LoadDBTable::Parents);
                    unwrappy.enclave_create_default_file_import();
                }

                for local_file in directory.location.iter() {
                    let mut files = HashMap::new();
                    let mut sidecars = HashSet::new();

                    let search_path = Path::new(&local_file);
                    if !search_path.exists() {
                        logging::info_log(format!("Cannot find file or path at: {}", &local_file));
                        return;
                    }

                    if search_path.is_file() {
                        files.insert(search_path.to_path_buf(), find_sidecar(search_path));
                        for sidecar in find_sidecar(search_path) {
                            sidecars.insert(sidecar);
                        }
                    }

                    if search_path.is_dir() {
                        for item in WalkDir::new(search_path).into_iter().filter_map(|a| a.ok()) {
                            if !item.path().is_file() {
                                continue;
                            }
                            logging::info_log(format!("Found item: {}", item.path().display()));
                            files.insert(item.path().to_path_buf(), find_sidecar(item.path()));
                            for sidecar in find_sidecar(item.path()) {
                                sidecars.insert(sidecar);
                            }
                        }
                    }
                    for sidecar in sidecars.iter() {
                        files.remove(sidecar);
                    }

                    logging::info_log("Starting to process files");

                    // Removes any sidecar files from files
                    files.par_iter().for_each(|(file, sidecars)| {
                        let file_id = parse_file(file, sidecars, data.clone(), globalload.clone());
                        match directory.file_action {
                            // Don't need to do anything as the default is to copy
                            sharedtypes::FileAction::Copy => {}
                            // Will remove source as we've already added it into the db
                            sharedtypes::FileAction::Move => {
                                std::fs::remove_file(file).unwrap();
                                for sidecar in sidecars {
                                    std::fs::remove_file(sidecar).unwrap();
                                }
                            }
                            // Will hardlink the file
                            sharedtypes::FileAction::HardLink => {
                                if let Some(fid) = file_id {
                                    let db = data.read().unwrap();
                                    let location = db.get_file(&fid);
                                    if let Some(dbfile_location) = location {
                                        std::fs::remove_file(file).unwrap();
                                        std::fs::hard_link(dbfile_location, file).unwrap();
                                    }
                                }
                            }
                        }
                    });
                }
            }

            cli_structs::TasksStruct::Scraper(action) => match action {
                cli_structs::ScraperAction::Test(inp) => {
                    dbg!(&inp);
                }
            },
            cli_structs::TasksStruct::Reimport(reimp) => match reimp {
                cli_structs::Reimport::DirectoryLocation(loc) => {
                    //let data = data.read().unwrap();
                    if !Path::new(&loc.location).exists() {
                        println!("Couldn't find location: {}", &loc.location);
                    }
                    /* // Loads the scraper info for parsing.
                    let scraperlibrary = scraper.read().unwrap().filter_sites_return_lib(&loc.site);
                    let libload = match scraperlibrary {
                        None => {
                            println!("Cannot find a loaded scraper. {}", &loc.site);
                            return;
                        }
                        Some(load) => load.clone(),
                    };
                    data.load_table(&sharedtypes::LoadDBTable::All);
                    let failedtoparse: HashSet<String> = HashSet::new();
                    let file_regen = crate::globalload::scraper_file_regen(libload);
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
                        let tag = crate::globalload::scraper_file_return(libload, &scraperinput);

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
                    }*/
                }
            },
            cli_structs::TasksStruct::Database(db) => {
                use crate::helpers;

                let dbstore = data.clone();
                match db {
                    cli_structs::Database::BackupDB => {
                        // backs up the db. check the location in setting or code if I change anything lol
                        let mut data = data.write().unwrap();
                        data.backup_db();
                    }
                    cli_structs::Database::CheckFiles(action) => {
                        let mut data = data.write().unwrap();

                        data.check_db_paths();

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
                        logging::info_log(
                            "Checking if we have any missing or bad files.".to_string(),
                        );
                        let mut nsid: Option<usize> = None;
                        {
                            let nso = data.namespace_get(&"source_url".to_owned());
                            if let Some(ns) = nso {
                                nsid = Some(ns);
                            }
                        }

                        // Spawn default ratelimiter of 1 item per second
                        let ratelimiter_obj = Arc::new(Mutex::new(download::ratelimiter_create(
                            &0,
                            &0,
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
                            let client = &mut download::client_create(vec![], false);
                            if !Path::new(&lispa).exists() {
                                logging::main(&format!("Cannot find hash: {}", &file.hash));
                                match action {
                                    cli_structs::CheckFilesEnum::Redownload => {}
                                    cli_structs::CheckFilesEnum::Print => {
                                        return;
                                    }
                                }
                                if let Some(nsid) = nsid {
                                    let rel = data.relationship_get_tagid(fid);
                                    for eachs in rel.iter() {
                                        let dat = data.tag_id_get(eachs).unwrap();
                                        logging::info_log(format!(
                                            "Got Tag: {} for fileid: {}",
                                            dat.name, fid
                                        ));
                                        if dat.namespace == nsid {
                                            let mut file = sharedtypes::FileObject {
                                                source: Some(sharedtypes::FileSource::Url(
                                                    dat.name.clone(),
                                                )),
                                                hash: sharedtypes::HashesSupported::Sha512(
                                                    file.hash.clone(),
                                                ),
                                                tag_list: Vec::new(),
                                                skip_if: Vec::new(),
                                            };
                                            download::dlfile_new(
                                                client,
                                                dbstore.clone(),
                                                &mut file,
                                                None,
                                                &ratelimiter_obj,
                                                &dat.name.clone(),
                                                &0,
                                                &0,
                                                None,
                                            );
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
                                    logging::error_log(format!(
                                        "BAD HASH: ID: {}  HASH: {}   2ND HASH: {}",
                                        &file.id, &file.hash, hinfo.0
                                    ));
                                    match action {
                                        cli_structs::CheckFilesEnum::Redownload => {}
                                        cli_structs::CheckFilesEnum::Print => {
                                            return;
                                        }
                                    }

                                    if nsid.is_some() {
                                        let rel = data.relationship_get_tagid(fid);
                                        for eachs in rel.iter() {
                                            let dat = data.tag_id_get(eachs).unwrap();
                                            logging::info_log(format!(
                                                "Got Tag: {} for fileid: {}",
                                                dat.name, fid
                                            ));
                                            if dat.namespace == nsid.unwrap() {
                                                let mut file = sharedtypes::FileObject {
                                                    source: Some(sharedtypes::FileSource::Url(
                                                        dat.name.clone(),
                                                    )),
                                                    hash: sharedtypes::HashesSupported::Sha512(
                                                        file.hash.clone(),
                                                    ),
                                                    tag_list: Vec::new(),
                                                    skip_if: Vec::new(),
                                                };
                                                download::dlfile_new(
                                                    client,
                                                    dbstore.clone(),
                                                    &mut file,
                                                    None,
                                                    &ratelimiter_obj,
                                                    &dat.name.clone(),
                                                    &0,
                                                    &0,
                                                    None,
                                                );
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
                        let mut data = data.write().unwrap();
                        data.load_table(&sharedtypes::LoadDBTable::Tags);
                        pause();
                    }
                    cli_structs::Database::CompressDatabase => {
                        let mut data = data.write().unwrap();
                        data.condense_db_all();
                    }
                    cli_structs::Database::RemoveWhereNot(db_n_rmv) => {
                        let mut data = data.write().unwrap();
                        let ns_id = match db_n_rmv {
                            cli_structs::NamespaceInfo::NamespaceString(ns) => {
                                data.load_table(&sharedtypes::LoadDBTable::Namespace);

                                match data.namespace_get(&ns.namespace_string) {
                                    None => {
                                        logging::info_log(format!(
                                            "Cannot find the tasks remove string in namespace {}",
                                            &ns.namespace_string
                                        ));
                                        return;
                                    }
                                    Some(id) => id,
                                }
                            }
                            cli_structs::NamespaceInfo::NamespaceId(ns) => ns.namespace_id,
                        };
                        logging::info_log(format!(
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
                        //data.drop_recreate_ns(&ns_id);
                        panic!();
                    }
                    // Removing db namespace. Will get id to remove then remove it.
                    cli_structs::Database::Remove(db_rmv) => {
                        let mut data = data.write().unwrap();
                        data.load_table(&sharedtypes::LoadDBTable::Namespace);
                        let ns_id = match db_rmv {
                            cli_structs::NamespaceInfo::NamespaceString(ns) => {
                                data.load_table(&sharedtypes::LoadDBTable::Namespace);

                                match data.namespace_get(&ns.namespace_string) {
                                    None => {
                                        logging::info_log(format!(
                                            "Cannot find the tasks remove string in namespace {}",
                                            &ns.namespace_string
                                        ));
                                        return;
                                    }
                                    Some(id) => id,
                                }
                            }
                            cli_structs::NamespaceInfo::NamespaceId(ns) => ns.namespace_id,
                        };
                        logging::info_log(format!("Found Namespace: {} Removing...", &ns_id));
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
