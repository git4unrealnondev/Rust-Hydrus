use libloading;
use log::{error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

static SUPPORTED_VERS: f32 = 0.001;

pub struct InternalScraper {
    _version: f32,
    _name: String,
    _sites: Vec<String>,
    _ratelimit: (u64, Duration),
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
            _ratelimit: (2, Duration::from_secs(2)),
        }
    }
    pub fn version_get(&self) -> f32 {
        self._version
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
        vecs
    }

    pub fn url_get(&self, params: Vec<String>) -> String {
        "a".to_string()
    }

    pub fn ratelimit_get(&self) -> (u64, Duration) {
        self._ratelimit
    }
}

pub struct ScraperManager {
    _string: Vec<String>,
    _sites: Vec<Vec<String>>,
    _loaded: Vec<bool>,
    _library: Vec<libloading::Library>,
    _scraper: Vec<InternalScraper>,
}
impl ScraperManager {
    pub fn new() -> Self {
        ScraperManager {
            _string: Vec::new(),
            _sites: Vec::new(),
            _loaded: Vec::new(),
            _library: Vec::new(),
            _scraper: Vec::new(),
        }
    }

    pub fn sites_get(&self) -> &Vec<Vec<String>> {
        &self._sites
    }

    pub fn scraper_get(&self) -> &Vec<InternalScraper> {
        &self._scraper
    }

    pub fn library_get(&self) -> &Vec<libloading::Library> {
        &self._library
    }

    pub fn load(&mut self, scraperfolder: String, libpath: String, libext: String) {
        let path = Path::new(&scraperfolder);

        if !path.exists() {
            fs::create_dir_all(&scraperfolder).unwrap();
        }

        let dirs = fs::read_dir(&scraperfolder).unwrap();

        for entry in dirs {
            let root: String = entry.as_ref().unwrap().path().display().to_string();
            let name = root.split('/');
            let vec: Vec<&str> = name.collect();
            let path = format!("{}{}lib{}.{}", &root, &libpath, vec[vec.len() - 1], &libext);

            info!("Loading scraper at: {}", path);

            if Path::new(&path).exists() {
                self._string.push(path.to_string());
                self._library
                    .push(unsafe { libloading::Library::new(&path).unwrap() })
            } else {
                let err = format!(
                    "Loading scraper couInternalScraper::neld not find {}",
                    &path
                );
                warn!("{}", err);
            }
        }
        for each in &mut self._library {
            //let mut internal: Vec<libloading::Symbol<unsafe extern  fn() -> InternalScraper>> = Vec::new();
            //let mut internal = Vec::new();
            let funtwo: Result<
                libloading::Symbol<unsafe extern "C" fn() -> InternalScraper>,
                libloading::Error,
            > = unsafe { each.get(b"new") };

            // Loads version in memsafe way from scraper
            let scraper = unsafe { funtwo.as_ref().unwrap()() };
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
            self._scraper.push(scraper);
            //unsafe{println!("{:?}", internal[0]().version_get());}
        }
    }
    pub fn url_load(&mut self, id: usize, params: Vec<String>) -> Vec<String> {
        let temp: libloading::Symbol<unsafe extern "C" fn(&Vec<String>) -> Vec<String>> =
            unsafe { self._library[id].get(b"url_get\0").unwrap() };
        unsafe { temp(&params) }
    }
    pub fn url_dump(&self, id: usize, params: Vec<String>) -> Vec<String> {
        let temp: libloading::Symbol<unsafe extern "C" fn(&Vec<String>) -> Vec<String>> =
            unsafe { self._library[id].get(b"url_dump\0").unwrap() };
        unsafe { temp(&params) }
    }
    pub fn parser_call(
        &self,
        id: usize,
        params: &String,
    ) -> Result<HashMap<String, HashMap<String, Vec<String>>>, &'static str> {
        let temp: libloading::Symbol<
            unsafe extern "C" fn(
                &String,
            ) -> Result<
                HashMap<String, HashMap<String, Vec<String>>>,
                &'static str,
            >,
        > = unsafe { self._library[id].get(b"parser\0").unwrap() };
        unsafe { temp(params) }
    }
    pub fn cookie_needed(&self, id: usize, params: String) -> (String, String) {
        let temp: libloading::Symbol<unsafe extern "C" fn() -> (String, String)> =
            unsafe { self._library[id].get(b"cookie_needed\0").unwrap() };
        unsafe { temp() }
    }
    ///
    /// Tells downloader to allow scraper to download.
    ///
    pub fn scraper_download_get(&self, id: usize) -> bool {
        let temp: libloading::Symbol<unsafe extern "C" fn() -> bool> =
            unsafe { self._library[id].get(b"scraper_download_get\0").unwrap() };
        unsafe { temp() }
    }
    ///
    /// Should only be called when scraper needs to download something assuming scraper_download_get returns true.
    /// TODO NOT YET IMPLEMENTED PROPERLY.
    ///
    pub fn scraper_download(&self, id: usize, params: String) -> bool {
        let temp: libloading::Symbol<unsafe extern "C" fn() -> bool> =
            unsafe { self._library[id].get(b"scraper_download\0").unwrap() };
        unsafe { temp() }
    }
}