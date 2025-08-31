use std::time::Duration;

#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

static PLUGIN_NAME: &str = "File Downloader";
static PLUGIN_DESCRIPTION: &str =
    "Tries to download files directly if this plugin can recognize a url.";

pub const REGEX_COLLECTIONS: &str = r"(http(s)?://www.|((www.|http(s)?://)))[a-zA-Z0-9-].[a-zA-Z0-9-_.]*/[a-zA-Z0-9/_%-]+\.[a-zA-Z0-9/_%\.?=&-]+";

#[no_mangle]
pub fn get_global_info() -> Vec<sharedtypes::GlobalPluginScraper> {
    let tag_vec = (
        Some(sharedtypes::SearchType::Regex(REGEX_COLLECTIONS.into())),
        vec![],
        vec!["source_url".to_string()],
    );

    let callbackvec = vec![sharedtypes::GlobalCallbacks::Tag(tag_vec)];

    let mut plugin = sharedtypes::return_default_globalpluginparser();
    plugin.name = PLUGIN_NAME.to_string();
    plugin.storage_type = Some(sharedtypes::ScraperOrPlugin::Plugin(
        sharedtypes::PluginInfo2 {
            com_channel: true,
            redirect: None,
        },
    ));
    plugin.callbacks = callbackvec;

    vec![plugin]
}

#[no_mangle]
pub fn on_regex_match(
    tag: &String,
    tag_ns: &String,
    regex_match: &String,
    callback: &Option<sharedtypes::SearchType>,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut out = Vec::new();
    if regex_match.contains("bsky.app") {
        return out;
    }
    dbg!(tag, tag_ns);

    let subtag = sharedtypes::SubTag {
        namespace: sharedtypes::GenericNamespaceObj {
            name: tag_ns.to_string(),
            description: None,
        },
        tag: tag.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        limit_to: None,
    };

    let taginfo = sharedtypes::TagObject {
        namespace: sharedtypes::GenericNamespaceObj {
            name: "source_url".to_string(),
            description: None,
        },
        tag: regex_match.to_string(),
        tag_type: sharedtypes::TagType::Normal,
        relates_to: Some(subtag),
    };

    let earlyexistag = sharedtypes::Tag {
        tag: regex_match.to_string(),
        namespace: sharedtypes::GenericNamespaceObj {
            name: "source_url".to_string(),
            description: None,
        },
    };

    let file = sharedtypes::FileObject {
        source: Some(sharedtypes::FileSource::Url(regex_match.to_string())),
        hash: sharedtypes::HashesSupported::None,
        tag_list: vec![taginfo],
        skip_if: vec![sharedtypes::SkipIf::FileTagRelationship(earlyexistag)],
    };
    let ratelimit = (1, Duration::from_secs(1));

    let mut default_job = sharedtypes::return_default_jobsobj();

    default_job.site = "direct download".to_string();
    default_job.param = vec![sharedtypes::ScraperParam::Url(regex_match.into())];
    default_job.jobmanager = sharedtypes::DbJobsManager {
        jobtype: sharedtypes::DbJobType::FileUrl,
        recreation: None,
    };

    out.push(sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: None,
            setting: None,
            relationship: None,
            parents: None,
            jobs: Some(vec![default_job]),
            namespace: None,
            file: None,
        },
    ]));

    /*client::job_add(
        None,
        0,
        0,
        "direct download".to_string(),
        regex_match.to_string(),
        true,
        sharedtypes::CommitType::StopOnNothing,
        sharedtypes::DbJobType::FileUrl,
        BTreeMap::new(),
        BTreeMap::new(),
        sharedtypes::DbJobsManager {
            jobtype: sharedtypes::DbJobType::FileUrl,
            recreation: None,
        },
    );*/
    out
    //client::add_file_nonblocking(file, ratelimit);
}
