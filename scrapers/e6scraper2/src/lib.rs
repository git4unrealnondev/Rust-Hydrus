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
        let mut vecs: Vec<String> = Vec::new();
        for each in &self._sites {
            vecs.push(each.to_string());
        }
        return vecs;
    }
}

#[no_mangle]
pub fn new() -> InternalScraper {
    return InternalScraper::new();
}
