use crate::config::Config;
use crate::db::Document;
use crate::error::CustomError;
use crate::sources::Source;
use crate::subsplease_scraper::AnimeScheduleEntry;
use crate::Result;

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

#[derive(Debug, Default, PartialEq, Deserialize, Serialize, Hash)]
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
    mongodb: mongodb::Client,
}

impl AniListAPI<'_> {
    pub fn new(config: &Config, mongodb: mongodb::Client) -> AniListAPI {
        AniListAPI { config, mongodb }
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

    pub async fn fetch_user_or_default(&self, id: Option<u64>) -> Result<Vec<User>> {
        let filter = match id {
            Some(id) => {
                doc! { "id": id as i64 }
            }
            None => doc! {},
        };
        let users: Vec<User> = self
            .mongodb
            .database(&self.config.db.mongodb.database)
            .collection("users")
            .find(filter, None)
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
impl Source<'_> for AniListAPI<'_> {
    type Data = MediaLists;

    async fn extract(&mut self, id: Option<u64>) -> Result<Self::Data> {
        let mut data = Vec::new();

        let users = self.fetch_user_or_default(id).await?;
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
    use crate::db::DB;
    use crate::test::helpers::{init, Fixtures, ONCE};

    #[tokio::test]
    async fn test_fetch_lists() {
        ONCE.get_or_init(init).await;
        let config = Config::default();
        let fixtures = Fixtures::default();
        let db = DB::new(&config).await;
        let api = AniListAPI::new(&config, db.mongodb.client);
        let users = api
            .fetch_user_or_default(Some(fixtures.user.id))
            .await
            .unwrap();
        let actual = api.fetch_lists(users[0].id).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_extract() {
        ONCE.get_or_init(init).await;
        let config = Config::default();
        let db = DB::new(&config).await;
        let mut api = AniListAPI::new(&config, db.mongodb.client);
        let actual = api.extract(None).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }
}
