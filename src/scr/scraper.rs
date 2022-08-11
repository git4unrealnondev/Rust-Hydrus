use libloading;
use log::{error, info, warn};
use std::env;
use std::fs;
use std::mem;
use std::path::Path;

static SUPPORTED_VERS: f32 = 0.001;

pub struct InternalScraper {
    _version: f32,
    _name: String,
    _sites: Vec<String>,
}

///
/// Internal Scraping Reference for scrapers.
/// Copy pasta this code is and you should have a pretty good idea on what to do.
///
#[allow(dead_code)]
impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0.001,
            _name: "test.to_string".to_string(),
            _sites: crate::vec_of_strings!("example", "example1"),
        }
    }
    pub fn version_get(&self) -> f32 {
        return self._version;
    }
    pub fn version_set(&mut self, inp: f32) {
        self._version = inp;
    }
    pub fn name_get(&self) -> &String {
        &self._name
    }
    pub fn name_put(&mut self, inp: String) {
        self._name = inp;
    }
    pub fn sites_get(&self) -> Vec<String> {
        let mut vecs: Vec<String> = Vec::new();
        for each in &self._sites {
            vecs.push(each.to_string());
        }
        return vecs;
    }
}

pub struct ScraperManager {
    _string: Vec<String>,
    _sites: Vec<Vec<String>>,
    _loaded: Vec<bool>,
    _library: Vec<libloading::Library>,
}
impl ScraperManager {
    pub fn new() -> Self {
        ScraperManager {
            _string: Vec::new(),
            _sites: Vec::new(),
            _loaded: Vec::new(),
            _library: Vec::new(),
        }
    }

    pub fn sites_get(&self) -> &Vec<Vec<String>> {
        &self._sites
    }

    pub fn load(&mut self, scraperfolder: String, libpath: String, libext: String) {
        let scrapersdir: String = "./scrapers".to_string();
        let path = Path::new(&scraperfolder);
        let mut paths: Vec<String> = Vec::new();

        if !path.exists() {
            fs::create_dir_all(&scraperfolder);
        }

        let dirs = fs::read_dir(&scraperfolder).unwrap();

        for entry in dirs {
            let root: String = entry.as_ref().unwrap().path().display().to_string();
            let name = root.split("/");
            let vec: Vec<&str> = name.collect();
            let path = format!("{}{}lib{}.{}", &root, &libpath, vec[vec.len() - 1], &libext);
            if Path::new(&path).exists() {
                self._string.push(path.to_string());
                unsafe {
                    self._library
                        .push(libloading::Library::new(path.to_string()).unwrap());
                }
            } else {
                let err = format!("Loading scraper could not find {}", &path);
                warn!("{}", err);
            }
        }
        let mut cnt = 0;
        for each in &mut self._library {
            //let mut internal: Vec<libloading::Symbol<unsafe extern  fn() -> InternalScraper>> = Vec::new();
            //let mut internal = Vec::new();
            let funtwo: Result<
                libloading::Symbol<unsafe extern "C" fn() -> InternalScraper>,
                libloading::Error,
            > = unsafe { each.get(b"new\0") };

            // Loads version in memsafe way from scraper
            let scraper = unsafe { &funtwo.as_ref().unwrap()() };
            let version: f32 = scraper.version_get();

            if version < SUPPORTED_VERS {
                let msg = format!(
                    "PLUGIN LOAD: Loaded Version:{} Currently Supports:{}",
                    version, SUPPORTED_VERS
                );
                error!("{}", msg);
                //unsafe {mem::forget(funtwo.unwrap()());}
                panic!("{}", msg);
            }

            let sites: Vec<String> = scraper.sites_get();

            self._sites.push(sites);

            //unsafe{println!("{:?}", internal[0]().version_get());}
            cnt += 1;
        }
    }
}
