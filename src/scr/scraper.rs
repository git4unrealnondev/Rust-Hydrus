use crate::{logging, sharedtypes};
use log::{error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

static SUPPORTED_VERS: usize = 0;
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct InternalScraper {
    pub _version: usize,                 //Version of the scraper
    pub _name: String,                   // Name of the scraper
    pub _sites: Vec<String>,             // Sites supported by scraper
    pub _ratelimit: (u64, Duration),     // Ratelimiter object that has yet to be created.
    pub _type: sharedtypes::ScraperType, // What type of scraper to use in matching.
}

///
/// Internal Scraping Reference for scrapers.
/// Copy pasta this code is and you should have a pretty good idea on what to do.
///
#[allow(dead_code)]
impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0,
            _name: "test.to_string".to_string(),
            _sites: crate::vec_of_strings!("example", "example1"),
            _ratelimit: (2, Duration::from_secs(2)),
            _type: sharedtypes::ScraperType::Automatic,
        }
    }
    pub fn version_get(&self) -> usize {
        self._version
    }
    pub fn version_set(&mut self, inp: usize) {
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

    pub fn url_get(&self, _params: Vec<String>) -> String {
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
    pub _library: HashMap<InternalScraper, libloading::Library>,
    _scraper: Vec<InternalScraper>,
}

impl ScraperManager {
    pub fn new() -> Self {
        ScraperManager {
            _string: Vec::new(),
            _sites: Vec::new(),
            _loaded: Vec::new(),
            _library: HashMap::new(),
            _scraper: Vec::new(),
        }
    }

    pub fn sites_get(&self) -> &Vec<Vec<String>> {
        &self._sites
    }

    pub fn scraper_get(&self) -> &Vec<InternalScraper> {
        &self._scraper
    }

    pub fn library_get(&self) -> &HashMap<InternalScraper, libloading::Library> {
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
                let lib;
                let pulled_successfully;
                unsafe {
                    lib = libloading::Library::new(&path).unwrap();
                    let plugindatafunc: libloading::Symbol<
                        unsafe extern "C" fn() -> InternalScraper,
                    > = lib.get(b"new").unwrap();
                    pulled_successfully = plugindatafunc();
                }

                /*let lib = unsafe { libloading::Library::new(&path).unwrap() };

                let funtwo: Result<
                    libloading::Symbol<unsafe extern "C" fn() -> InternalScraper>,
                    libloading::Error,
                > = unsafe { lib.get(b"new") };

                let pulled_successfully = unsafe { funtwo.unwrap()() };
                lib.close();*/

                // Loads version in memsafe way from scraper
                //let scraper = unsafe { funtwo.as_ref().unwrap()() };
                self._library.insert(pulled_successfully, lib);
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

            let version = each.0.version_get();

            if version < SUPPORTED_VERS {
                let msg = format!(
                    "PLUGIN LOAD: Loaded Version:{} Currently Supports:{}",
                    version, SUPPORTED_VERS
                );
                error!("{}", msg);
                //unsafe {mem::forget(funtwo.unwrap()());}
                panic!("{}", msg);
            }

            //let sites: Vec<String> = scraper.sites_get();

            //    self._sites.push(sites);
            self._scraper.push(each.0.clone());
        }
    }

    pub fn returnlibloading(&self, scraper: &InternalScraper) -> &libloading::Library {
        &self._library[scraper]
    }

    pub fn return_libloading_string(&self, input: &String) -> Option<&libloading::Library> {
        for each in self._library.keys() {
            if each._sites.contains(input) {
                return Some(self._library.get(each).unwrap());
            }
        }
        None
    }
}

pub fn scraper_file_regen(libloading: &libloading::Library) -> sharedtypes::ScraperFileRegen {
    let temp: libloading::Symbol<unsafe extern "C" fn() -> sharedtypes::ScraperFileRegen> =
        unsafe { libloading.get(b"scraper_file_regen\0").unwrap() };
    unsafe { temp() }
}

///
/// Used to generate a download link given the input data
///
pub fn scraper_file_return(
    libloading: &libloading::Library,
    regen: &sharedtypes::ScraperFileInput,
) -> sharedtypes::SubTag {
    let temp: libloading::Symbol<
        unsafe extern "C" fn(&sharedtypes::ScraperFileInput) -> sharedtypes::SubTag,
    > = unsafe { libloading.get(b"scraper_file_return\0").unwrap() };
    unsafe { temp(regen) }
}

