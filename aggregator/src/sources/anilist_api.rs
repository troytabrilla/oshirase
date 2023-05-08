use crate::config::Config;
use crate::sources::Source;

use async_trait::async_trait;
use graphql_client::GraphQLQuery;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{error::Error, fmt};

type Json = serde_json::Value;

#[derive(Debug)]
struct AniListError {
    message: String,
}

impl AniListError {
    fn boxed(message: &str) -> Box<AniListError> {
        Box::new(AniListError {
            message: message.to_owned(),
        })
    }
}

impl fmt::Display for AniListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AniListError {}

#[derive(Debug, PartialEq)]
pub struct User {
    id: u64,
    name: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Media {
    pub media_id: Option<u64>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub format: Option<String>,
    pub season: Option<String>,
    pub season_year: Option<u64>,
    pub title: Option<String>,
    pub alt_title: Option<String>,
    pub image: Option<String>,
    pub episodes: Option<u64>,
    pub score: Option<u64>,
    pub progress: Option<u64>,
    pub latest: Option<u64>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct MediaLists {
    pub anime: Vec<Media>,
    pub manga: Vec<Media>,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist/schema.json",
    query_path = "graphql/anilist/user_query.graphql"
)]
struct AniListUserQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist/schema.json",
    query_path = "graphql/anilist/list_query.graphql"
)]
struct AniListListQuery;

#[derive(Debug)]
pub struct AniListAPI {
    config: Config,
}

impl AniListAPI {
    pub fn new(config: Config) -> AniListAPI {
        AniListAPI { config }
    }

    fn extract_value<'a>(json: &'a Json, key: &str) -> &'a Json {
        json.pointer(key).unwrap_or(&Json::Null)
    }

    async fn fetch<T>(&self, body: &T) -> Result<Json, Box<dyn Error>>
    where
        T: Serialize,
    {
        let client = reqwest::Client::new();
        let json = client
            .post(self.config.anilist_api.url.as_str())
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.config.anilist_api.auth.access_token),
            )
            .json(&body)
            .send()
            .await?
            .json::<Json>()
            .await?;

        Ok(json)
    }

    pub async fn fetch_user(&self) -> Result<User, Box<dyn Error>> {
        let variables = ani_list_user_query::Variables {};
        let body = AniListUserQuery::build_query(variables);

        let json = self.fetch(&body).await?;

        Ok(User {
            id: match Self::extract_value(&json, "/data/Viewer/id").as_u64() {
                Some(id) => id,
                None => return Err(AniListError::boxed("Could not find user ID.")),
            },
            name: match Self::extract_value(&json, "/data/Viewer/name").as_str() {
                Some(name) => name.to_owned(),
                None => return Err(AniListError::boxed("Could not find user name.")),
            },
        })
    }

    fn transform(&self, json: Option<&Vec<Json>>) -> Result<Vec<Media>, Box<dyn Error>> {
        match json {
            Some(json) => {
                let list: Vec<Media> =
                    json.iter().fold(Vec::new() as Vec<Media>, |mut acc, list| {
                        if let Some(entries) = Self::extract_value(list, "/entries").as_array() {
                            for entry in entries {
                                let media = Media {
                                    media_id: Self::extract_value(entry, "/media/id").as_u64(),
                                    media_type: Self::extract_value(entry, "/media/type")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    status: Self::extract_value(entry, "/status")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    format: Self::extract_value(entry, "/media/format")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    season: Self::extract_value(entry, "/media/season")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    season_year: Self::extract_value(entry, "/media/seasonYear")
                                        .as_u64(),
                                    title: Self::extract_value(entry, "/media/title/romaji")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    alt_title: Self::extract_value(entry, "/media/title/english")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    image: Self::extract_value(entry, "/media/coverImage/large")
                                        .as_str()
                                        .map(ToOwned::to_owned),
                                    episodes: Self::extract_value(entry, "/media/episodes")
                                        .as_u64(),
                                    score: Self::extract_value(entry, "/score").as_u64(),
                                    progress: Self::extract_value(entry, "/progress").as_u64(),
                                    latest: None,
                                };

                                acc.push(media);
                            }
                        }

                        acc
                    });

                Ok(list)
            }
            None => Err(Box::new(AniListError {
                message: "No response to transform.".to_owned(),
            })),
        }
    }

    pub async fn fetch_lists(&self, user_id: u64) -> Result<MediaLists, Box<dyn Error>> {
        let variables = ani_list_list_query::Variables {
            user_id: Some(user_id as i64),
        };
        let body = AniListListQuery::build_query(variables);

        let json = self.fetch(&body).await?;

        let anime = Self::extract_value(&json, "/data/anime/lists").as_array();
        let anime = self.transform(anime)?;

        let manga = Self::extract_value(&json, "/data/manga/lists").as_array();
        let manga = self.transform(manga)?;

        Ok(MediaLists { anime, manga })
    }
}

impl Default for AniListAPI {
    fn default() -> AniListAPI {
        let config = Config::default();
        AniListAPI::new(config)
    }
}

#[async_trait]
impl Source for AniListAPI {
    type Data = MediaLists;

    async fn extract(&self) -> Result<MediaLists, Box<dyn Error>> {
        let user = self.fetch_user().await.unwrap();
        let lists = self.fetch_lists(user.id).await.unwrap();

        Ok(lists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn test_new() {
        let api = AniListAPI::new(Config {
            anilist_api: AniListAPIConfig {
                url: "url".to_owned(),
                auth: AniListAPIAuthConfig {
                    access_token: "access_token".to_owned(),
                },
            },
            db: DBConfig {
                mongodb: MongoDBConfig {
                    host: "host".to_owned(),
                },
            },
        });
        assert_eq!(api.config.anilist_api.url, "url");
        assert_eq!(api.config.anilist_api.auth.access_token, "access_token");
        assert_eq!(api.config.db.mongodb.host, "host");
    }

    #[test]
    fn test_default() {
        let api = AniListAPI::default();
        assert_eq!(api.config.anilist_api.url, "https://graphql.anilist.co");
    }

    #[tokio::test]
    async fn test_fetch_user() {
        let api = AniListAPI::default();
        let actual = api.fetch_user().await.unwrap();
        assert_eq!(actual.name, "***REMOVED***");
    }

    #[tokio::test]
    async fn test_fetch_lists() {
        let api = AniListAPI::default();
        let user = api.fetch_user().await.unwrap();
        let actual = api.fetch_lists(user.id).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_extract() {
        let api = AniListAPI::default();
        let actual = api.extract().await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }
}
