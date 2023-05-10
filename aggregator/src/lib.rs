use std::{error::Error, fmt};
use tokio::try_join;

pub mod config;
mod db;
mod sources;

use anilist_api::*;
use config::Config;
use sources::*;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct AggregatorError {
    message: String,
}

impl AggregatorError {
    fn boxed(message: &str) -> Box<AggregatorError> {
        Box::new(AggregatorError {
            message: message.to_owned(),
        })
    }
}

impl fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AggregatorError {}

#[derive(Debug)]
pub struct Data {
    lists: MediaLists,
}

pub struct Aggregator {
    anilist_api: AniListAPI,
    config: Config,
    data: Option<Data>,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let config = Config::default();
        let anilist_api = AniListAPI::default();

        Aggregator {
            anilist_api,
            config,
            data: None,
        }
    }
}

impl Aggregator {
    pub fn new(config: Config) -> Aggregator {
        let anilist_api = AniListAPI::new(config.clone());

        Aggregator {
            anilist_api,
            config,
            data: None,
        }
    }

    async fn extract(&mut self) -> Result<&mut Self> {
        let lists = self.anilist_api.extract().await?;
        self.data = Some(Data { lists });

        Ok(self)
    }

    async fn transform(&mut self) -> Result<&mut Self> {
        // @todo Combine data from sources into one result, i.e. update `latest` field
        Ok(self)
    }

    async fn load(&self) -> Result<&Self> {
        let mongodb = db::MongoDB::new(self.config.db.mongodb.clone());

        let lists = match &self.data {
            Some(data) => &data.lists,
            None => return Err(AggregatorError::boxed("No lists to persist.")),
        };

        let anime_future = mongodb.upsert_documents("anime", &lists.anime);
        let manga_future = mongodb.upsert_documents("manga", &lists.manga);

        try_join!(anime_future, manga_future)?;

        Ok(self)
    }

    pub async fn run(&mut self) -> Result<()> {
        self.extract().await?.transform().await?.load().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_aggregator_run() {
        let mongodb = db::MongoDB::default();
        mongodb
            .client
            .database("test")
            .collection::<Media>("anime")
            .drop(None)
            .await
            .unwrap();
        mongodb
            .client
            .database("test")
            .collection::<Media>("manga")
            .drop(None)
            .await
            .unwrap();

        let mut aggregator = Aggregator::default();
        aggregator.run().await.unwrap();

        let anime: bson::Document = mongodb
            .client
            .database("test")
            .collection("anime")
            .find_one(doc! { "media_id": 918 }, None)
            .await
            .unwrap()
            .unwrap();

        let manga: bson::Document = mongodb
            .client
            .database("test")
            .collection("manga")
            .find_one(doc! { "media_id": 30044 }, None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(anime.get("title").unwrap().as_str().unwrap(), "Gintama");
        assert_eq!(manga.get("title").unwrap().as_str().unwrap(), "Gintama");
    }
}