///
/// Checks job storage type. If manual then do nothing if Automatic then it will store info before
/// the scraper even starts. Useful for storing API keys or anything similar to that.
///
pub fn cookie_need(libloading: &libloading::Library) -> (sharedtypes::ScraperType, String) {
    let temp: libloading::Symbol<unsafe extern "C" fn() -> (sharedtypes::ScraperType, String)> =
        unsafe { libloading.get(b"cookie_needed\0").unwrap() };
    unsafe { temp() }
}
///
/// Tells downloader to allow scraper to download.
///
pub fn scraper_download_get(libloading: &libloading::Library) -> bool {
    let temp: libloading::Symbol<unsafe extern "C" fn() -> bool> =
        unsafe { libloading.get(b"scraper_download_get\0").unwrap() };
    unsafe { temp() }
}
///
/// Should only be called when scraper needs to download something assuming scraper_download_get returns true.
/// TODO NOT YET IMPLEMENTED PROPERLY.
///
pub fn scraper_download(libloading: &libloading::Library, _params: String) -> bool {
    let temp: libloading::Symbol<unsafe extern "C" fn() -> bool> =
        unsafe { libloading.get(b"scraper_download\0").unwrap() };
    unsafe { temp() }
}

pub fn url_load(
    libloading: &libloading::Library,
    params: &Vec<sharedtypes::ScraperParam>,
) -> Vec<String> {
    let temp: libloading::Symbol<
        unsafe extern "C" fn(&Vec<sharedtypes::ScraperParam>) -> Vec<String>,
    > = unsafe { libloading.get(b"url_get\0").unwrap() };
    unsafe { temp(params) }
}
pub fn url_dump(
    params: &Vec<sharedtypes::ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
    arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    scraper: &InternalScraper,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let scrapermanager = arc_scrapermanager.lock().unwrap();
    let scraper_library = scrapermanager.library_get().get(&scraper).unwrap();
    let temp: libloading::Symbol<
        unsafe extern "C" fn(
            &Vec<sharedtypes::ScraperParam>,
            &sharedtypes::ScraperData,
        ) -> Vec<(String, sharedtypes::ScraperData)>,
    > = unsafe { scraper_library.get(b"url_dump\0").unwrap() };
    unsafe { temp(params, scraperdata) }
}
///
/// Calls a parser. Gives the HTML to the parser to parse.
///
pub fn parser_call(
    url_output: &String,
    actual_params: &sharedtypes::ScraperData,
    arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    scraper: &InternalScraper,
) -> Result<(sharedtypes::ScraperObject, sharedtypes::ScraperData), sharedtypes::ScraperReturn> {
    let scrapermanager = arc_scrapermanager.lock().unwrap();
    let scraper_library = scrapermanager.library_get().get(&scraper).unwrap();
    let temp: libloading::Symbol<
        unsafe extern "C" fn(
            &String,
            &sharedtypes::ScraperData,
        ) -> Result<
            (sharedtypes::ScraperObject, sharedtypes::ScraperData),
            sharedtypes::ScraperReturn,
        >,
    > = unsafe { scraper_library.get(b"parser\0").unwrap() };
    unsafe { temp(url_output, actual_params) }
} //ScraperObject

pub fn url_load_test(libloading: &libloading::Library, params: Vec<String>) -> Vec<String> {
    let temp: libloading::Symbol<unsafe extern "C" fn(&Vec<String>) -> Vec<String>> =
        unsafe { libloading.get(b"url_get\0").unwrap() };
    unsafe { temp(&params) }
}

pub fn db_upgrade_call(libloading: &libloading::Library, db_version: &usize) {
    let temp: libloading::Symbol<unsafe extern "C" fn(&usize)> =
        match unsafe { libloading.get(b"db_upgrade_call\0") } {
            Err(err) => {
                logging::error_log(&format!(
                    "Could not run scraper upgrade for db version {} because of {}.",
                    db_version, err
                ));
                return;
            }
            Ok(lib) => lib,
        };

    unsafe { temp(db_version) }
}
