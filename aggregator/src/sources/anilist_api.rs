use cached::proc_macro::cached;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use std::{error::Error, fmt, fs::File};

#[derive(Debug)]
struct AniListError {
    message: String,
}

impl fmt::Display for AniListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl Error for AniListError {}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    url: String,
    access_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MediaType {
    Anime,
    Manga,
}

impl MediaType {
    fn from(r#type: Option<&str>) -> Option<MediaType> {
        match r#type {
            Some("ANIME") => Some(MediaType::Anime),
            Some("MANGA") => Some(MediaType::Manga),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MediaStatus {
    Current,
    Planning,
    Completed,
    Dropped,
    Paused,
    Repeating,
}

impl MediaStatus {
    fn from(status: Option<&str>) -> Option<MediaStatus> {
        match status {
            Some("CURRENT") => Some(MediaStatus::Current),
            Some("PLANNING") => Some(MediaStatus::Planning),
            Some("COMPLETED") => Some(MediaStatus::Completed),
            Some("DROPPED") => Some(MediaStatus::Dropped),
            Some("PAUSED") => Some(MediaStatus::Paused),
            Some("REPEATING") => Some(MediaStatus::Repeating),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MediaFormat {
    TV,
    TVShort,
    Movie,
    Special,
    OVA,
    ONA,
    Music,
    Manga,
    Novel,
    OneShot,
}

impl MediaFormat {
    fn from(format: Option<&str>) -> Option<MediaFormat> {
        match format {
            Some("TV") => Some(MediaFormat::TV),
            Some("TV_SHORT") => Some(MediaFormat::TVShort),
            Some("MOVIE") => Some(MediaFormat::Movie),
            Some("SPECIAL") => Some(MediaFormat::Special),
            Some("OVA") => Some(MediaFormat::OVA),
            Some("ONA") => Some(MediaFormat::ONA),
            Some("MUSIC") => Some(MediaFormat::Music),
            Some("MANGA") => Some(MediaFormat::Manga),
            Some("NOVEL") => Some(MediaFormat::Novel),
            Some("ONE_SHOT") => Some(MediaFormat::OneShot),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MediaSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

impl MediaSeason {
    fn from(season: Option<&str>) -> Option<MediaSeason> {
        match season {
            Some("WINTER") => Some(MediaSeason::Winter),
            Some("SPRING") => Some(MediaSeason::Spring),
            Some("SUMMER") => Some(MediaSeason::Summer),
            Some("FALL") => Some(MediaSeason::Fall),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Media {
    pub media_id: Option<u64>,
    pub media_type: Option<MediaType>,
    pub status: Option<MediaStatus>,
    pub format: Option<MediaFormat>,
    pub season: Option<MediaSeason>,
    pub season_year: Option<u64>,
    pub title: Option<String>,
    pub alt_title: Option<String>,
    pub image: Option<String>,
    pub episodes: Option<u64>,
    pub score: Option<u64>,
    pub progress: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MediaLists {
    pub anime: Vec<Media>,
    pub manga: Vec<Media>,
}

#[cached]
fn get_config() -> Config {
    let file: File = File::open("config/anilist_api.yaml").expect("Could not open config.");
    let config: Config = serde_yaml::from_reader(&file).expect("Could not parse config.");
    config
}

fn transform(json: &serde_json::Value) -> Vec<Media> {
    let entries = json.as_array().unwrap();
    let mut list: Vec<Media> = Vec::new();

    for entry in entries {
        list.push(Media {
            media_id: entry["media"]["id"].as_u64(),
            media_type: MediaType::from(entry["media"]["type"].as_str()),
            status: MediaStatus::from(entry["media"]["status"].as_str()),
            format: MediaFormat::from(entry["media"]["format"].as_str()),
            season: MediaSeason::from(entry["media"]["season"].as_str()),
            season_year: entry["media"]["seasonYear"].as_u64(),
            title: entry["media"]["title"]["romaji"].as_str().map(String::from),
            alt_title: entry["media"]["title"]["english"]
                .as_str()
                .map(String::from),
            image: entry["media"]["coverImage"]["large"]
                .as_str()
                .map(String::from),
            episodes: entry["media"]["episodes"].as_u64(),
            score: entry["score"].as_u64(),
            progress: entry["progress"].as_u64(),
        });
    }

    list
}

async fn fetch(body: &serde_json::Value) -> Result<serde_json::Value, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let config = get_config();
    let url = config.url.as_str();
    let access_token = config.access_token.as_str();

    let json = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", access_token),
        )
        .json(body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(json)
}

async fn fetch_user() -> Result<serde_json::Value, Box<dyn Error>> {
    let body = serde_json::json!({
        "query": r#"query {
            Viewer {
                id
                name
            }
        }"#
    });

    let json = fetch(&body).await?;
    let user = json["data"]["Viewer"].to_owned();

    Ok(user)
}

async fn fetch_lists() -> Result<serde_json::Value, Box<dyn Error>> {
    let user = fetch_user().await?;
    let id = user["id"].as_u64().ok_or(AniListError {
        message: String::from("Could not get user ID."),
    })?;

    let body = serde_json::json!({
        "query": r#"query($userId: Int) {
            anime: MediaListCollection(userId: $userId, type: ANIME, status: CURRENT) {
                lists {
                    name
                    status
                    entries {
                        media {
                            id
                            type
                            format
                            season
                            seasonYear
                            title {
                                romaji
                                english
                            }
                            coverImage {
                                large
                            }
                            episodes
                        }
                        status
                        score
                        progress
                    }
                }
            }
            manga: MediaListCollection(userId: $userId, type: MANGA, status: CURRENT) {
                lists {
                    name
                    status
                    entries {
                        media {
                            id
                            type
                            format
                            season
                            seasonYear
                            title {
                                romaji
                                english
                            }
                            coverImage {
                                large
                            }
                            episodes
                        }
                        status
                        score
                        progress
                    }
                }
            }
        }"#,
        "variables": {
            "userId": id
        }
    });

    fetch(&body).await
}

pub async fn aggregate() -> Result<MediaLists, Box<dyn Error>> {
    let json = fetch_lists().await?;

    let anime = transform(&json["data"]["anime"]["lists"][0]["entries"]);
    let manga = transform(&json["data"]["manga"]["lists"][0]["entries"]);

    Ok(MediaLists { anime, manga })
}
