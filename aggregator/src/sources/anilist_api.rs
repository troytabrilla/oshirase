use crate::config::AniListAPIConfig;
use crate::db::DB;
use crate::sources::Source;
use crate::CustomError;
use crate::Result;

use async_trait::async_trait;
use graphql_client::GraphQLQuery;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex;

type Json = serde_json::Value;

#[derive(Debug, PartialEq)]
pub struct User {
    id: u64,
    name: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Hash)]
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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
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
    config: AniListAPIConfig,
    db: Arc<Mutex<DB>>,
}

impl AniListAPI {
    pub fn new(config: &AniListAPIConfig, db: Arc<Mutex<DB>>) -> AniListAPI {
        AniListAPI {
            config: config.clone(),
            db,
        }
    }

    fn extract_value<'b>(json: &'b Json, key: &str) -> &'b Json {
        json.pointer(key).unwrap_or(&Json::Null)
    }

    async fn fetch<T>(&self, body: &T) -> Result<Json>
    where
        T: Serialize,
    {
        let client = reqwest::Client::new();
        let json = client
            .post(self.config.url.as_str())
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.config.auth.access_token),
            )
            .json(&body)
            .send()
            .await?
            .json::<Json>()
            .await?;

        Ok(json)
    }

    pub async fn fetch_user(&self) -> Result<User> {
        let variables = ani_list_user_query::Variables {};
        let body = AniListUserQuery::build_query(variables);

        let json = self.fetch(&body).await?;

        Ok(User {
            id: match Self::extract_value(&json, "/data/Viewer/id").as_u64() {
                Some(id) => id,
                None => return Err(CustomError::boxed("Could not find user ID.")),
            },
            name: match Self::extract_value(&json, "/data/Viewer/name").as_str() {
                Some(name) => name.to_owned(),
                None => return Err(CustomError::boxed("Could not find user name.")),
            },
        })
    }

    fn transform(&self, json: Option<&Vec<Json>>) -> Result<Vec<Media>> {
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
            None => Err(Box::new(CustomError {
                message: "No response to transform.".to_owned(),
            })),
        }
    }

    pub async fn fetch_lists(&self, user_id: u64) -> Result<MediaLists> {
        let variables = ani_list_list_query::Variables {
            user_id: Some(user_id as i64),
        };
        let body = AniListListQuery::build_query(variables);

        let json = self.fetch(&body).await?;

        let anime = Self::extract_value(&json, "/data/anime/lists").as_array();
        let anime = self.transform(anime)?;

        let manga = Self::extract_value(&json, "/data/manga/lists").as_array();
        let manga = self.transform(manga)?;

        let lists = MediaLists { anime, manga };

        Ok(lists)
    }

    fn get_cache_key(user_id: u64) -> String {
        format!("anilist_api:fetch_lists:{}", user_id)
    }

    async fn check_cache(&mut self, user: &User) -> Result<MediaLists> {
        let redis = &mut self.db.lock().await.redis;
        let key = Self::get_cache_key(user.id);

        let cached: MediaLists = redis.check_cache(&key).await?;

        Ok(cached)
    }

    async fn cache_value(&mut self, user: &User, lists: &MediaLists) {
        let redis = &mut self.db.lock().await.redis;
        let key = Self::get_cache_key(user.id);

        redis.cache_value_ex(&key, lists, 600).await;
    }
}

#[async_trait]
impl Source for AniListAPI {
    type Data = MediaLists;

    async fn extract(&mut self) -> Result<MediaLists> {
        let user = self.fetch_user().await?;

        // @todo Add option to skip cache
        match self.check_cache(&user).await {
            Ok(cached) => return Ok(cached),
            Err(err) => {
                println!("Could not get cached response: {}", err);
            }
        }

        let lists = self.fetch_lists(user.id).await?;
        self.cache_value(&user, &lists).await;

        Ok(lists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[tokio::test]
    async fn test_anilist_api_new() {
        let db = Arc::new(Mutex::new(DB::default()));
        let api = AniListAPI::new(
            &AniListAPIConfig {
                url: "url".to_owned(),
                auth: AniListAPIAuthConfig {
                    access_token: "access_token".to_owned(),
                },
            },
            db,
        );
        assert_eq!(api.config.url, "url");
        assert_eq!(api.config.auth.access_token, "access_token");
    }

    #[tokio::test]
    async fn test_anilist_api_fetch_user() {
        let config = Config::default();
        let db = Arc::new(Mutex::new(DB::default()));
        let api = AniListAPI::new(&config.anilist_api, db);
        let actual = api.fetch_user().await.unwrap();
        assert!(!actual.name.is_empty());
    }

    #[tokio::test]
    async fn test_anilist_api_fetch_lists() {
        let config = Config::default();
        let db = Arc::new(Mutex::new(DB::default()));
        let api = AniListAPI::new(&config.anilist_api, db);
        let user = api.fetch_user().await.unwrap();
        let actual = api.fetch_lists(user.id).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_anilist_api_extract() {
        let config = Config::default();
        let db = Arc::new(Mutex::new(DB::default()));
        let mut api = AniListAPI::new(&config.anilist_api, db);
        let actual = api.extract().await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }
}
