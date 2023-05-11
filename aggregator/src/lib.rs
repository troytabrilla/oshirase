pub mod config;
mod db;
mod sources;

use anilist_api::*;
use config::*;
use db::*;
use sources::*;
use subsplease_scraper::*;

use std::{error::Error, fmt, sync::Arc};
use tokio::{sync::Mutex, try_join};

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
    config: Config,
    data: Option<Data>,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let config = Config::default();

        Aggregator::new(&config)
    }
}

impl Aggregator {
    pub fn new(config: &Config) -> Aggregator {
        Aggregator {
            config: config.clone(),
            data: None,
        }
    }

    // @todo Add option to skip cache (pub struct ExtractOptions;)
    async fn extract(&mut self) -> Result<&mut Self> {
        let db = Arc::new(Mutex::new(DB::new(&self.config.db)));
        let mut anilist_api = AniListAPI::new(&self.config.anilist_api, db.clone());
        let mut subsplease_scraper =
            SubsPleaseScraper::new(&self.config.subsplease_scraper, db.clone());

        let lists = anilist_api.extract().await?;
        let schedule = subsplease_scraper.extract().await?;

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
        let database = mongodb.client.database("test");
        database
            .collection::<Media>("anime")
            .drop(None)
            .await
            .unwrap();
        database
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
