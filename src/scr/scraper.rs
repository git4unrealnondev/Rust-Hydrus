use crate::sharedtypes::{ScraperParam, SiteStruct};

use crate::{logging, plugins, sharedtypes};
use log::error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

static SUPPORTED_VERS: usize = 0;

pub struct ScraperManager {
    _string: Vec<String>,
    _loaded: Vec<bool>,
    sites: HashMap<SiteStruct, Vec<String>>,
    pub _library: HashMap<SiteStruct, libloading::Library>,
    _scraper: Vec<SiteStruct>,
}

impl ScraperManager {
    pub fn new() -> Self {
        ScraperManager {
            sites: HashMap::new(),
            _string: Vec::new(),
            _loaded: Vec::new(),
            _library: HashMap::new(),
            _scraper: Vec::new(),
        }
    }
    pub fn debug(&self) {
        dbg!(&self._string);
        dbg!(&self._loaded);
        dbg!(&self._library);
        dbg!(&self._scraper);
    }

    pub fn sites_get(&self, site_struct: &sharedtypes::SiteStruct) -> Vec<String> {
        if let Some(sites) = self.sites.get(site_struct) {
            sites.clone()
        } else {
            Vec::new()
        }
    }

    pub fn scraper_get(&self) -> &Vec<SiteStruct> {
        &self._scraper
    }

    pub fn library_get(&self) -> &HashMap<SiteStruct, libloading::Library> {
        &self._library
    }

    pub fn load(&mut self, scraperfolder: String, libpath: String, libext: String) {
        for scraper_path in
            plugins::get_loadable_paths(&scraperfolder, &plugins::LoadableType::Release).iter()
        {
            self._string
                .push(scraper_path.to_string_lossy().to_string());
            let lib;
            let pulled_successfully;
            unsafe {
                lib = libloading::Library::new(scraper_path).unwrap();
                let plugindatafunc: libloading::Symbol<
                    unsafe extern "C" fn() -> Vec<sharedtypes::SiteStruct>,
                > = lib.get(b"new").unwrap();
                pulled_successfully = plugindatafunc();
            }

            for site in pulled_successfully {
                let lib_storage;
                unsafe {
                    lib_storage = libloading::Library::new(scraper_path).unwrap();
                }

                let version = site.version;
                if version < SUPPORTED_VERS {
                    let msg = format!(
                        "PLUGIN LOAD: Loaded Version:{} Currently Supports:{}",
                        version, SUPPORTED_VERS
                    );
                    error!("{}", msg);
                    panic!("{}", msg);
                }
                self._scraper.push(site.clone());
                self.sites.insert(site.clone(), site.sites.clone());
                self._library.insert(site, lib_storage);
            }
        }
    }

    pub fn returnlibloading(&self, scraper: &SiteStruct) -> &libloading::Library {
        &self._library[scraper]
    }

    pub fn return_libloading_string(&self, input: &String) -> Option<&libloading::Library> {
        for each in self._library.keys() {
            if each.sites.contains(input) {
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
pub fn cookie_need_LEGACY(libloading: &libloading::Library) -> (sharedtypes::ScraperType, String) {
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
    scraper: &SiteStruct,
) -> Vec<(String, sharedtypes::ScraperData)> {
    let scrapermanager = arc_scrapermanager.lock().unwrap();
    let scraper_library = scrapermanager.library_get().get(scraper).unwrap();
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
    actual_params: &Vec<ScraperParam>,
    scraperdata: &sharedtypes::ScraperData,
    arc_scrapermanager: Arc<Mutex<ScraperManager>>,
    scraper: &SiteStruct,
) -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn> {
    let scrapermanager = arc_scrapermanager.lock().unwrap();
    let scraper_library = scrapermanager.library_get().get(scraper).unwrap();
    let temp: libloading::Symbol<
        unsafe extern "C" fn(
            &String,
            &Vec<sharedtypes::ScraperParam>,
            &sharedtypes::ScraperData,
        )
            -> Result<sharedtypes::ScraperObject, sharedtypes::ScraperReturn>,
    > = unsafe { scraper_library.get(b"parser\0").unwrap() };
    unsafe { temp(url_output, actual_params, scraperdata) }
} //ScraperObject

pub fn url_load_test(libloading: &libloading::Library, params: Vec<String>) -> Vec<String> {
    let temp: libloading::Symbol<unsafe extern "C" fn(&Vec<String>) -> Vec<String>> =
        unsafe { libloading.get(b"url_get\0").unwrap() };
    unsafe { temp(&params) }
}

pub fn db_upgrade_call(
    libloading: &libloading::Library,
    db_version: &usize,
    site_struct: &sharedtypes::SiteStruct,
) {
    let temp: libloading::Symbol<unsafe extern "C" fn(&usize, &sharedtypes::SiteStruct)> =
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

    unsafe { temp(db_version, site_struct) }
}

///
/// Gets called onstartup of the software
///
pub fn on_start(libloading: &libloading::Library, site_struct: &sharedtypes::SiteStruct) {
    let temp: libloading::Symbol<unsafe extern "C" fn(&sharedtypes::SiteStruct)> =
        match unsafe { libloading.get(b"on_start\0") } {
            Err(err) => {
                return;
            }
            Ok(lib) => lib,
        };

    unsafe { temp(site_struct) }
}

#[cfg(test)]
pub(crate) mod test_scrapermanager {
    use super::ScraperManager;

    pub fn create_default() -> ScraperManager {
        ScraperManager::new()
    }
    pub fn emulate_loaded() -> ScraperManager {
        let mut scrapermanager = create_default();

        scrapermanager
    }
}
