use crate::scr::database;
use crate::scr::download;
use crate::scr::file;
use crate::scr::scraper;
use crate::scr::time;
use ahash::AHashMap;
use log::info;
use std::time::Duration;

use super::sharedtypes::CommitType;

pub struct Jobs {
    _jobid: Vec<u128>,
    _secs: usize,
    _sites: Vec<String>,
    _params: Vec<Vec<String>>,
    //References jobid in _inmemdb hashmap :D
    _jobstorun: Vec<usize>,
    _jobref: AHashMap<usize, JobsRef>,
    scrapermanager: scraper::ScraperManager,
}
#[derive(Debug, Clone)]
pub struct JobsRef {
    pub _idindb: usize,       // What is my ID in the inmemdb
    pub _sites: String,       // Site that the user is querying
    pub _params: Vec<String>, // Anything that the user passes into the db.
    pub _jobsref: usize,      // reference time to when to run the job
    pub _jobstime: usize,     // reference time to when job is added
    pub _committype: CommitType,
    //pub _scraper: scraper::ScraperManager // Reference to the scraper that will be used
}

///
/// Jobs manager creates & manages jobs
///
impl Jobs {
    pub fn new(newmanager: scraper::ScraperManager) -> Self {
        Jobs {
            _jobid: Vec::new(),
            _sites: Vec::new(),
            _params: Vec::new(),
            _secs: 0,
            _jobstorun: Vec::new(),
            _jobref: AHashMap::new(),
            scrapermanager: newmanager,
        }
    }

    ///
    /// Loads jobs to run into _jobstorun
    ///
    pub fn jobs_get(&mut self, db: &database::Main) {
        self._secs = time::time_secs();
        let ttl = db.jobs_get_max();
        let hashjobs = db.jobs_get_all();
        let beans = self.scrapermanager.sites_get();
        dbg!(&beans);
        for each in hashjobs {
            if time::time_secs() >= each.1._jobsref + each.1._jobstime {
                for eacha in beans {
                    if eacha.contains(&each.1._sites) {
                        self._jobref.insert(*each.0, each.1.clone());
                    }
                }
            }
        }
        dbg!(&self._jobref);
        let msg = format!(
            "Loaded {} jobs out of {} jobs. Didn't load {} Jobs due to lack of scrapers or timing.",
            &self._jobref.len(),
            db.jobs_get_max(),
             db.jobs_get_max() - &self._jobref.len(),
        );
        info!("{}", msg);
        println!("{}", msg);
    }

    ///
    /// Runs jobs as they are needed to.
    ///

