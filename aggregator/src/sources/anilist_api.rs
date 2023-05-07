use crate::config::Conf;
use crate::sources::Source;

use async_trait::async_trait;
use graphql_client::GraphQLQuery;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{error::Error, fmt};

type Json = serde_json::Value;
type Object = serde_json::Map<String, Json>;

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
struct Media {
    media_id: Option<u64>,
    media_type: Option<String>,
    status: Option<String>,
    format: Option<String>,
    season: Option<String>,
    season_year: Option<u64>,
    title: Option<String>,
    alt_title: Option<String>,
    image: Option<String>,
    episodes: Option<u64>,
    score: Option<u64>,
    progress: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct User {
    id: u64,
    name: String,
}

#[derive(Debug, PartialEq)]
pub struct MediaLists {
    anime: Vec<Media>,
    manga: Vec<Media>,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist/schema.json",
    query_path = "graphql/anilist/user_query.graphql"
)]
pub struct AniListUserQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist/schema.json",
    query_path = "graphql/anilist/list_query.graphql"
)]
pub struct AniListListQuery;

type MediaListStatus = ani_list_list_query::MediaListStatus;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    url: String,
    access_token: String,
}

#[derive(Debug)]
pub struct AniListAPI {
    config: Config,
}

impl Conf for AniListAPI {
    type Config = Config;
}

impl AniListAPI {
    pub fn new(config: Config) -> AniListAPI {
        AniListAPI { config }
    }

    pub fn from(filename: &str) -> AniListAPI {
        let config = Self::get_config(filename).expect("Could not load anilist_api config.");
        Self::new(config)
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
            .post(self.config.url.as_str())
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.config.access_token),
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

    fn transform(&self, json: Option<&Object>) -> Result<Vec<Media>, Box<dyn Error>> {
        match json {
            Some(json) => Err(Box::new(AniListError {
                message: "Could not transform response.".to_owned(),
            })),
            None => Err(Box::new(AniListError {
                message: "No response to transform.".to_owned(),
            })),
        }
    }

    pub async fn fetch_lists(
        &self,
        user_id: u64,
        status: Option<MediaListStatus>,
    ) -> Result<MediaLists, Box<dyn Error>> {
        let variables = ani_list_list_query::Variables {
            user_id: Some(user_id as i64),
            status: match status {
                Some(status) => Some(status),
                None => Some(MediaListStatus::CURRENT),
            },
        };
        let body = AniListListQuery::build_query(variables);

        let json = self.fetch(&body).await?;

        let anime = Self::extract_value(&json, "/data/anime").as_object();
        let anime = self.transform(anime)?;

        let manga = Self::extract_value(&json, "/data/manga").as_object();
        let manga = self.transform(manga)?;

        Ok(MediaLists { anime, manga })
    }
}

#[async_trait]
impl Source for AniListAPI {
    // @todo implement this properly
    // @todo save to mongodb
    // @todo set up docker
    async fn aggregate(&self) -> Result<(), Box<dyn Error>> {
        Err(Box::new(AniListError {
            message: "No lists available.".to_owned(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let api = AniListAPI::new(Config {
            url: "url".to_owned(),
            access_token: "access_token".to_owned(),
        });
        assert_eq!(api.config.url, "url");
        assert_eq!(api.config.access_token, "access_token");
    }

    #[test]
    fn test_from() {
        let api = AniListAPI::from("config/anilist_api.yaml");
        assert_eq!(api.config.url, "https://graphql.anilist.co");
    }

    #[test]
    #[should_panic]
    fn test_from_failure() {
        AniListAPI::from("config/should_fail.yaml");
    }

    #[tokio::test]
    async fn test_fetch_user() {
        let api = AniListAPI::from("config/anilist_api.yaml");
        let actual = api.fetch_user().await.unwrap();
        assert_eq!(actual.name, "***REMOVED***");
    }

    #[tokio::test]
    async fn test_fetch_lists() {
        let api = AniListAPI::from("config/anilist_api.yaml");
        let user = api.fetch_user().await.unwrap();
        let actual = api.fetch_lists(user.id, None).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_aggregate() {
        let api = AniListAPI::from("config/anilist_api.yaml");
        api.aggregate().await.unwrap();
        panic!("Check DB.");
    }
}
