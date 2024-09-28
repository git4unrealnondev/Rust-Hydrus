use crate::database;
use crate::download;
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
pub struct Threads {
    _workers: usize,
    worker: Vec<Worker>,
}

///
/// Holder for workers.
/// Workers manage their own threads.
///
impl Threads {
    pub fn new() -> Self {
        let workers = 0;
        let worker = Vec::new();

        Threads {
            _workers: workers,
            worker,
        }
    }

    ///
    /// Adds a worker to the threadvec.
    ///
    pub fn startwork(
        &mut self,
        scraper: scraper::InternalScraper,
        jobs: Vec<sharedtypes::DbJobsObj>,
        db: &mut Arc<Mutex<database::Main>>,
        scrapermanager: libloading::Library,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
    ) {
        let worker = Worker::new(
            self._workers,
            scraper,
            jobs,
            //&mut self._runtime,
            db,
            scrapermanager,
            pluginmanager,
        );
        self._workers += 1;
        self.worker.push(worker);

        //self._workers.push(worker);
    }
}
///
/// Worker holder for data. Will add a scraper processor soon tm.
///
struct Worker {
    id: usize,
    scraper: scraper::InternalScraper,
    jobs: Vec<sharedtypes::DbJobsObj>,
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
        scraper: scraper::InternalScraper,
        jobs: Vec<sharedtypes::DbJobsObj>,
        //rt: &mut Runtime,
        dba: &mut Arc<Mutex<database::Main>>,
        libloading: libloading::Library,
        pluginmanager: &mut Arc<Mutex<PluginManager>>,
    ) -> Worker {
        info_log(&format!(
            "Creating Worker for id: {} Scraper Name: {} With a jobs length of: {}",
            &id,
            &scraper._name,
            &jobs.len()
        ));
        let mut db = dba.clone();
        let mut jblist = jobs.clone();
        let manageeplugin = pluginmanager.clone();
        let scrap = scraper.clone();
        let ratelimiter_main = Arc::new(Mutex::new(download::ratelimiter_create(
            scrap._ratelimit.0,
            scrap._ratelimit.1,
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

            // Main loop for processing
            // All queries have been deduplicated.
            let mut job_loop = true;
            while job_loop {
                for job in jblist.clone() {
                    let mut par_vec: Vec<sharedtypes::ScraperParam> = Vec::new();
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

                    {
                        // Gets info from DB. If it exists then insert into params hashset.
                        let unwrappydb = &mut db.lock().unwrap();
                        let datafromdb = unwrappydb
                            .settings_get_name(&format!(
                                "{}_{}",
                                scrap._type,
                                scrap._name.to_owned()
                            ))
                            .unwrap()
                            .param
                            .clone();
                        match datafromdb {
                            None => {}
                            Some(param) => {
                                // Adds database tag if applicable.
                                let scrap_data = sharedtypes::ScraperParam {
                                    param_data: param,
                                    param_type: sharedtypes::ScraperParamType::Database,
                                };

                                par_vec.push(scrap_data);
                            }
                        }
                    }

                    let sc = sharedtypes::ScraperData {
                        job: JobScraper {
                            site: job.site.clone(),
                            param: par_vec,
                            original_param: job.param.clone().unwrap(),
                            job_type: job.jobmanager.jobtype,
                            //job_ref: job.clone(),
                        },
                        system_data: BTreeMap::new(),
                        user_data: BTreeMap::new(),
                    };
                    job_ref_hash.insert(sc.clone(), job);
                    job_params.lock().unwrap().insert(sc);
                }
                // dbg!(&job_params.lock().unwrap();
                let temp = job_params.lock().unwrap().clone();
                for mut scraper_data in temp {
                    let scraper_data_orig = scraper_data.clone();
                    let urlload = match scraper_data.job.job_type {
                        sharedtypes::DbJobType::Params => {
                            let temp = scraper::url_dump(
                                &libloading,
                                &scraper_data.job.param,
                                &scraper_data,
                            );
                            scraper_data = temp.1;
                            temp.0
                        }
                        sharedtypes::DbJobType::Plugin => {
                            continue;
                        }
                        sharedtypes::DbJobType::FileUrl => {
                            let parpms: Vec<String> = scraper_data
                                .job
                                .original_param
                                .split_whitespace()
                                .map(str::to_string)
                                .collect();
                            parpms
                        }
                        sharedtypes::DbJobType::Scraper => {
                            vec![scraper_data.job.original_param.clone()]
                        }
                    };
                    /*// Only instants the ratelimit if we don't already have it.
                    let rlimit = match rate_limit_key.get_mut(&job.site) {
                        None => {
                            info_log(&format!("Creating ratelimiter for site: {}", &job.site));
                            let u_temp = rate_limit_vec.len();
                            rate_limit_key.insert(job.site.clone(), u_temp);
                            rate_limit_vec.push(download::ratelimiter_create(
                                scrap._ratelimit.0,
                                scrap._ratelimit.1,
                            ));
                                                        &mut rate_limit_vec[u_temp]
                        }
                        Some(u_temp) => rate_limit_vec.get_mut(*u_temp).unwrap(),
                    };*/

                    'urlloop: for urll in urlload {
                        'errloop: loop {
                            download::ratelimiter_wait(&ratelimiter_obj);
                            let resp =
                                task::block_on(download::dltext_new(urll.to_string(), &mut client));
                            let st = match resp {
                                Ok(respstring) => {
                                    scraper::parser_call(&libloading, &respstring, &scraper_data)
                                }
                                Err(_) => continue,
                            };

                            let (out_st, scraper_data_parser) = match st {
                                Ok(objectscraper) => objectscraper,
                                Err(ScraperReturn::Nothing) => {
                                    job_params.lock().unwrap().remove(&scraper_data);
                                    dbg!("Exiting loop due to nothing.");
                                    break 'urlloop;
                                }
                                Err(ScraperReturn::EMCStop(emc)) => {
                                    panic!("EMC STOP DUE TO: {}", emc);
                                }
                                Err(ScraperReturn::Stop(stop)) => {
                                    let temp = scraper_data.clone().job;
                                    job_params.lock().unwrap().remove(&scraper_data);

                                    logging::error_log(&format!(
                                        "Stopping job: {:?} due to {}",
                                        &temp.param.clone(),
                                        stop
                                    ));
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
                                let to_parse = parse_tags(&db, tag, None, &scraper_data);
                                for each in to_parse {
                                    job_params.lock().unwrap().insert(each);
                                }
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

                            let job_site = scraper_data.job.site.clone();
                            let pool = ThreadPool::default();
                            // Parses files from urls
                            for file in out_st.file {
                                let ratelimiter_obj = ratelimiter_main.clone();
                                let manageeplugin = manageeplugin.clone();
                                let mut db = db.clone();
                                let job_params = job_params.clone();
                                let rate_limit_store = rate_limit_store.clone();
                                let job_site = job_site.clone();
                                let scraper_data = scraper_data.clone();
                                let client = client.clone();
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
                                        let unwrappydb = db.lock().unwrap();
                                        if let Some(nsid) = unwrappydb.namespace_get(&file_tag.namespace.name){
if let Some(_)=unwrappydb.tag_get_name(file_tag.tag.to_string(), *nsid){info_log(&format!("Skipping file: {} Due to skip tag {} already existing in Tags Table.",&source, file_tag.tag));
return;
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

                                        let urls_scrap = parse_tags(&db, tag, Some(fileid), &scraper_data.clone());
                                        for urlz in urls_scrap {
                                            //let url_job = JobScraper {};
                                            //dbg!(&urlz);
                                            job_params.lock().unwrap().insert(urlz);
                                            //      job_ref_hash.insert(urlz, job);
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

                        //dbg!("End of URL Loop");
                        //let unwrappydb = &mut db.lock().unwrap();
                        //unwrappydb.transaction_flush();
                    }
                    //println!("End of loop");
                    let unwrappydb = &mut db.lock().unwrap();
                    //dbg!(&job);
                    unwrappydb.del_from_jobs_table(&scraper_data.clone().job);
                    job_params.lock().unwrap().remove(&scraper_data);
                    unwrappydb.del_from_jobs_table(&scraper_data_orig.job);
                    job_params.lock().unwrap().remove(&scraper_data_orig);
                    logging::info_log(&format!("Removing job {:?}", &scraper_data));

                    // Safer way to remove jobs from list.
                    if let Some(jobscr) = job_ref_hash.get(&scraper_data) {
                        if let Some(index) = jblist.iter().position(|r| r == jobscr) {
                            jblist.remove(index);
                        }
                    }

                    if let Some(jobscr) = job_ref_hash.get(&scraper_data_orig) {
                        if let Some(index) = jblist.iter().position(|r| r == jobscr) {
                            jblist.remove(index);
                        }
                    }

                    unwrappydb.transaction_flush();
                }

                if job_params.lock().unwrap().is_empty() {
                    job_loop = false;
                } else {
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
            scraper,
            jobs,
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
    scraper_data: &sharedtypes::ScraperData,
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
                sharedtypes::SkipIf::None => {
                    url_return.insert(jobscraped);
                }
                sharedtypes::SkipIf::Tag(taginfo) => 'tag: {
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
                            if taginfo.needsrelationship {
                                let rel_hashset = unwrappy.relationship_get_fileid(tag_id);
                                match rel_hashset {
                                    None => {
                                        println!(
                                            "Downloading: {} because no relationship",
                                            taginfo.tag
                                        );
                                        println!("Will download from: {}", taginfo.tag);
                                        url_return.insert(jobscraped);
                                        break 'tag;
                                    }
                                    Some(_) => {
                                        println!(
                                            "Skipping because this already has a relationship. {}",
                                            taginfo.tag
                                        );

                                        //println!("Will download from: {}", taginfo.tag);
                                        //url_return.insert(jobscraped);
                                        break 'tag;
                                    }
                                }
                            }
                            println!("Ignoring: {}", taginfo.tag);

                            break 'tag;
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
    download::ratelimiter_wait(&ratelimiter_obj);

    // Download file doesn't exist.

    // URL doesn't exist in DB Will download
    info_log(&format!("Downloading: {} to: {}", &source, &location));
    let blopt;
    {
        blopt = download::dlfile_new(&client, &file, &location, Some(manageeplugin));
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
