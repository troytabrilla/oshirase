use crate::config::AniListAPIConfig;
use crate::db::{Document, Redis};
use crate::sources::Source;
use crate::subsplease_scraper::AnimeScheduleEntry;
use crate::CustomError;
use crate::ExtractOptions;
use crate::Result;

use async_trait::async_trait;
use graphql_client::GraphQLQuery;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex;

type Json = serde_json::Value;

#[derive(PartialEq)]
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
    pub schedule: Option<AnimeScheduleEntry>,
}

impl Document for Media {}

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

pub struct AniListAPI {
    config: AniListAPIConfig,
    redis: Arc<Mutex<Redis>>,
}

impl AniListAPI {
    pub fn new(config: &AniListAPIConfig, redis: Arc<Mutex<Redis>>) -> AniListAPI {
        AniListAPI {
            config: config.clone(),
            redis,
        }
    }

    fn extract_value<'a>(json: &'a Json, key: &str) -> &'a Json {
        json.pointer(key).unwrap_or(&Json::Null)
    }

    fn extract_value_as_array<'a>(json: &'a Json, key: &str) -> Option<&'a Vec<Json>> {
        Self::extract_value(json, key).as_array()
    }

    fn extract_value_as_u64(json: &Json, key: &str) -> Option<u64> {
        Self::extract_value(json, key).as_u64()
    }

    fn extract_value_as_string(json: &Json, key: &str) -> Option<String> {
        Self::extract_value(json, key)
            .as_str()
            .map(ToOwned::to_owned)
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
            id: match Self::extract_value_as_u64(&json, "/data/Viewer/id") {
                Some(id) => id,
                None => return Err(CustomError::boxed("Could not find user ID.")),
            },
            name: match Self::extract_value_as_string(&json, "/data/Viewer/name") {
                Some(name) => name,
                None => return Err(CustomError::boxed("Could not find user name.")),
            },
        })
    }

    fn transform(&self, json: Option<&Vec<Json>>) -> Result<Vec<Media>> {
        match json {
            Some(json) => {
                let list: Vec<Media> =
                    json.iter().fold(Vec::new() as Vec<Media>, |mut acc, list| {
                        if let Some(entries) = Self::extract_value_as_array(list, "/entries") {
                            for entry in entries {
                                let media = Media {
                                    media_id: Self::extract_value_as_u64(entry, "/media/id"),
                                    media_type: Self::extract_value_as_string(entry, "/media/type"),
                                    status: Self::extract_value_as_string(entry, "/status"),
                                    format: Self::extract_value_as_string(entry, "/media/format"),
                                    season: Self::extract_value_as_string(entry, "/media/season"),
                                    season_year: Self::extract_value_as_u64(
                                        entry,
                                        "/media/seasonYear",
                                    ),
                                    title: Self::extract_value_as_string(
                                        entry,
                                        "/media/title/romaji",
                                    ),
                                    alt_title: Self::extract_value_as_string(
                                        entry,
                                        "/media/title/english",
                                    ),
                                    image: Self::extract_value_as_string(
                                        entry,
                                        "/media/coverImage/large",
                                    ),
                                    episodes: Self::extract_value_as_u64(entry, "/media/episodes"),
                                    score: Self::extract_value_as_u64(entry, "/score"),
                                    progress: Self::extract_value_as_u64(entry, "/progress"),
                                    latest: None,
                                    schedule: None,
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

        let anime = Self::extract_value_as_array(&json, "/data/anime/lists");
        let anime = self.transform(anime)?;

        let manga = Self::extract_value_as_array(&json, "/data/manga/lists");
        let manga = self.transform(manga)?;

        let lists = MediaLists { anime, manga };

        Ok(lists)
    }
}

#[async_trait]
impl Source for AniListAPI {
    type Data = MediaLists;

    async fn extract(&mut self, options: Option<&ExtractOptions>) -> Result<MediaLists> {
        let user = self.fetch_user().await?;
        let cache_key = format!("anilist_api:fetch_lists:{}", user.id);

        let dont_cache = match options {
            Some(options) => options.dont_cache,
            None => false,
        };

        if !dont_cache {
            if let Some(cached) = self.get_cached(&cache_key).await {
                return Ok(cached);
            }
        }

        let lists = self.fetch_lists(user.id).await?;

        if !dont_cache {
            self.cache_value(&cache_key, &lists).await;
        }

        Ok(lists)
    }

    async fn get_cached(&mut self, key: &str) -> Option<MediaLists> {
        let redis = &mut self.redis.lock().await;

        redis.get_cached(key).await
    }

    async fn cache_value(&mut self, key: &str, lists: &MediaLists) {
        let redis = &mut self.redis.lock().await;

        redis.cache_value_ex(key, lists, 600).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[tokio::test]
    async fn test_anilist_api_new() {
        let config = Config::default();
        let redis = Arc::new(Mutex::new(Redis::new(&config.db.redis).await));
        let api = AniListAPI::new(
            &AniListAPIConfig {
                url: "url".to_owned(),
                auth: AniListAPIAuthConfig {
                    access_token: "access_token".to_owned(),
                },
            },
            redis,
        );
        assert_eq!(api.config.url, "url");
        assert_eq!(api.config.auth.access_token, "access_token");
    }

    #[tokio::test]
    async fn test_anilist_api_fetch_user() {
        let config = Config::default();
        let redis = Arc::new(Mutex::new(Redis::new(&config.db.redis).await));
        let api = AniListAPI::new(&config.anilist_api, redis);
        let actual = api.fetch_user().await.unwrap();
        assert!(!actual.name.is_empty());
    }

    #[tokio::test]
    async fn test_anilist_api_fetch_lists() {
        let config = Config::default();
        let redis = Arc::new(Mutex::new(Redis::new(&config.db.redis).await));
        let api = AniListAPI::new(&config.anilist_api, redis);
        let user = api.fetch_user().await.unwrap();
        let actual = api.fetch_lists(user.id).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_anilist_api_extract() {
        let config = Config::default();
        let redis = Arc::new(Mutex::new(Redis::new(&config.db.redis).await));
        let mut api = AniListAPI::new(&config.anilist_api, redis);
        let options = ExtractOptions { dont_cache: true };
        let actual = api.extract(Some(&options)).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }
}
