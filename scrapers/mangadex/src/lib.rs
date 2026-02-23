use url::{Url, form_urlencoded};
use std::collections::HashMap;
use serde::de::{Deserializer, Error};
#[derive(Debug)]
pub struct MangaDexAPI {
    base_url: Url,
}

use serde::{Deserialize, Serialize};

fn empty_string_to_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, attempt to deserialize the value as an Option<String>
    let value: Option<String> = Option::deserialize(deserializer)?;

    // If the value is None (because of a null) or it's an empty string, return None
    match value {
        Some(v) if v.is_empty() => Ok(None),
        other => Ok(other),  // Otherwise return the string inside Option
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct MangaData {
    pub result: String,
    pub response: String,
    pub data: Vec<Manga>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manga {
    pub id: String,
    pub r#type: String, // `type` is a keyword in Rust, so we need to escape it
    pub attributes: MangaAttributes,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MangaAttributes {
    pub title: HashMap<String, String>, // A map of language codes to titles
    pub altTitles: Vec<HashMap<String, String>>,
    pub description: Description,
    pub isLocked: bool,
    pub links: Links,
    pub originalLanguage: String,
    #[serde(deserialize_with = "empty_string_to_none")]
    pub lastVolume: Option<String>,
    #[serde(deserialize_with = "empty_string_to_none")]
    pub lastChapter: Option<String>,
    #[serde(deserialize_with = "empty_string_to_none")]
    pub publicationDemographic: Option<String>,
    pub status: String,
    pub year: u32,
    #[serde(deserialize_with = "empty_string_to_none")]
    pub contentRating: Option<String>,
   pub tags: Vec<Tag>,
    pub state: String,
    pub chapterNumbersResetOnNewVolume: bool,
    pub createdAt: String,
    pub updatedAt: String,
    pub version: u32,
    pub availableTranslatedLanguages: Vec<String>,
    pub latestUploadedChapter: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AltTitle {
    pub en: Option<String>,
    pub ja: Option<String>,
    #[serde(rename = "ja-ro")]
    pub ja_ro: Option<String>, // Optional because it may not always exist
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Description {
    pub en: String,
    pub ja: Option<String>, // Optional because it may not always exist
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    pub mu: Option<String>,
    pub amz: Option<String>,
    pub mal: Option<String>,
    pub raw: Option<String>,
}

#[derive(Deserialize,Serialize, Debug)]
struct Tag {
    id: String,
    #[serde(rename = "type")]
    tag_type: String,
    attributes: TagAttributes,
    relationships: Vec<Relationship>,
}

#[derive(Deserialize, Debug, Serialize)]
struct TagAttributes {
    name: Name,
    description: Option<serde_json::Value>, // assuming description can be an empty object or null
    group: String,
    version: u32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Name {
    pub en: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub r#type: String,
    #[serde(rename = "related")]
    pub related: Option<String>, // Optional because not all relationships have this field
}impl MangaDexAPI {
    pub fn new() -> Self {
        MangaDexAPI {
            base_url: Url::parse("https://api.mangadex.org").unwrap(),
        }
    }

    fn build_url(&self, endpoint: &str, params: Option<HashMap<String, String>>) -> Url {
        let mut url = self.base_url.clone();
        url.set_path(endpoint);

        if let Some(query_params) = params {
            let query = form_urlencoded::Serializer::new(String::new())
                .extend_pairs(query_params)
                .finish();
            url.set_query(Some(&query));
        }

        url
    }

    pub fn get_manga(&self, manga_id: &str) -> Url {
        let endpoint = &format!("/manga/{}", manga_id);
        self.build_url(endpoint, None)
    }

    pub fn search_manga(&self, title: Option<&str>, author: Option<&str>, genre: Option<&str>, limit: Option<u32>) -> Url {
        let mut params = HashMap::new();
        
        if let Some(t) = title {
            params.insert("title".to_string(), t.to_string());
        }
        
        if let Some(a) = author {
            params.insert("author".to_string(), a.to_string());
        }
        
        if let Some(g) = genre {
            params.insert("genre".to_string(), g.to_string());
        }
        
        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }
        
        let endpoint = "/manga";
        self.build_url(endpoint, Some(params))
    }

    pub fn get_chapter(&self, manga_id: &str, chapter_id: &str) -> Url {
        let endpoint = &format!("/manga/{}/chapter/{}", manga_id, chapter_id);
        self.build_url(endpoint, None)
    }

    pub fn get_chapter_list(&self, manga_id: &str, limit: Option<u32>) -> Url {
        let mut params = HashMap::new();
        
        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }
        
        let endpoint = &format!("/manga/{}/chapters", manga_id);
        self.build_url(endpoint, Some(params))
    }

    pub fn get_user_info(&self, user_id: &str) -> Url {
        let endpoint = &format!("/user/{}", user_id);
        self.build_url(endpoint, None)
    }

    pub fn get_tag_list(&self) -> Url {
        let endpoint = "/tags";
        self.build_url(endpoint, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_func() {
let api = MangaDexAPI::new();

    // Example: Get a specific manga
    let manga_url = api.get_manga("a123b456-7890-abc1-def2-ghi3jkl4mno5");
    println!("Manga URL: {}", manga_url);

    // Example: Search for manga by title
    let search_url = api.search_manga(Some("Naruto"), None, None, Some(5));
    println!("Search URL: {}", search_url);

    // Example: Get manga chapters
    let chapters_url = api.get_chapter_list("a123b456-7890-abc1-def2-ghi3jkl4mno5", Some(3));
    println!("Chapters URL: {}", chapters_url);

    // Example: Get tags
    let tags_url = api.get_tag_list();
    println!("Tags URL: {}", tags_url);
// The JSON string you provided
    let json_data = r#"{"result":"ok","response":"collection","data":[{"id":"4c08c48b-4b4a-4ed7-a5be-19d2b4a8e2b2","type":"manga","attributes":{"title":{"ja-ro":"Isekai de Yakuza Hajimemashita: Yume no Henkyou Slow Life wa Oazuke-chuu"},"altTitles":[{"en":"I Became a Yakuza in Another World \u2014 My Peaceful Slow Life Will Have to Wait"},{"ja":"\u7570\u4e16\u754c\u3067\u30e4\u30af\u30b6\u59cb\u3081\u307e\u3057\u305f\uff5e\u5922\u306e\u8fba\u5883\u30b9\u30ed\u30fc\u30e9\u30a4\u30d5\u306f\u304a\u9810\u3051\u4e2d\uff5e"},{"ja-ro":"Isekai de Yakuza Hajimemashita"}],"description":{"en":"I was supposed to live a peaceful slow life, but the yakuza's boss's grand daughter got obsessively attached to me and somehow I ended up as the boss!?"},"isLocked":false,"links":{"mu":"7g0vqxu","amz":"https:\/\/www.amazon.co.jp\/dp\/4199509593","mal":"191218","raw":"https:\/\/unicorn.comic-ryu.jp\/series\/yakuzahajime"},"officialLinks":null,"originalLanguage":"ja","lastVolume":"","lastChapter":"","publicationDemographic":"seinen","status":"ongoing","year":2025,"contentRating":"safe","tags":[{"id":"0bc90acb-ccc1-44ca-a34a-b9f3a73259d0","type":"tag","attributes":{"name":{"en":"Reincarnation"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"423e2eae-a7a2-4a8b-ac03-a8351462d71d","type":"tag","attributes":{"name":{"en":"Romance"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"4d32cc48-9f00-4cca-9b5a-a839f0764984","type":"tag","attributes":{"name":{"en":"Comedy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"aafb99c1-7f60-43fa-b75f-fc9502ce29c7","type":"tag","attributes":{"name":{"en":"Harem"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"ace04997-f6bd-436e-b261-779182193d3d","type":"tag","attributes":{"name":{"en":"Isekai"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"e5301a23-ebd9-49dd-a0cb-2add944c7fe9","type":"tag","attributes":{"name":{"en":"Slice of Life"},"description":{},"group":"genre","version":1},"relationships":[]}],"state":"published","chapterNumbersResetOnNewVolume":false,"createdAt":"2026-02-12T23:34:28+00:00","updatedAt":"2026-02-17T22:26:03+00:00","version":8,"availableTranslatedLanguages":["en"],"latestUploadedChapter":"3b8fb9ec-436a-44de-8f61-e99d0f741d33"},"relationships":[{"id":"12e0e19b-f346-438b-b0ff-f8f70a787022","type":"author"},{"id":"e9a20f5d-15dd-42de-b737-a2e8876056e9","type":"artist"},{"id":"4754d08d-11ff-497b-979f-2ad3327d2b86","type":"cover_art"},{"id":"d4a129e5-b9dd-4d10-b629-fe4e21d9da23","type":"creator"}]},{"id":"b709ed9d-511f-4902-aba8-922c6296d677","type":"manga","attributes":{"title":{"ja-ro":"Isekai ni Kita Mitai dakedo Dou Sureba Yoi no Darou: Shachiku SE no My Pace Boukenki"},"altTitles":[{"ja":"\u7570\u4e16\u754c\u306b\u6765\u305f\u307f\u305f\u3044\u3060\u3051\u3069\u5982\u4f55\u3059\u308c\u3070\u826f\u3044\u306e\u3060\u308d\u3046\u3000\uff5e\u793e\u755c\uff33\uff25\u306e\u30de\u30a4\u30da\u30fc\u30b9\u5192\u967a\u8a18\uff5e"},{"en":"What Should I do in a Different World?"},{"ja-ro":"Isekai ni Kita Mitai dakedo Dou Sureba Ii no Darou: Shachiku SE no My Pace Boukenki"}],"description":{"en":"On a certain morning, an average 35 year old salaryman was suddenly transported to another world. \"I am without knowledge, without the ability to communicate, and I am especially not confident about my physical capabilities. I have no idea what to do. However, my current goal is to figure out how to keep living using the skills granted to me from the existence known as the interface. Hero? Demon King? I know nothing about that. I am just a salaryman.\"","ja":"\u6bce\u65e5\u4ed5\u4e8b\u8a70\u3081\u306e\u30b7\u30b9\u30c6\u30e0\u30a8\u30f3\u30b8\u30cb\u30a2\u3001\u524d\u5ddd\u5f70\u6d69\uff0835\u6b73\uff09\u3002\u4eca\u65e5\u3082\u75b2\u308c\u304c\u6b8b\u308b\u8eab\u4f53\u3067\u51fa\u52e4\u3057\u3088\u3046\u3068\u3001\u5bb6\u3092\u51fa\u305f\u77ac\u9593\u2015\u2015\u7a81\u7136\u7729\u3044\u5149\u306b\u5305\u307e\u308c\u3001\u6c17\u3065\u3051\u3070\u898b\u77e5\u3089\u306c\u68ee\u3067\u5012\u308c\u3066\u3044\u305f!?\u3000\u624b\u6301\u3061\u306e\u4ed5\u4e8b\u9053\u5177\u3068\u3001\u5c11\u3057\u306e\u30b5\u30d0\u30a4\u30d0\u30eb\u77e5\u8b58\u3092\u99c6\u4f7f\u3057\u3001\u5927\u81ea\u7136\u306e\u4e2d\u3092\u5f77\u5fa8\u3063\u3066\u3044\u305f\u3068\u3053\u308d\u3001\u91d1\u9aea\u78a7\u773c\u306e\u7f8e\u5c11\u5973\u3001\u30ea\u30b6\u30c6\u30a3\u30a2\u306b\u51fa\u4f1a\u3046\u3002\u732a\u306b\u8972\u308f\u308c\u304b\u3051\u3066\u3044\u305f\u3068\u3053\u308d\u3092\u306a\u3093\u3068\u304b\u52a9\u3051\u51fa\u3057\u305f\u5f70\u6d69\u306f\u3001\u6575\u610f\u304c\u7121\u3044\u3053\u3068\u3092\u793a\u305d\u3046\u3068\u3001\u5484\u55df\u306b\u6b66\u5668\u3092\u6295\u3052\u51fa\u3057\u3066\u8dea\u304f\u3002\u3057\u304b\u3057\u306a\u3093\u3068\u305d\u308c\u306f\u3053\u306e\u4e16\u754c\u306e\u300c\u30d7\u30ed\u30dd\u30fc\u30ba\u300d\u3060\u3063\u305f!?"},"isLocked":false,"links":{"al":"159191","ap":"isekai-ni-kita-mitai-dakedo-dou-sureba-yoi-no-darou-shachiku-se-no-my-pace-boukenki","bw":"series\/165928\/list","kt":"69833","mu":"6d53spz","nu":"it-seems-i-came-to-another-world-now-what-should-i-do","amz":"https:\/\/www.amazon.co.jp\/dp\/B0BQ3KS1L6","ebj":"https:\/\/ebookjapan.yahoo.co.jp\/books\/740029","mal":"178094","raw":"https:\/\/comic-boost.com\/series\/334"},"officialLinks":null,"originalLanguage":"ja","lastVolume":"","lastChapter":"","publicationDemographic":"seinen","status":"ongoing","year":2022,"contentRating":"safe","tags":[{"id":"423e2eae-a7a2-4a8b-ac03-a8351462d71d","type":"tag","attributes":{"name":{"en":"Romance"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"87cc87cd-a395-47af-b27a-93258283bbc6","type":"tag","attributes":{"name":{"en":"Adventure"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"ace04997-f6bd-436e-b261-779182193d3d","type":"tag","attributes":{"name":{"en":"Isekai"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"cdc58593-87dd-415e-bbc0-2ec27bf404cc","type":"tag","attributes":{"name":{"en":"Fantasy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"f4122d1c-3b44-44d0-9936-ff7502c39ad3","type":"tag","attributes":{"name":{"en":"Adaptation"},"description":{},"group":"format","version":1},"relationships":[]}],"state":"published","chapterNumbersResetOnNewVolume":false,"createdAt":"2022-12-24T15:45:15+00:00","updatedAt":"2026-02-04T02:25:59+00:00","version":13,"availableTranslatedLanguages":["id","en"],"latestUploadedChapter":"90d64713-71b5-4436-ab89-ab07599f1ea8"},"relationships":[{"id":"4f87fb8f-b21a-437e-8b84-4f52b435bc58","type":"author"},{"id":"1a2e8535-568c-42ec-884d-4ca1bc755977","type":"artist"},{"id":"776629d2-0a74-4ba9-91ae-9c375d1204d3","type":"cover_art"},{"id":"e1cb49f7-935b-4cb6-b575-ebd8e90e4566","type":"manga","related":"alternate_version"},{"id":"28ac2f72-9735-47c4-9cbb-6723672ba17a","type":"creator"}]},{"id":"870d53e7-199c-4af0-ae5f-279c1fca55fb","type":"manga","attributes":{"title":{"ja-ro":"Jashin no Mago: Dark Elf Shimai to Sugosu Isekai Hikikomori Seikatsu"},"altTitles":[{"ja":"\u90aa\u795e\u306e\u5b6b\u3000\u30c0\u30fc\u30af\u30a8\u30eb\u30d5\u59c9\u59b9\u3068\u904e\u3054\u3059\u7570\u4e16\u754c\u5f15\u304d\u3053\u3082\u308a\u751f\u6d3b"},{"en":"Grandson of the Evil God: Leading a Shut-In Life in Another World with Dark Elf Sisters"}],"description":{"en":"Satoshi Katakura, living a shut-in life, earns money through investments using the inheritance from his grandmother. One day, he wakes up to find that an otherworldly realm has spread outside his home. Appearing before him are dark elf sisters who revere his grandmother as the Evil God.\n\n**From this day forward, we pledge our eternal loyalty to you as our master!**\n\n*What is the sisters\u2019 true purpose? What is the real identity of his grandmother?*","ja":"\u7956\u6bcd\u306e\u907a\u7523\u3092\u5143\u624b\u306b\u6295\u8cc7\u3067\u7a3c\u304e\u3001\u5f15\u304d\u3053\u3082\u308a\u751f\u6d3b\u3092\u9001\u308b\u7247\u5009\u8061\u3002\u3042\u308b\u65e5\u76ee\u899a\u3081\u308b\u3068\u3001\u5bb6\u306e\u5916\u306b\u306f\u7570\u4e16\u754c\u304c\u5e83\u304c\u3063\u3066\u3044\u305f\u3002\u305d\u3053\u306b\u73fe\u308c\u305f\u306e\u306f\u3001\u8061\u306e\u7956\u6bcd\u3092\u300c\u90aa\u795e\u300d\u3068\u4ef0\u3050\u30c0\u30fc\u30af\u30a8\u30eb\u30d5\u306e\u59c9\u59b9\u3002\u300c\u6211\u3005\u306f\u672c\u65e5\u3088\u308a\u3001\u3042\u306a\u305f\u3092\u4e3b\u3068\u4ef0\u304e\u6c38\u9060\u306e\u5fe0\u8aa0\u3092\u6367\u3052\u307e\u3059\uff01\u300d\u3002\u59c9\u59b9\u306e\u76ee\u7684\u306f\uff1f\u7956\u6bcd\u306e\u6b63\u4f53\u306e\u771f\u5b9f\u306f\uff1f\u7269\u8a9e\u304c\u5927\u304d\u304f\u52d5\u304d\u51fa\u3059\u30fc\u30fc\u3053\u3068\u306f\u306a\u304f\u2025\u2025\u3002"},"isLocked":false,"links":{"al":"207186","ap":"jashin-no-mago-dark-elf-shimai-to-sugosu-isekai-hikikomori-seikatsu","bw":"series\/572402","amz":"https:\/\/www.amazon.co.jp\/dp\/B0GF683HMD","ebj":"https:\/\/ebookjapan.yahoo.co.jp\/books\/949676\/","mal":"186228","raw":"https:\/\/yanmaga.jp\/comics\/\u90aa\u795e\u306e\u5b6b_\u30c0\u30fc\u30af\u30a8\u30eb\u30d5\u59c9\u59b9\u3068\u904e\u3054\u3059\u7570\u4e16\u754c\u5f15\u304d\u3053\u3082\u308a\u751f\u6d3b"},"officialLinks":null,"originalLanguage":"ja","lastVolume":"","lastChapter":"","publicationDemographic":"seinen","status":"ongoing","year":2025,"contentRating":"suggestive","tags":[{"id":"4d32cc48-9f00-4cca-9b5a-a839f0764984","type":"tag","attributes":{"name":{"en":"Comedy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"a1f53773-c69a-4ce5-8cab-fffcd90b1565","type":"tag","attributes":{"name":{"en":"Magic"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"cdc58593-87dd-415e-bbc0-2ec27bf404cc","type":"tag","attributes":{"name":{"en":"Fantasy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"e5301a23-ebd9-49dd-a0cb-2add944c7fe9","type":"tag","attributes":{"name":{"en":"Slice of Life"},"description":{},"group":"genre","version":1},"relationships":[]}],"state":"published","chapterNumbersResetOnNewVolume":false,"createdAt":"2025-11-21T17:28:02+00:00","updatedAt":"2026-02-19T04:56:18+00:00","version":14,"availableTranslatedLanguages":["en"],"latestUploadedChapter":"6ec6445e-5e11-46b8-abf4-53c055111d32"},"relationships":[{"id":"0df5ac8b-8ab9-4fa7-b7b9-f9a73a265c0a","type":"author"},{"id":"6bc3f57c-dd8b-48ee-b432-c56054550ecc","type":"artist"},{"id":"bf1e7031-5e33-408b-bfdf-abb09b108436","type":"cover_art"},{"id":"27bde0e8-71b0-4bf2-8e25-2902a7b2dd4b","type":"creator"}]},{"id":"1081031e-7065-48fd-aa4b-8b388ed0c0ae","type":"manga","attributes":{"title":{"en":"I Live in Isekai"},"altTitles":[{"ja":"\u7570\u4e16\u754c\u306b\u751f\u304d\u308b"}],"description":{"en":"Michi gets into another world without any memory of his past life, now he is just following his new life... and a lots of secrets are being revealed and dark shades are started to cover the life of Michi!"},"isLocked":false,"links":{"raw":"https:\/\/www.pixiv.net\/user\/120721952\/series\/307779","engtl":"https:\/\/namicomi.com\/en\/title\/Sn38Zzvb\/i-live-in-isekai?utm_source=mangadex"},"officialLinks":null,"originalLanguage":"en","lastVolume":"","lastChapter":"","publicationDemographic":null,"status":"ongoing","year":2025,"contentRating":"suggestive","tags":[{"id":"07251805-a27e-4d59-b488-f0bfbec15168","type":"tag","attributes":{"name":{"en":"Thriller"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"36fd93ea-e8b8-445e-b836-358f02b3d33d","type":"tag","attributes":{"name":{"en":"Monsters"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"391b0423-d847-456f-aff0-8b0cfc03066b","type":"tag","attributes":{"name":{"en":"Action"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"39730448-9a5f-48a2-85b0-a70db87b1233","type":"tag","attributes":{"name":{"en":"Demons"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"3bb26d85-09d5-4d2e-880c-c34b974339e9","type":"tag","attributes":{"name":{"en":"Ghosts"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"3de8c75d-8ee3-48ff-98ee-e20a65c86451","type":"tag","attributes":{"name":{"en":"Animals"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"423e2eae-a7a2-4a8b-ac03-a8351462d71d","type":"tag","attributes":{"name":{"en":"Romance"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"4d32cc48-9f00-4cca-9b5a-a839f0764984","type":"tag","attributes":{"name":{"en":"Comedy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"5920b825-4181-4a17-beeb-9918b0ff7a30","type":"tag","attributes":{"name":{"en":"Boys' Love"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"5fff9cde-849c-4d78-aab0-0d52b2ee1d25","type":"tag","attributes":{"name":{"en":"Survival"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"631ef465-9aba-4afb-b0fc-ea10efe274a8","type":"tag","attributes":{"name":{"en":"Zombies"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"799c202e-7daa-44eb-9cf7-8a3c0441531e","type":"tag","attributes":{"name":{"en":"Martial Arts"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"81c836c9-914a-4eca-981a-560dad663e73","type":"tag","attributes":{"name":{"en":"Magical Girls"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"87cc87cd-a395-47af-b27a-93258283bbc6","type":"tag","attributes":{"name":{"en":"Adventure"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"891cf039-b895-47f0-9229-bef4c96eccd4","type":"tag","attributes":{"name":{"en":"Self-Published"},"description":{},"group":"format","version":1},"relationships":[]},{"id":"a1f53773-c69a-4ce5-8cab-fffcd90b1565","type":"tag","attributes":{"name":{"en":"Magic"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"aafb99c1-7f60-43fa-b75f-fc9502ce29c7","type":"tag","attributes":{"name":{"en":"Harem"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"ace04997-f6bd-436e-b261-779182193d3d","type":"tag","attributes":{"name":{"en":"Isekai"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"b29d6a3d-1569-4e7a-8caf-7557bc92cd5d","type":"tag","attributes":{"name":{"en":"Gore"},"description":{},"group":"content","version":1},"relationships":[]},{"id":"b9af3a63-f058-46de-a9a0-e0c13906197a","type":"tag","attributes":{"name":{"en":"Drama"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"cdad7e68-1419-41dd-bdce-27753074a640","type":"tag","attributes":{"name":{"en":"Horror"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"cdc58593-87dd-415e-bbc0-2ec27bf404cc","type":"tag","attributes":{"name":{"en":"Fantasy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"d7d1730f-6eb0-4ba6-9437-602cac38664c","type":"tag","attributes":{"name":{"en":"Vampires"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"da2d50ca-3018-4cc0-ac7a-6b7d472a29ea","type":"tag","attributes":{"name":{"en":"Delinquents"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"dd1f77c5-dea9-4e2b-97ae-224af09caf99","type":"tag","attributes":{"name":{"en":"Monster Girls"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"e5301a23-ebd9-49dd-a0cb-2add944c7fe9","type":"tag","attributes":{"name":{"en":"Slice of Life"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"eabc5b4c-6aff-42f3-b657-3e90cbd00b75","type":"tag","attributes":{"name":{"en":"Supernatural"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"ee968100-4191-4968-93d3-f82d72be7e46","type":"tag","attributes":{"name":{"en":"Mystery"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"f4122d1c-3b44-44d0-9936-ff7502c39ad3","type":"tag","attributes":{"name":{"en":"Adaptation"},"description":{},"group":"format","version":1},"relationships":[]},{"id":"f8f62932-27da-4fe4-8ee1-6779a8c5edba","type":"tag","attributes":{"name":{"en":"Tragedy"},"description":{},"group":"genre","version":1},"relationships":[]}],"state":"published","chapterNumbersResetOnNewVolume":false,"createdAt":"2025-08-05T04:16:58+00:00","updatedAt":"2026-01-18T18:23:50+00:00","version":16,"availableTranslatedLanguages":["en"],"latestUploadedChapter":"c0933cb6-c316-4fb5-bb48-32eab3c1b6f8"},"relationships":[{"id":"5b4277c7-9522-4a1a-83a0-e14f3a22b4fa","type":"author"},{"id":"041d9307-5483-4c00-9199-43f8d703e35a","type":"artist"},{"id":"3009fd73-8708-4211-afb4-52a10615d2b2","type":"artist"},{"id":"376d57b4-bb64-40ee-9008-6bd9689c7f12","type":"artist"},{"id":"5b4277c7-9522-4a1a-83a0-e14f3a22b4fa","type":"artist"},{"id":"7b9fbe03-a35d-4adb-962d-5d374f2a87f0","type":"artist"},{"id":"8ec2577e-a7d3-451f-a1a8-ac6b211433be","type":"artist"},{"id":"af4ec72b-e005-4837-b994-8d97b0feefc6","type":"artist"},{"id":"21c4cdc0-26d1-47e3-8dbc-4f756869515f","type":"cover_art"},{"id":"f56aa6ad-6e44-4018-8a3d-43aaa00bc383","type":"creator"}]},{"id":"f97979cc-c0dd-42e2-a32b-c56f8047f7ba","type":"manga","attributes":{"title":{"ja-ro":"Ore dake Fuguu Skill no Isekai Shoukan Hangyakuki ~Saijaku Skill \"Kyuushuu\" ga Subete wo Nomikomu made~"},"altTitles":[{"ja":"\u4ffa\u3060\u3051\u4e0d\u9047\u30b9\u30ad\u30eb\u306e\u7570\u4e16\u754c\u53ec\u559a\u53db\u9006\u8a18\uff5e\u6700\u5f31\u30b9\u30ad\u30eb\u3010\u5438\u53ce\u3011\u304c\u5168\u3066\u3092\u98f2\u307f\u8fbc\u3080\u307e\u3067\uff5e"},{"ja-ro":"Ore dake Fuguu Skill no Isekai Shoukan Hangyakuki ~Saijaku Skill \"Kyuushuu\" ga Subete o Nomikomu made~"},{"en":"I'm the Only One with a Failure of a Skill in Another World's Summoning Rebellion: Until the Weakest Skill [Absorption] Swallows Everything"},{"en":"I'm the Only One with Unfavorable Skills, Isekai Summoning Rebellion"}],"description":{"en":"Nakatani Yuto, a high school student, and his classmates are suddenly summoned to another world as \"heroes\". While his classmates are given powerful skills, Yuto is given the unfortunate skill of [Absorb]. Yuto, who has no use for it, is mortally wounded in a battle with a demon, but his classmates leave him to die. But at that moment, Yuto hears a voice out of nowhere...?","ja":"\u9ad8\u6821\u751f\u306e\u4e2d\u8c37\u5915\u6597\u306f\u7a81\u7136\u30af\u30e9\u30b9\u30e1\u30a4\u30c8\u3068\u5171\u306b\u7570\u4e16\u754c\u3078\u300c\u52c7\u8005\u300d\u3068\u3057\u3066\u53ec\u559a\u3055\u308c\u3066\u3057\u307e\u3046\u3002\u30af\u30e9\u30b9\u30e1\u30a4\u30c8\u9054\u304c\u5f37\u529b\u306a\u6280\u80fd\uff08\u30b9\u30ad\u30eb\uff09\u3092\u6388\u304b\u308b\u306a\u304b\u3001\u30e6\u30a6\u30c8\u304c\u6388\u304b\u3063\u305f\u306e\u306f\u4e0d\u9047\u30b9\u30ad\u30eb\u3010\u5438\u3011\u3067\u3042\u3063\u305f\u3002 \u4f7f\u3044\u7269\u306b\u306a\u3089\u306a\u3044\u30e6\u30a6\u30c8\u306f\u9b54\u7269\u3068\u306e\u6226\u95d8\u3067\u81f4\u547d\u50b7\u3092\u8ca0\u3046\u304c\u3001\u30af\u30e9\u30b9\u30e1\u30a4\u30c8\u306f\u5f7c\u3092\u898b\u6bba\u3057\u306b\u2026\u3002\u305d\u306e\u77ac\u9593\u3001\u30e6\u30a6\u30c8\u306b\u306f\u3069\u3053\u304b\u3089\u3068\u3082\u306a\u304f\u201c\u58f0\u201d\u304c\u805e\u3053\u3048\u3066\u2026\uff01\uff1f"},"isLocked":false,"links":{"al":"158654","ap":"ore-dake-fuguu-skill-no-isekai-shoukan-hangyakuki","bw":"series\/388493","kt":"65193","mu":"7cdy9a5","amz":"https:\/\/www.amazon.co.jp\/dp\/B0C5WZ796P","ebj":"https:\/\/ebookjapan.yahoo.co.jp\/books\/741455","mal":"151383","raw":"https:\/\/tonarinoyj.jp\/episode\/316112896809499266"},"officialLinks":null,"originalLanguage":"ja","lastVolume":"","lastChapter":"","publicationDemographic":"seinen","status":"ongoing","year":2022,"contentRating":"suggestive","tags":[{"id":"36fd93ea-e8b8-445e-b836-358f02b3d33d","type":"tag","attributes":{"name":{"en":"Monsters"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"391b0423-d847-456f-aff0-8b0cfc03066b","type":"tag","attributes":{"name":{"en":"Action"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"39730448-9a5f-48a2-85b0-a70db87b1233","type":"tag","attributes":{"name":{"en":"Demons"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"4d32cc48-9f00-4cca-9b5a-a839f0764984","type":"tag","attributes":{"name":{"en":"Comedy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"5fff9cde-849c-4d78-aab0-0d52b2ee1d25","type":"tag","attributes":{"name":{"en":"Survival"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"87cc87cd-a395-47af-b27a-93258283bbc6","type":"tag","attributes":{"name":{"en":"Adventure"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"97893a4c-12af-4dac-b6be-0dffb353568e","type":"tag","attributes":{"name":{"en":"Sexual Violence"},"description":{},"group":"content","version":1},"relationships":[]},{"id":"a1f53773-c69a-4ce5-8cab-fffcd90b1565","type":"tag","attributes":{"name":{"en":"Magic"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"aafb99c1-7f60-43fa-b75f-fc9502ce29c7","type":"tag","attributes":{"name":{"en":"Harem"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"ac72833b-c4e9-4878-b9db-6c8a4a99444a","type":"tag","attributes":{"name":{"en":"Military"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"ace04997-f6bd-436e-b261-779182193d3d","type":"tag","attributes":{"name":{"en":"Isekai"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"b29d6a3d-1569-4e7a-8caf-7557bc92cd5d","type":"tag","attributes":{"name":{"en":"Gore"},"description":{},"group":"content","version":1},"relationships":[]},{"id":"b9af3a63-f058-46de-a9a0-e0c13906197a","type":"tag","attributes":{"name":{"en":"Drama"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"cdc58593-87dd-415e-bbc0-2ec27bf404cc","type":"tag","attributes":{"name":{"en":"Fantasy"},"description":{},"group":"genre","version":1},"relationships":[]},{"id":"da2d50ca-3018-4cc0-ac7a-6b7d472a29ea","type":"tag","attributes":{"name":{"en":"Delinquents"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"dd1f77c5-dea9-4e2b-97ae-224af09caf99","type":"tag","attributes":{"name":{"en":"Monster Girls"},"description":{},"group":"theme","version":1},"relationships":[]},{"id":"fad12b5e-68ba-460e-b933-9ae8318f5b65","type":"tag","attributes":{"name":{"en":"Gyaru"},"description":{},"group":"theme","version":1},"relationships":[]}],"state":"published","chapterNumbersResetOnNewVolume":false,"createdAt":"2022-09-07T19:11:02+00:00","updatedAt":"2026-02-04T15:21:06+00:00","version":39,"availableTranslatedLanguages":["en","fr","pt-br","id","es-la"],"latestUploadedChapter":"1384b513-55aa-4cd6-b20b-345503ed7527"},"relationships":[{"id":"6c4b2523-d250-44f0-956d-eccb10447767","type":"author"},{"id":"c4d82183-0d19-4925-a91f-5b906adfc80a","type":"artist"},{"id":"5f768555-f9d9-4232-a2d8-bf42eccf906a","type":"cover_art"},{"id":"6640d5cb-18a0-4264-8c0a-9dcbe39be5ef","type":"creator"}]}],"limit":5,"offset":0,"total":944}"#;

    // Parse the JSON into the Response struct
    let parsed_response: MangaData = serde_json::from_str(json_data).unwrap();

    // Output the parsed response
    println!("{:#?}", parsed_response);
    }
}