    pub fn jobs_run(&mut self, db: &mut database::Main) {
        // Sets up and checks scrapers

        let loaded_params: AHashMap<u128, Vec<String>> = AHashMap::new();
        let mut loaded_params: AHashMap<u128, Vec<String>> = AHashMap::new();
        let mut ratelimit: AHashMap<u128, (u64, Duration)> = AHashMap::new();

        // Handles any thing if theirs nothing to load.
        dbg!(&self._params);
        if self.scrapermanager.scraper_get().is_empty() || self._params.is_empty() {
            println!("No jobs to run...");
            return;
        }

        for each in 0..self.scrapermanager.scraper_get().len() {
            let name = self.scrapermanager.scraper_get()[each].name_get();

            dbg!(&format!("manual_{}", name));

            let name_result = db.settings_get_name(&format!("manual_{}", name));
            let each_u128: u128 = each.try_into().unwrap();
            let mut to_load = Vec::new();
            match name_result {
                Ok(_) => {
                    println!("Dont have to add manual to db.");

                    let rlimit = self.scrapermanager.scraper_get()[each].ratelimit_get();
                    to_load.push(self._params[each][0].to_string());
                    to_load.push(name_result.unwrap().1.to_string());

                    loaded_params.insert(each_u128, to_load);
                    ratelimit.insert(each_u128, rlimit);
                }
                Err("None") => {
                    let rlimit = self.scrapermanager.scraper_get()[each].ratelimit_get();
                    let (cookie, cookie_name) = self.library_cookie_needed(
                        self._jobstorun[each].into(),
                        self._params[each][0].to_string(),
                    );
                    db.setting_add(
                        format!(
                            "manual_{}",
                            self.scrapermanager.scraper_get()[each].name_get()
                        ),
                        "Manually controlled scraper.".to_string(),
                        0,
                        cookie_name.to_string(),
                        true,
                    );
                    to_load.push(self._params[each][0].to_string());
                    loaded_params.insert(each_u128, to_load);
                    ratelimit.insert(each_u128, rlimit);
                }
                Err(&_) => continue,
            };
        }

        // setup for scraping jobs will probably outsource this to another file :D.
        for each in 0..self._jobstorun.len() {
            let each_u128: u128 = each.try_into().unwrap();
            println!(
                "Running Job: {} {} {:?}",
                self._jobstorun[each], self._sites[each], self._params[each]
            );

            let parzd: Vec<&str> = self._params[each][0].split(' ').collect::<Vec<&str>>();
            let mut parsed: Vec<String> = Vec::new();
            for a in parzd {
                parsed.push(a.to_string());
            }

            let index: usize = self._jobstorun[each].into();

            // url is the output from the designated scraper that has the correct

            let bools: Vec<bool> = Vec::new();

            let url: Vec<String> =
                self.library_url_dump(self._jobstorun[each].into(), &loaded_params[&each_u128]);

            let boo = self.library_download_get(self._jobstorun[each].into());
            //let mut ratelimiter = block_on(download::ratelimiter_create(ratelimit[&each_u128]));
            if boo {
                break;
            }
            let beans =
                download::dltext(url, &mut self.scrapermanager, self._jobstorun[each].into());
            println!("Downloading Site: {}", &each);
            // parses db input and adds tags to db.
            let (url_vec, urln_vec) = db.parse_input(&beans);
            let urls_to_remove: Vec<String> = Vec::new();

            // Filters out already downloaded files.
            let namespace_id = db.namespace_get(&"parsed_url".to_string()).0;
            let mut cnt = 0;

            let location = db.settings_get_name(&"FilesLoc".to_string()).unwrap().1;
            file::folder_make(&(location).to_string());

            // Total files that are already downloaded.
            // Re-adds tags & relationships into DB Only enable if their are changes to scrapers.
            dbg!(&loaded_params[&each_u128]);
            if self._params[each][1] == "true" {
                for urls in urln_vec.keys() {
                    dbg!(format!("Checking url for tags: {}", &urls));

                    let url_id = db.tag_get_name(urls.to_string(), namespace_id).0;
                    let fileids = db.relationship_get_fileid(&url_id);
                    for fids in &fileids {
                        for tags in &urln_vec[urls] {
                            db.tag_add(tags.0.to_string(), "".to_string(), tags.1, true);
                            let tagid = db.tag_get_name(tags.0.to_string(), tags.1).0;
                            db.relationship_add(*fids, tagid, true);
                        }
                    }
                }
            }

            let utl_total = url_vec.len();

            dbg!(format!("Total Files pulled: {}", &url_vec.len()));
            for urls in url_vec.keys() {
                let map = download::file_download(urls, &location);
                println!("Downloading file# : {} / {}", &cnt, &utl_total);

                // Populates the db with files.
                for every in map.0.keys() {
                    db.file_add(
                        0,
                        map.0[every].to_string(),
                        map.1.to_string(),
                        location.to_string(),
                        true,
                    );
                    cnt += 1;
                }

                // Populates the db with relations.
                let hash = db.file_get_hash(&map.0[&urls.to_string()]).0;
                let url_namespace = db.namespace_get(&"parsed_url".to_string()).0;
                db.tag_add(urls.to_string(), "".to_string(), url_namespace, true);
                let urlid = db.tag_get_name(urls.to_string(), url_namespace).0;
                db.relationship_add(hash, urlid, true);
                for tags in &url_vec[urls] {
                    db.tag_add(tags.0.to_string(), "".to_string(), tags.1, true);
                    let tagid = db.tag_get_name(tags.0.to_string(), tags.1).0;
                    db.relationship_add(hash, tagid, true);
                }
            }
        }
    }

    /// ALL of the lower functions are just wrappers for the scraper library.
    /// This is nice because their's no unsafe code anywhere else inside code base.

    ///
    /// Returns a url to grab for.
    ///
    pub fn library_url_get(&mut self, memid: usize, params: &[String]) -> Vec<String> {
        self.scrapermanager.url_load(memid, params.to_vec())
    }

    ///
    /// Parses stuff from dltext.
    ///
    pub fn library_parser_call(
        &mut self,
        memid: usize,
        params: &String,
    ) -> Result<AHashMap<String, AHashMap<String, Vec<String>>>, &'static str> {
        self.scrapermanager.parser_call(memid, params)
    }

    ///
    /// Returns a url to grab for.
    ///
    pub fn library_url_dump(&mut self, memid: usize, params: &[String]) -> Vec<String> {
        self.scrapermanager.url_dump(memid, params.to_vec())
    }
    ///
    /// pub fn cookie_needed(&mut self, id: usize, params: String) -> (bool, String)
    ///
    pub fn library_cookie_needed(&self, memid: usize, params: String) -> (String, String) {
        self.scrapermanager.cookie_needed(memid, params)
    }

    ///
    /// Tells system if scraper should handle downloads.
    ///
    pub fn library_download_get(&self, memid: usize) -> bool {
        self.scrapermanager.scraper_download_get(memid)
    }
}
