use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct PluginManager {
    _plugin: HashMap<String, (String, String, Vec<String>)>,
}

///
/// Plugin Manager Handler
///
impl PluginManager {
    pub fn new(pluginsloc: String) -> Self {
        let reftoself = PluginManager {
            _plugin: HashMap::new(),
        };

        reftoself.load_plugins(&pluginsloc);

        reftoself
    }

    ///
    /// Loads plugins into plugin manager
    ///
    fn load_plugins(&self, pluginsloc: &String) {
        let plugin_path = Path::new(pluginsloc);

        // Errors out if I cant create a folder
        if !plugin_path.exists() {
            let path_check = fs::create_dir_all(&plugin_path);
            match path_check {
                Ok(_) => (),
                Err(_) => panic!(
                    "{}",
                    format!("CANNOT CREATE FOLDER: {} DUE TO PERMISSIONS.", &pluginsloc)
                ),
            }
        }

        let dirs = fs::read_dir(&plugin_path).unwrap();

        for entry in dirs {
            let refs = entry.as_ref().unwrap();
            if refs.path().is_dir() {
                dbg!(&refs.path(), "TRUE", &refs.path().file_name().unwrap());
            }
            //if refs.as_ref().unwrap().path().is_dir() {dbg!(entry, "TRUE");}
            else {
                dbg!(&entry.unwrap().path(), "FALSE");
            }
        }
    }
}
