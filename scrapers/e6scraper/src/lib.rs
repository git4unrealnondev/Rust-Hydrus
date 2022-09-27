use json;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::time::Duration;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub struct InternalScraper {
    _version: f32,
    _name: String,
    _sites: Vec<String>,
    _ratelimit: (u64, Duration),
}

impl InternalScraper {
    pub fn new() -> Self {
        InternalScraper {
            _version: 0.001,
            _name: "e6scraper".to_string(),
            _sites: vec_of_strings!("e6", "e621", "e621.net"),
            _ratelimit: (2, Duration::from_secs(1)),
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

    if params.len() == 0 {
        return "".to_string();
    }

    if params.len() == 1 {
        formatted = format!("{}{}{}", &url, &tag_store, &params[0].replace(" ", "+"));
        return format!("{}{}{}", formatted, page, pagenum);
    }

    if params.len() == 2 {
        formatted = format!(
            "{}{}{}{}",
            &url,
            &params[1],
            &tag_store,
            &params[0].replace(" ", "+")
        );
    }
    return format!("{}{}{}", formatted, page, pagenum);
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
pub fn url_get(params: &Vec<String>) -> Vec<String> {
    let mut ret = Vec::new();
    ret.push(build_url(params, 1));
    return ret;
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

    return (
        "manual".to_string(),
        format!("?login={}&api_key={}", user, api),
    );
}
///
/// Gets url to query cookie url.
/// Not used or implemented corrently. :D
///
#[no_mangle]
pub fn cookie_url() -> String {
    return "e6scraper_cookie".to_string();
}

///
/// Gets all items from array in json and returns it into the hashmap.
/// The key is the sub value.
///
fn retvec(vecstr: &mut HashMap<String, Vec<String>>, jso: &json::JsonValue, sub: &str) {
    let mut vec = Vec::new();

    for each in jso[sub].members() {
                vec.push(each.to_string());
            }
    vecstr.insert(sub.to_string(), vec);
}

///
/// Parses return from download.
///
#[no_mangle]
pub fn parser(params: &String) -> Result<HashMap<String, HashMap<String, Vec<String>>>, &'static str> {
    let mut vecvecstr: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    //for each in params.keys() {
        //dbg!(params);
        //dbg!(json::parse(params));
        let js = json::parse(params).unwrap();
        //dbg!(&js["posts"]);
        //dbg!(&js["posts"].len());
        let mut file = File::create("main1.json").unwrap();

        // Write a &str in the file (ignoring the result).
        writeln!(&mut file, "{}", js.to_string()).unwrap();

        //let vecstr: HashMap<String> = Vec::new();
        //let vecvecstr: Vec<Vec<String>> = Vec::new();

        //for each in 0..js["posts"].len() {
        //    dbg!(&js["posts"][each]);
        //}
        //dbg!(&js[each]);
        if js["posts"].len() == 0 {return Err("NothingHere")}

        for inc in 0..js["posts"].len() {
            let mut vecstr: HashMap<String, Vec<String>> = HashMap::new();
            //dbg!(&js["posts"][inc]["tags"]["general"].entries());
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "general");
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "species");
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "character");
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "copyright");
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "artist");
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "lore");
            retvec(&mut vecstr, &js["posts"][inc]["tags"], "meta");
            retvec(&mut vecstr, &js["posts"][inc], "sources");
            retvec(&mut vecstr, &js["posts"][inc], "pools");
            //retvec(&mut vecstr, &js["posts"][inc]["relationships"], "parent_id");
            retvec(&mut vecstr, &js["posts"][inc]["relationships"], "children");

            // Filtering for parents
            //if js["posts"][inc]["relationships"]["parent_id"].to_string() != "null".to_string() {
            //vecstr.insert("parent_id".to_string(), [js["posts"][inc]["relationships"]["parent_id"].to_string()].to_vec());
            //dbg!(js["posts"][inc]["relationships"]["parent_id"].to_string());
            //}
            vecstr.insert("md5".to_string(), [js["posts"][inc]["file"]["md5"].to_string()].to_vec());
            vecstr.insert("id".to_string(), [js["posts"][inc]["id"].to_string()].to_vec());

            vecvecstr.insert(js["posts"][inc]["file"]["url"].to_string(), vecstr);

       // }
    }


    return Ok(vecvecstr);
}
///
/// Should this scraper handle anything relating to downloading.
///
#[no_mangle]
pub fn scraper_download_get() -> bool {
    false
}
