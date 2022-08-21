use std::io;
use std::io::BufRead;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub struct InternalScraper {
    _version: f32,
    _name: String,
    _sites: Vec<String>,
}

impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0.001,
            _name: "e6scraper".to_string(),
            _sites: vec_of_strings!("e6", "e621", "e621.net"),
        }
    }
    pub fn version_get(&self) -> f32 {
        return self._version;
    }
    pub fn name_get(&self) -> &String {
        &self._name
    }
    pub fn name_put(&mut self, inp: String) {
        self._name = inp;
    }
    pub fn sites_get(&self) -> Vec<String> {
        println!("AHAGAFAD");
        let mut vecs: Vec<String> = Vec::new();
        for each in &self._sites {
            vecs.push(each.to_string());
        }
        return vecs;
    }
}
///
/// Builds the URL for scraping activities.
///
fn build_url(params: &Vec<String>, pagenum: u64) -> String {
    let url = "https://e621.net/posts.json";
    let tag_store = "&tags=";
    let page = "&page=";
    let startpage = 1;
    let mut formatted: String = "".to_string();

    if params.len() == 0 {return "".to_string();}

    if params.len() == 1 {

        formatted = format!("{}{}{}", &url, &tag_store, &params[0].replace(" ", "+"));
    return format!("{}{}{}", formatted, page, pagenum);}

    if params.len() == 2 {

        for each in params {
            dbg!(each);
        }

        formatted= format!("{}{}{}{}", &url, &params[1], &tag_store, &params[0].replace(" ", "+"));

    }


     return format!("{}{}{}", formatted, page, pagenum);
    /*let mut formatted = format!("{}{}{}", &url, &tag_store, &params.replace(" ", "+"));
    let parzd: Vec<&str> = params.split(" ").collect::<Vec<&str>>();
    let mut parsed: Vec<String> = Vec::new();
    for a in parzd {
        parsed.push(a.to_string());
    }
    if pagenum == startpage {
        formatted = format!("{}{}{}", formatted, page, pagenum);
    }
    if pagenum > startpage {
        formatted = format!("{}{}{}", formatted, page,pagenum);
    }
    return formatted;*/
}
///
/// Reutrns an internal scraper object.
/// Only really useful to store variables. not useful for calling functions. :C
///
#[no_mangle]
pub fn new() -> InternalScraper {
    return InternalScraper::new();
}
///
/// Returns one url from the parameters.
///
#[no_mangle]
pub fn url_get(params: &Vec<String>) -> String {
    build_url(params, 1)
}
///
/// Dumps a list of urls to scrape
///
#[no_mangle]
pub fn url_dump(params: &Vec<String>) -> Vec<String> {
    let mut ret = Vec::new();
    let hardlimit = 751;
    for i in 1..hardlimit {
        let a = build_url(params, i);
        ret.push(a);
    }
    return ret;
}
///
/// Returns bool true or false if a cookie is needed. If so return the cookie name in storage
///
#[no_mangle]
pub fn cookie_needed() -> (String, String) {
    println!("Enter E6 Username");
    let user = io::stdin().lock().lines().next().unwrap().unwrap();
    println!("Enter E6 API Key");
    let api = io::stdin().lock().lines().next().unwrap().unwrap();

    return ("manual".to_string(), format!("?login={}&api_key={}", user, api))
}
///
/// Gets url to query cookie url.
/// Not used or implemented corrently. :D
///
#[no_mangle]
pub fn cookie_url() -> String {
    return "e6scraper_cookie".to_string()
}
