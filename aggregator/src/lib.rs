pub mod config;
mod db;
mod merge;
mod sources;

use anilist_api::*;
use config::*;
use db::*;
use merge::*;
use sources::*;
use subsplease_scraper::*;

use std::{error::Error, fmt};
use tokio::try_join;

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

#[derive(Clone)]
pub struct ExtractOptions {
    pub dont_cache: bool,
}

pub struct RunOptions {
    pub extract_options: Option<ExtractOptions>,
}

#[derive(Debug)]
pub struct Data {
    lists: MediaLists,
    schedule: AnimeSchedule,
}

pub struct Aggregator {
    config: Config,
    db: DB,
}

impl Aggregator {
    pub async fn new(config: &Config) -> Aggregator {
        let db = DB::new(&config.db).await;

        Aggregator {
            config: config.clone(),
            db,
        }
    }

    async fn extract(&mut self, options: Option<&ExtractOptions>) -> Result<Data> {
        let mut anilist_api = AniListAPI::new(&self.config.anilist_api, self.db.redis.clone());
        let mut subsplease_scraper =
            SubsPleaseScraper::new(&self.config.subsplease_scraper, self.db.redis.clone());

        let lists = anilist_api.extract(options).await?;
        let schedule = subsplease_scraper.extract(options).await?;

        Ok(Data { lists, schedule })
    }

    async fn transform(&mut self, mut data: Data) -> Result<Data> {
        // @todo Combine data from sources into one result, i.e. update `latest` field, add schedule, etc
        let anime = &mut data.lists.anime;
        let schedule = &data.schedule.0;

        let anime = Merge::merge(anime, schedule)?;

        data.lists.anime = anime.to_vec();

        Ok(data)
    }

    async fn load(&self, data: Data) -> Result<()> {
        let mongodb = &self.db.mongodb.lock().await;

        let anime_future = mongodb.upsert_documents("anime", &data.lists.anime);
        let manga_future = mongodb.upsert_documents("manga", &data.lists.manga);

        try_join!(anime_future, manga_future)?;

        Ok(())
    }

    pub async fn run(&mut self, options: Option<RunOptions>) -> Result<()> {
        let extract_options = match options {
            Some(options) => options.extract_options,
            None => None,
        };

        let data = self.extract(extract_options.as_ref()).await?;
        let data = self.transform(data).await?;
        self.load(data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_aggregator_run() {
        let config = Config::default();
        let mongodb = MongoDB::new(&config.db.mongodb);
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

        let mut aggregator = Aggregator::new(&config).await;
        let options = RunOptions {
            extract_options: Some(ExtractOptions { dont_cache: true }),
        };
        aggregator.run(Some(options)).await.unwrap();

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
