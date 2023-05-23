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

#[derive(Debug, Default, PartialEq, Deserialize, Serialize, Hash)]
pub enum MediaType {
    #[default]
    Anime,
    Manga,
}

impl MediaType {
    pub fn from_str(s: &str) -> Result<MediaType> {
        let s = s.to_lowercase();
        if s == "anime" {
            Ok(MediaType::Anime)
        } else if s.to_lowercase() == "manga" {
            Ok(MediaType::Manga)
        } else {
            Err(CustomError::boxed(&format!("Invalid media type: {s}.")))
        }
    }

    pub fn from_option_str(s: Option<&str>) -> Option<MediaType> {
        match s {
            Some(s) => Self::from_str(s).ok(),
            None => None,
        }
    }
}

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
    pub media_type: Option<MediaType>,
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

#[derive(Debug, Deserialize)]
struct CoverImage {
    large: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaTitle {
    romaji: Option<String>,
    english: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResultMedia {
    id: Option<u64>,
    r#type: Option<String>,
    format: Option<String>,
    season: Option<String>,
    #[serde(rename = "seasonYear")]
    season_year: Option<u64>,
    title: MediaTitle,
    #[serde(rename = "coverImage")]
    cover_image: CoverImage,
    episodes: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    media: ResultMedia,
    status: Option<String>,
    score: Option<u64>,
    progress: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MediaList {
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct MediaListCollection {
    lists: Vec<MediaList>,
}

#[derive(Debug, Deserialize)]
struct AniListListQueryData {
    anime: MediaListCollection,
    manga: MediaListCollection,
}

#[derive(Debug, Deserialize)]
struct AniListListQueryResults {
    data: AniListListQueryData,
}

pub struct AniListAPI<'a> {
    config: &'a Config,
}

impl AniListAPI<'_> {
    pub fn new(config: &Config) -> AniListAPI {
        AniListAPI { config }
    }

    async fn fetch<T>(&self, body: &T) -> Result<AniListListQueryResults>
    where
        T: Serialize,
    {
        let client = reqwest::Client::new();
        let results = client
            .post(self.config.anilist_api.url.as_str())
            .json(&body)
            .send()
            .await?
            .json::<AniListListQueryResults>()
            .await?;

        Ok(results)
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

    fn transform(&self, lists: &[MediaList]) -> Result<Vec<Media>> {
        let list = lists
            .iter()
            .fold(Vec::new() as Vec<Media>, |mut acc, list| {
                for entry in &list.entries {
                    let media = Media {
                        media_id: entry.media.id,
                        media_type: MediaType::from_option_str(
                            entry.media.r#type.clone().as_deref(),
                        ),
                        status: entry.status.clone(),
                        format: entry.media.format.clone(),
                        season: entry.media.season.clone(),
                        season_year: entry.media.season_year,
                        title: entry.media.title.romaji.clone(),
                        english_title: entry.media.title.english.clone(),
                        image: entry.media.cover_image.large.clone(),
                        episodes: entry.media.episodes,
                        score: entry.score,
                        progress: entry.progress,
                        schedule: None,
                        latest: None,
                        alt_titles: None,
                    };

                    acc.push(media);
                }

                acc
            });

        Ok(list)
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

        let results = self.fetch(&body).await?;

        let anime = self.transform(&results.data.anime.lists)?;

        let manga = self.transform(&results.data.manga.lists)?;

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
