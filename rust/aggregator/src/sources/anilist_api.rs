use crate::alt_titles_db::AltTitlesEntry;
use crate::config::Config;
use crate::error::CustomError;
use crate::result::Result;
use crate::sources::Document;
use crate::sources::{Extract, ExtractOptions};
use crate::subsplease_scraper::AnimeScheduleEntry;

use async_trait::async_trait;
use bson::doc;
use futures::TryStreamExt;
use graphql_client::GraphQLQuery;
use serde::{Deserialize, Serialize};

type Json = serde_json::Value;

#[derive(Debug, PartialEq, Deserialize, Serialize, Hash)]
pub struct User {
    pub id: u64,
    pub name: String,
}

impl Document for User {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash)]
pub struct Latest {
    pub title: String,
    pub episode: u64,
    pub url: String,
}

#[derive(Debug, Default, PartialEq, Deserialize, Serialize, Hash)]
pub struct Media {
    pub media_id: Option<u64>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub format: Option<String>,
    pub season: Option<String>,
    pub season_year: Option<u64>,
    pub title: Option<String>,
    pub english_title: Option<String>,
    pub image: Option<String>,
    pub episodes: Option<u64>,
    pub score: Option<u64>,
    pub progress: Option<u64>,
    pub schedule: Option<AnimeScheduleEntry>,
    pub latest: Option<Latest>,
    pub alt_titles: Option<AltTitlesEntry>,
}

impl Document for Media {}

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct MediaLists {
    pub anime: Vec<Media>,
    pub manga: Vec<Media>,
}

impl MediaLists {
    fn append(&mut self, other: &mut MediaLists) {
        self.anime.append(&mut other.anime);
        self.manga.append(&mut other.manga);
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist/schema.json",
    query_path = "graphql/anilist/list_query.graphql"
)]
struct AniListListQuery;

pub struct AniListAPI<'a> {
    config: &'a Config,
}

impl AniListAPI<'_> {
    pub fn new(config: &Config) -> AniListAPI {
        AniListAPI { config }
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
            .post(self.config.anilist_api.url.as_str())
            .json(&body)
            .send()
            .await?
            .json::<Json>()
            .await?;

        Ok(json)
    }

    pub async fn fetch_users(&self, mongodb_client: mongodb::Client) -> Result<Vec<User>> {
        let users: Vec<User> = mongodb_client
            .database(&self.config.db.mongodb.database)
            .collection("users")
            .find(None, None)
            .await?
            .try_collect()
            .await?;

        Ok(users)
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
                                    english_title: Self::extract_value_as_string(
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
                                    alt_titles: None,
                                };

                                acc.push(media);
                            }
                        }

                        acc
                    });

                Ok(list)
            }
            None => Err(CustomError::boxed("No response to transform.")),
        }
    }

    pub async fn fetch_lists(&self, user_id: u64) -> Result<MediaLists> {
        let variables = ani_list_list_query::Variables {
            user_id: Some(user_id as i64),
            status_in: Some(vec![
                Some(ani_list_list_query::MediaListStatus::CURRENT),
                Some(ani_list_list_query::MediaListStatus::PLANNING),
                Some(ani_list_list_query::MediaListStatus::COMPLETED),
                Some(ani_list_list_query::MediaListStatus::DROPPED),
                Some(ani_list_list_query::MediaListStatus::PAUSED),
                Some(ani_list_list_query::MediaListStatus::REPEATING),
            ]),
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
impl Extract<'_> for AniListAPI<'_> {
    type Data = MediaLists;

    async fn extract(&self, options: Option<ExtractOptions>) -> Result<Self::Data> {
        let mut data = Vec::new();

        let mongodb_client = match options {
            Some(options) => match options.mongodb_client {
                Some(mongodb_client) => mongodb_client,
                None => return Err(CustomError::boxed("No mongodb client provided.")),
            },
            None => return Err(CustomError::boxed("No options provided.")),
        };

        let users = self.fetch_users(mongodb_client).await?;
        for user in users {
            data.push(self.fetch_lists(user.id).await?);
        }

        let data = data
            .iter_mut()
            .reduce(|acc, d| {
                acc.append(d);
                acc
            })
            .ok_or(CustomError::boxed("Could not reduce lists."))?;

        Ok(std::mem::take(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::db::MongoDB;
    use crate::test::helpers::{init, reset_db, ONCE};

    #[tokio::test]
    async fn test_fetch_lists() {
        ONCE.get_or_init(init).await;
        reset_db().await;

        let config = Config::default();
        let mongodb = MongoDB::new(&config).await;
        let api = AniListAPI::new(&config);
        let users = api.fetch_users(mongodb.client).await.unwrap();
        let actual = api.fetch_lists(users[0].id).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_extract() {
        ONCE.get_or_init(init).await;
        reset_db().await;

        let config = Config::default();
        let mongodb = MongoDB::new(&config).await;
        let api = AniListAPI::new(&config);
        let options = ExtractOptions {
            mongodb_client: Some(mongodb.client),
        };
        let actual = api.extract(Some(options)).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }
}
