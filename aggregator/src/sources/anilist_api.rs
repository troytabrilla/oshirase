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

#[cached]
fn get_config() -> Config {
    let file: File = File::open("config/anilist_api.yaml").expect("Could not find config.");
    let config: Config = serde_yaml::from_reader(&file).expect("Could not parse config.");
    config
}

async fn fetch(body: &serde_json::Value) -> Result<serde_json::Value, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let config = get_config();
    let url = &config.url[..];
    let access_token = &config.access_token[..];

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

    fetch(&body).await
}

pub async fn fetch_lists() -> Result<serde_json::Value, Box<dyn Error>> {
    let user = fetch_user().await?;
    let id = user["data"]["Viewer"]["id"].as_u64().ok_or(AniListError {
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
