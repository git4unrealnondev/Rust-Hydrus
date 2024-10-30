use crate::database;
use crate::download;
use crate::scraper::ScraperManager;
use crate::sharedtypes::ScraperData;
use std::collections::BTreeMap;

use crate::logging;
//use crate::jobs::JobsRef;
use crate::logging::info_log;
use crate::plugins::PluginManager;
use crate::scraper;

use crate::sharedtypes;
use crate::sharedtypes::JobScraper;
use crate::sharedtypes::ScraperReturn;

use async_std::task;

//use log::{error, info};
use ratelimit::Ratelimiter;
use reqwest::blocking::Client;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
//use std::sync::Mutex;
use rusty_pool::Builder;
use rusty_pool::ThreadPool;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use thread_control::*;

pub struct Threads {
    _workers: usize,
    worker: HashMap<usize, Worker>,
    worker_control: HashMap<usize, Flag>,
    scraper_storage: HashMap<scraper::InternalScraper, usize>,
}

///
/// Holder for workers.
/// Workers manage their own threads.
///
impl Threads {
    pub fn new() -> Self {
        let workers = 0;

        Threads {
            _workers: workers,
            worker: HashMap::new(),
            worker_control: HashMap::new(),
            scraper_storage: HashMap::new(),
        }
    }

    ///
    /// Adds a worker to the threadvec.
    ///
    pub fn startwork(
        &mut self,
        jobstorage: Arc<Mutex<crate::jobs::Jobs>>,
        db: &mut Arc<Mutex<database::Main>>,
        scrapermanager: scraper::InternalScraper,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
        arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    ) {
        if !self.scraper_storage.contains_key(&scrapermanager) {
            let (flag, control) = make_pair();
            self.scraper_storage
                .insert(scrapermanager.clone(), self._workers);
            let worker = Worker::new(
                self._workers,
                jobstorage,
                db,
                scrapermanager,
                pluginmanager,
                control,
                arc_scrapermanager,
            );
            self.worker_control.insert(self._workers, flag);
            self.worker.insert(self._workers, worker);
            self._workers += 1;
        }

        //self._workers.push(worker);
    }

    ///
    /// Checks and clears the worker pools & long stored data
    ///
    pub fn check_threads(&mut self) {
        let mut temp = Vec::new();

        // Checks if a thread is processing data currently
        for (id, threadflag) in &self.worker_control {
            if !threadflag.alive() {
                logging::info_log(&format!("Removing Worker Thread {}", id));
                temp.push(id.clone())
            }
        }

        //Removing the data from thread handler
        for id in temp {
            self.worker_control.remove(&id);
            self.worker.remove(&id);
            for (scraper, idscraper) in self.scraper_storage.clone() {
                if idscraper == id {
                    self.scraper_storage.remove(&scraper);
                }
            }
        }

        //Reset counter
        if self.worker.is_empty() {
            self._workers = 0;
        }
    }
}
///
/// Worker holder for data. Will add a scraper processor soon tm.
///
struct Worker {
    id: usize,
    thread: Option<std::thread::JoinHandle<()>>,
}

///
/// closes the thread that the worker contains.
/// Used in easy thread handeling
/// Only reason to do this over doing this with default drop behaviour is the logging.
///
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            futures::executor::block_on(async { thread.join().unwrap() });
            info_log(&format!("Shutting Down Worker from Worker: {}", self.id));
            println!("Shutting Down Worker from Worker: {}", self.id);
        }
    }
}

