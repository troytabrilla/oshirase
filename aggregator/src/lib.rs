use std::{error::Error, fmt};
use tokio::try_join;

pub mod config;
mod db;
mod sources;

use anilist_api::*;
use config::Config;
use sources::*;
use subsplease_scraper::*;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct CustomError {
    message: String,
}

impl CustomError {
    pub fn new(message: &str) -> CustomError {
        CustomError {
            message: message.to_owned(),
        }
    }
}

impl CustomError {
    fn boxed(message: &str) -> Box<CustomError> {
        Box::new(CustomError::new(message))
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CustomError {}

#[derive(Debug)]
pub struct Data {
    lists: MediaLists,
    schedule: Vec<AnimeSchedule>,
}

pub struct Aggregator {
    anilist_api: AniListAPI,
    subsplease_scraper: SubsPleaseScraper,
    config: Config,
    data: Option<Data>,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let config = Config::default();
        let anilist_api = AniListAPI::default();
        let subsplease_scraper = SubsPleaseScraper::default();

        Aggregator {
            anilist_api,
            subsplease_scraper,
            config,
            data: None,
        }
    }
}

impl Aggregator {
    pub fn new(config: Config) -> Aggregator {
        let anilist_api = AniListAPI::new(&config);
        let subsplease_scraper = SubsPleaseScraper::new(&config.subsplease_scraper);

        Aggregator {
            anilist_api,
            subsplease_scraper,
            config,
            data: None,
        }
    }

    // @todo Add option to skip cache (pub struct ExtractOptions;)
    async fn extract(&mut self) -> Result<&mut Self> {
        let lists = self.anilist_api.extract().await?;
        let schedule = self.subsplease_scraper.extract().await?;

        self.data = Some(Data { lists, schedule });

        Ok(self)
    }

    async fn transform(&mut self) -> Result<&mut Self> {
        // @todo Combine data from sources into one result, i.e. update `latest` field
        Ok(self)
    }

    async fn load(&self) -> Result<&Self> {
        let mongodb = db::MongoDB::new(&self.config.db.mongodb);

        let lists = match &self.data {
            Some(data) => &data.lists,
            None => return Err(CustomError::boxed("No lists to persist.")),
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
        mongodb.client.database("test").drop(None).await.unwrap();

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
