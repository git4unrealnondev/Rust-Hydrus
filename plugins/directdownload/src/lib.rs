use std::time::Duration;

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

static PLUGIN_NAME: &str = "File Downloader";

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
    tag_name: &str,
    tag_namespace: &sharedtypes::GenericNamespaceObj,
    regex_match: &str,
    _callback: &Option<sharedtypes::SearchType>,
) -> Vec<sharedtypes::DBPluginOutputEnum> {
    let mut out = Vec::new();
    if regex_match.contains("bsky.app") {
        return out;
    }

    let subtag = sharedtypes::SubTag {
        namespace: tag_namespace.clone(),
        tag: tag_name.to_string(),
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

    let tag_list = vec![sharedtypes::FileTagAction {
        tags: vec![taginfo],
        ..Default::default()
    }];

    let _file = sharedtypes::FileObjectV1 {
        source: Some(sharedtypes::FileSource::Url(vec![regex_match.to_string()])),
        hash: sharedtypes::HashesSupported::None,
        tag_list,
        skip_if: vec![sharedtypes::SkipIf::FileTagRelationship(earlyexistag)],
        ..Default::default()
    };
    let _ratelimit = (1, Duration::from_secs(1));

    let job = sharedtypes::DbJobsObj {
        site: "direct download".to_string(),
        param: vec![sharedtypes::ScraperParam::Url(regex_match.into())],
        jobmanager: sharedtypes::DbJobsManager {
            jobtype: sharedtypes::DbJobType::FileUrl,
            recreation: None,
        },
        ..Default::default()
    };

    out.push(sharedtypes::DBPluginOutputEnum::Add(vec![
        sharedtypes::DBPluginOutput {
            tag: vec![],
            setting: vec![],
            relationship: vec![],
            jobs: vec![job],
            file: vec![],
        },
    ]));

    out
}