impl Worker {
    fn new(
        id: usize,
        jobstorage: Arc<Mutex<crate::jobs::Jobs>>,
        dba: &mut Arc<Mutex<database::Main>>,
        scraper: scraper::InternalScraper,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
        threadflagcontrol: Control,
        arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    ) -> Worker {
        /*info_log(&format!(
            "Creating Worker for id: {} Scraper Name: {} With a jobs length of: {}",
            &id,
            &scraper._name,
            &jobstorage..len()
        ));*/
        let mut db = dba.clone();

        let manageeplugin = pluginmanager.clone();

        let ratelimiter_main = Arc::new(Mutex::new(download::ratelimiter_create(
            scraper._ratelimit.0,
            scraper._ratelimit.1,
        )));
        let thread = thread::spawn(move || {
            let ratelimiter_obj = ratelimiter_main.clone();
            let mut job_params: Arc<Mutex<BTreeSet<ScraperData>>> =
                Arc::new(Mutex::new(BTreeSet::new()));
            let mut job_ref_hash: BTreeMap<ScraperData, sharedtypes::DbJobsObj> = BTreeMap::new();
            let mut rate_limit_vec: Vec<Ratelimiter> = Vec::new();
            let mut rate_limit_key: HashMap<String, usize> = HashMap::new();

            let mut rate_limit_store: Arc<Mutex<HashMap<String, Ratelimiter>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let mut client = download::client_create();

            'bigloop: loop {
                let jobsstorage;
                {
                    let temp = jobstorage.lock().unwrap();
                    jobsstorage = temp.jobs_get(&scraper).clone();
                }
                for job in jobsstorage {
                    let currentjob = job.clone();
                    let mut par_vec: Vec<sharedtypes::ScraperParam> = Vec::new();
                    {
                        let parpms: Vec<String> = job
                            .param
                            .as_ref()
                            .unwrap()
                            .split_whitespace()
                            .map(str::to_string)
                            .collect();
                        for par in parpms {
                            let temp = sharedtypes::ScraperParam {
                                param_data: par,
                                param_type: sharedtypes::ScraperParamType::Normal,
                            };
                            par_vec.push(temp)
                        }

                        let unwrappydb = &mut db.lock().unwrap();
                        let datafromdb = unwrappydb
                            .settings_get_name(&format!("{}_{}", scraper._type, &scraper._name));
                        match datafromdb {
                            None => {}
                            Some(setting) => {
                                match &setting.param {
                                    None => {}
                                    Some(param) => {
                                        // Adds database tag if applicable.
                                        let scrap_data = sharedtypes::ScraperParam {
                                            param_data: param.clone(),
                                            param_type: sharedtypes::ScraperParamType::Database,
                                        };

                                        par_vec.push(scrap_data);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(recursion) = &job.jobmanager.recreation {
                        match recursion {
                            sharedtypes::DbJobRecreation::AlwaysTime((timestamp, count)) => {
                                let mut temp = jobstorage.lock().unwrap();
                                let mut data = job.clone();
                                data.time = Some(crate::time_func::time_secs());
                                data.reptime = Some(*timestamp);
                                temp.jobs_decrement_count(&data, &scraper);
                            }
                            _ => {}
                        }
                    }

                    // Legacy data holder for plugin system
                    let job_holder_legacy = sharedtypes::JobScraper {
                        site: job.site.clone(),
                        param: par_vec.clone(),
                        original_param: job.param.clone().unwrap(),
                        job_type: job.jobmanager.jobtype,
                    };
                    // Legacy data holder obj for plguins
                    let scraper_data_holder = sharedtypes::ScraperData {
                        job: job_holder_legacy.clone(),
                        system_data: job.system_data,
                        user_data: job.user_data,
                    };

                    let urlload = match job.jobmanager.jobtype {
                        sharedtypes::DbJobType::Params => {
                            let temp = scraper::url_dump(
                                &par_vec,
                                &scraper_data_holder,
                                arc_scrapermanager.clone(),
                                &scraper,
                            );
                            //job = temp.1;
                            temp
                        }
                        sharedtypes::DbJobType::Plugin => {
                            continue;
                        }
                        /*sharedtypes::DbJobType::FileUrl => {
                            let parpms: Vec<(String, ScraperData)> = (
                                job.param
                                    .clone()
                                    .unwrap()
                                    .split_whitespace()
                                    .map(str::to_string)
                                    .collect(),
                                scraper_data_holder,
                            );
                            parpms
                        }*/
                        sharedtypes::DbJobType::Scraper => {
                            vec![(job.param.clone().unwrap(), scraper_data_holder)]
                        }
                    };
                    let scraper_library;
                    {
                        let scrapermanager = arc_scrapermanager.lock().unwrap();
                        scraper_library =
                            scrapermanager.library_get().get(&scraper).unwrap().clone();
                    }

                    'urlloop: for (urll, scraperdata) in urlload {
                        'errloop: loop {
                            let resp = task::block_on(download::dltext_new(
                                urll.to_string(),
                                &mut client,
                                &ratelimiter_obj,
                            ));

                            let st = match resp {
                                Ok(respstring) => scraper::parser_call(
                                    &respstring,
                                    &scraperdata,
                                    arc_scrapermanager.clone(),
                                    &scraper,
                                ),
                                Err(_) => continue,
                            };

                            let (out_st, scraper_data_parser) = match st {
                                Ok(objectscraper) => objectscraper,
                                Err(ScraperReturn::Nothing) => {
                                    //job_params.lock().unwrap().remove(&scraper_data);
                                    dbg!("Exiting loop due to nothing.");
                                    break 'urlloop;
                                }
                                Err(ScraperReturn::EMCStop(emc)) => {
                                    panic!("EMC STOP DUE TO: {}", emc);
                                }
                                Err(ScraperReturn::Stop(stop)) => {
                                    //let temp = scraper_data.clone().job;
                                    //job_params.lock().unwrap().remove(&scraper_data);

                                    logging::error_log(&format!("Stopping job: {:?}", stop));
                                    continue;
                                }
                                Err(ScraperReturn::Timeout(time)) => {
                                    let time_dur = Duration::from_secs(time);
                                    thread::sleep(time_dur);
                                    continue;
                                }
                            };

                            //scraper_data = scraper_data_parser;
                            //Parses tags from urls
                            for tag in out_st.tag {
                                let to_parse = parse_tags(&db, tag, None);
                                let mut joblock;
                                joblock = jobstorage.lock().unwrap();
                                for urlz in to_parse {
                                    dbg!(format!("Adding job: {:?}", &urlz));
                                    joblock.jobs_add(&scraper, urlz, true, true);
                                }
                                //let url_job = JobScraper {};
                                //dbg!(&urlz);
                                //job_params.lock().unwrap().insert(urlz);

                                //      job_ref_hash.insert(urlz, job);

                                //for each in to_parse {
                                //    job_params.lock().unwrap().insert(each);
                                //}
                            }

                            let source_url_id = {
                                let unwrappydb = &mut db.lock().unwrap();
                                match unwrappydb.namespace_get(&"source_url".to_string()).cloned() {
                                    None => unwrappydb
                                        .namespace_add(
                                            "source_url".to_string(),
                                            Some("Source URL for a file.".to_string()),
                                            true,
                                        )
                                        .clone(),
                                    Some(id) => id,
                                }
                                // defaults to 0 due to unknown.
                            };

                            let pool = ThreadPool::default();
                            // Parses files from urls
                            for file in out_st.file {
                                let ratelimiter_obj = ratelimiter_main.clone();
                                let manageeplugin = manageeplugin.clone();
                                let mut db = db.clone();
                                let client = client.clone();
                                let jobstorage = jobstorage.clone();
                                let scraper = scraper.clone();
                                let location = {
                                    let unwrappydb = &mut db.lock().unwrap();
                                    unwrappydb.location_get()
                                };

                                pool.execute(move || {
                                                if let Some(ref source) = file.source_url {
                                                // If url exists in db then don't download
                                                //thread::sleep(Duration::from_secs(10));
                                                let url_tag;
                                                {
                                                    let unwrappydb = db.lock().unwrap();
                                                    url_tag = unwrappydb
                                                        .tag_get_name(source.clone(), source_url_id)
                                                        .cloned();
                                                };
                                                for file_tag in file.skip_if.iter() {
                                                    match file_tag {
                                                sharedtypes::SkipIf::FileNamespaceNumber((
                            unique_tag,
                            namespace_filter,
                            filter_number,
                        )) => {
let unwrappydb = db.lock().unwrap();
                            let mut cnt = 0;

                            if let Some(nidf) = unwrappydb.namespace_get(&namespace_filter.name) {
                            if let Some(nid) = unwrappydb.namespace_get(&unique_tag.namespace.name) {
                                if let Some(tid) =
                                    unwrappydb.tag_get_name(unique_tag.tag.clone(), *nid)
                                {
                                    if let Some(fids) = unwrappydb.relationship_get_fileid(tid) {
                                        if fids.len() == 1 {
                                            let fid = fids.iter().next().unwrap();
                                            for tidtofilter in
                                                unwrappydb.relationship_get_tagid(fid).unwrap().iter()
                                            {
                                                if unwrappydb.namespace_contains_id(nidf, tidtofilter)
                                                {
                                                    cnt += 1;
                                                }
                                            }}
                                        }
                                    }
                                }
                            }
                            if cnt > *filter_number {
                                info_log(&format!(
                                            "Not downloading because unique namespace is greater then limit number. {}",
                                            unique_tag.tag
                                        ));
                                                            return;
                            } else {
                                info_log(&format!("Downloading due to unique namespace not existing or number less then limit number."));
                            }
                        }

                                                        sharedtypes::SkipIf::FileTagRelationship(tag) => {
let unwrappydb = db.lock().unwrap();
                                                    if let Some(nsid) = unwrappydb.namespace_get(&tag.namespace.name){
            if let Some(_)=unwrappydb.tag_get_name(tag.tag.to_string(), *nsid){info_log(&format!("Skipping file: {} Due to skip tag {} already existing in Tags Table.",&source, tag.tag));
            return;
            }}

                                                    }
                                                }
                                                                                            }

                                                // Get's the hash & file ext for the file.
                                                 let fileid = match url_tag {
                                                    None => {
                                                        match download_add_to_db( ratelimiter_obj, source, location, manageeplugin, &client, db.clone(), &file, source_url_id) {
                                                            None => return,
                                                            Some(id) =>id,
            }
                                                    }
                                                    Some(url_id) => {
                                                        let file_id;
                                                        {
                                                            // We've already got a valid relationship
                                                            let unwrappydb = &mut db.lock().unwrap();
                                                            file_id = unwrappydb
                                                                .relationship_get_one_fileid(&url_id)
                                                                .copied();
                                                            if let Some(fid) = file_id {
                                                                unwrappydb.file_get_id(&fid).unwrap();
                                                            }
                                                        }
                                                        // fixes busted links.
                                                        if let Some(file_id) = file_id {
                                                            info_log(&format!(
                                                            "Skipping file: {} Due to already existing in Tags Table.",
                                                            &source
                                                        ));

                                                            file_id
                                                        } else {match download_add_to_db( ratelimiter_obj, source, location, manageeplugin, &client, db.clone(), &file, source_url_id) {
                                                            None => return,
                                                            Some(id) =>id,
            }

                                                        }
                                                    }
                                                };

                                                // We've got valid fileid for reference.

                                                for taz in file.tag_list {
                                                    //dbg!(&taz);
                                                    let tag = taz;

                                                    let urls_scrap = parse_tags(&db, tag, Some(fileid));

                                            {

                                            let mut joblock;
joblock = jobstorage.lock().unwrap();for urlz in urls_scrap {
                                                joblock.jobs_add(&scraper, urlz, true, true);
                                                        //let url_job = JobScraper {};
                                                        //dbg!(&urlz);
                                                        //job_params.lock().unwrap().insert(urlz);


                                                        //      job_ref_hash.insert(urlz, job);
                                                    }

                                            }

                                                                                                    }
                                                }
                                            });
                                // End of err catching loop.
                                // break 'errloop;
                            }
                            pool.join();
                            break 'errloop;
                        }

                        /*{
                            let mut joblock = jobstorage.lock().unwrap();
                            joblock.jobs_remove_dbjob(&scraper, &currentjob);

                            let mut db = db.lock().unwrap();
                            db.transaction_flush();
                        }*/

                        //let unwrappydb = &mut db.lock().unwrap();
                        //unwrappydb.del_from_jobs_byid(&job.id);
                    }
                    {
                        let mut joblock = jobstorage.lock().unwrap();
                        joblock.jobs_remove_dbjob(&scraper, &currentjob);

                        let mut db = db.lock().unwrap();
                        db.transaction_flush();
                    }

                    {
                        let joblock = jobstorage.lock().unwrap();
                        if joblock.jobs_get(&scraper).is_empty() {
                            let mut db = db.lock().unwrap();
                            db.transaction_flush();
                            break 'bigloop;
                        }
                    }
                }
                let joblock = jobstorage.lock().unwrap();
                if joblock.jobs_get(&scraper).is_empty() {
                    threadflagcontrol.stop();
                    break 'bigloop;
                }
            }
            threadflagcontrol.stop();
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

///
/// Parses tags and adds the tags into the db.
///
fn parse_tags(
    db: &Arc<Mutex<database::Main>>,
    tag: sharedtypes::TagObject,
    file_id: Option<usize>,
) -> BTreeSet<sharedtypes::ScraperData> {
    let mut url_return: BTreeSet<sharedtypes::ScraperData> = BTreeSet::new();

    let unwrappy = &mut db.lock().unwrap();

    //dbg!(&tag);

    match tag.tag_type {
        sharedtypes::TagType::Normal => {
            //println!("Adding tag: {} {:?}", tag.tag, &file_id);
            // We've recieved a normal tag. Will parse.

            let namespace_id =
                unwrappy.namespace_add(tag.namespace.name, tag.namespace.description, true);
            let tag_id = unwrappy.tag_add(&tag.tag, namespace_id, true, None);
            match tag.relates_to {
                None => {
                    /*let relate_ns_id = unwrappy.namespace_add(
                        relate.namespace.name.clone(),
                        relate.namespace.description,
                        true,
                    );*/
                }
                Some(relate) => {
                    let relate_ns_id = unwrappy.namespace_add(
                        relate.namespace.name.clone(),
                        relate.namespace.description,
                        true,
                    );
                    let limit_to = match relate.limit_to {
                        None => None,
                        Some(tag) => {
                            let namespace_id = unwrappy.namespace_add(
                                tag.namespace.name,
                                tag.namespace.description,
                                true,
                            );

                            let tid = unwrappy.tag_add(&tag.tag, namespace_id, true, None);
                            Some(tid)
                        }
                    };
                    let relate_tag_id = unwrappy.tag_add(&relate.tag, relate_ns_id, true, None);
                    unwrappy.parents_add(tag_id, relate_tag_id, limit_to, true);
                }
            }

            match file_id {
                None => {}
                Some(id) => {
                    unwrappy.relationship_add(id, tag_id, true);
                }
            }
            url_return
        }
        sharedtypes::TagType::ParseUrl((jobscraped, skippy)) => {
            match skippy {
                None => {
                    url_return.insert(jobscraped);
                }
                Some(skip_if) => {
                    match skip_if {
                        sharedtypes::SkipIf::FileNamespaceNumber((
                            unique_tag,
                            namespace_filter,
                            filter_number,
                        )) => {
                            let mut cnt = 0;
                            if let Some(nidf) = unwrappy.namespace_get(&namespace_filter.name) {
                                if let Some(nid) =
                                    unwrappy.namespace_get(&unique_tag.namespace.name)
                                {
                                    if let Some(tid) =
                                        unwrappy.tag_get_name(unique_tag.tag.clone(), *nid)
                                    {
                                        if let Some(fids) = unwrappy.relationship_get_fileid(tid) {
                                            if fids.len() == 1 {
                                                let fid = fids.iter().next().unwrap();
                                                for tidtofilter in unwrappy
                                                    .relationship_get_tagid(fid)
                                                    .unwrap()
                                                    .iter()
                                                {
                                                    if unwrappy
                                                        .namespace_contains_id(nidf, tidtofilter)
                                                    {
                                                        cnt += 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if cnt >= filter_number {
                                dbg!(&cnt, &filter_number);
                                info_log(&format!(
                                            "Not downloading because unique namespace is greater then limit number. {}",
                                            unique_tag.tag
                                        ));
                            } else {
                                info_log(&format!("Downloading due to unique namespace not existing or number less then limit number."));
                                url_return.insert(jobscraped);
                            }
                        }
                        sharedtypes::SkipIf::FileTagRelationship(taginfo) => 'tag: {
                            let nid = unwrappy.namespace_get(&taginfo.namespace.name);
                            let id = match nid {
                                None => {
                                    println!("Namespace does not exist: {:?}", taginfo.namespace);
                                    url_return.insert(jobscraped);
                                    break 'tag;
                                }
                                Some(id) => id,
                            };

                            match unwrappy.tag_get_name(taginfo.tag.clone(), *id) {
                                None => {
                                    println!("WillDownload: {}", taginfo.tag);
                                    url_return.insert(jobscraped);
                                }
                                Some(tag_id) => {
                                    let rel_hashset = unwrappy.relationship_get_fileid(tag_id);
                                    match rel_hashset {
                                        None => {
                                            println!(
                                                "Downloading: {} because no relationship",
                                                taginfo.tag
                                            );
                                            println!("Will download from: {}", taginfo.tag);
                                            url_return.insert(jobscraped);
                                        }
                                        Some(_) => {
                                            info_log(&format!(
                                            "Skipping because this already has a relationship. {}",
                                            taginfo.tag
                                        ));

                                            //println!("Will download from: {}", taginfo.tag);
                                            //url_return.insert(jobscraped);
                                        }
                                    }
                                    println!("Ignoring: {}", taginfo.tag);

                                    break 'tag;
                                }
                            }
                        }
                    }
                }
            }
            // Returns the url that we need to parse.
            url_return
        }
        sharedtypes::TagType::Special => {
            // Do nothing will handle this later lol.
            url_return
        }
    }
}

fn download_add_to_db(
    ratelimiter_obj: Arc<Mutex<Ratelimiter>>,
    source: &String,
    location: String,
    manageeplugin: Arc<Mutex<PluginManager>>,
    client: &Client,
    db: Arc<Mutex<database::Main>>,
    file: &sharedtypes::FileObject,
    source_url_id: usize,
) -> Option<usize> {
    // Download file doesn't exist.

    // URL doesn't exist in DB Will download
    let blopt;
    {
        blopt = download::dlfile_new(
            &client,
            db.clone(),
            &file,
            &location,
            Some(manageeplugin),
            &ratelimiter_obj,
        );
    }
    match blopt {
        None => {
            return None;
        }
        Some((hash, file_ext)) => {
            let file = sharedtypes::DbFileStorage::NoIdExist(sharedtypes::DbFileObjNoId {
                hash,
                ext: file_ext,
                location,
            });
            let unwrappydb = &mut db.lock().unwrap();
            let fileid = unwrappydb.file_add(file, true);
            let tagid = unwrappydb.tag_add(source, source_url_id, true, None);
            unwrappydb.relationship_add(fileid, tagid, true);
            Some(fileid.clone())
        }
    }
}
