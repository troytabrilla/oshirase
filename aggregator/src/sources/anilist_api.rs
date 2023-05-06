use crate::config::Conf;
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
    fn from(message: &str) -> Box<AniListError> {
        Box::new(AniListError {
            message: String::from(message),
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
    schema_path = "graphql/anilist_schema.json",
    query_path = "graphql/anilist_user_query.graphql"
)]
pub struct AniListUserQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist_schema.json",
    query_path = "graphql/anilist_list_query.graphql"
)]
pub struct AniListListQuery;

#[derive(Debug)]
pub struct AniListAPI {
    config: Config,
}

impl AniListAPI {
    pub fn new(config: Config) -> AniListAPI {
        AniListAPI { config }
    }

    pub fn from(filename: &str) -> AniListAPI {
        let config = Self::get_config(filename).expect("Could not load config.");
        Self::new(config)
    }

    pub async fn fetch_user(&self, access_token: Option<&str>) -> Result<User, Box<dyn Error>> {
        let variables = ani_list_user_query::Variables {};
        let body = AniListUserQuery::build_query(variables);
        let access_token = access_token.unwrap_or(&self.config.access_token);
        let client = reqwest::Client::new();

        let json = client
            .post(self.config.url.as_str())
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", access_token),
            )
            .json(&body)
            .send()
            .await?
            .json::<Json>()
            .await?;

        let err = AniListError::from("Could not fetch user.");

        Ok(User {
            id: match json.pointer("/data/Viewer/id") {
                Some(id) => match id.as_u64() {
                    Some(id) => id,
                    None => return Err(err),
                },
                None => return Err(err),
            },
            name: match json.pointer("/data/Viewer/name") {
                Some(name) => match name.as_str() {
                    Some(name) => String::from(name),
                    None => return Err(err),
                },
                None => return Err(err),
            },
        })
    }

    fn transform(&self, json: &Json) -> Result<Vec<Media>, Box<dyn Error>> {
        Err(Box::new(AniListError {
            message: String::from("Could not transform response."),
        }))
    }

    // @todo Use graphql_client
    pub async fn fetch_lists(
        &self,
        user_id: u64,
        status: Option<&str>,
    ) -> Result<MediaLists, Box<dyn Error>> {
        let client = reqwest::Client::new();
        Err(Box::new(AniListError {
            message: String::from("No lists found."),
        }))
    }
}

#[async_trait]
impl Source for AniListAPI {
    type Data = MediaLists;

    // @todo implement this properly
    async fn aggregate(&self) -> Result<MediaLists, Box<dyn Error>> {
        println!("{:#?}", self.config);
        Err(Box::new(AniListError {
            message: String::from("No lists available."),
        }))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    url: String,
    access_token: String,
}

impl Conf for AniListAPI {
    type Config = Config;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let api = AniListAPI::new(Config {
            url: String::from("url"),
            access_token: String::from("access_token"),
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
        let actual = api.fetch_user(None).await.unwrap();
        assert_eq!(actual.name, "***REMOVED***");
    }

    #[tokio::test]
    async fn test_fetch_lists() {
        let api = AniListAPI::from("config/anilist_api.yaml");
        let user = api.fetch_user(None).await.unwrap();
        let actual = api.fetch_lists(user.id, None).await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }

    #[tokio::test]
    async fn test_aggregate() {
        let api = AniListAPI::from("config/anilist_api.yaml");
        let actual = api.aggregate().await.unwrap();
        assert!(!actual.anime.is_empty());
        assert!(!actual.manga.is_empty());
    }
}
