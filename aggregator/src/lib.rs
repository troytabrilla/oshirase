mod config;
mod db;
mod error;
mod sources;
mod transform;
mod worker;

pub use config::Config;
pub use error::CustomError;
pub use worker::Worker;

use anilist_api::*;
use db::*;
use sources::*;
use subsplease_scraper::*;
use transform::*;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

// @todo Allocate data on the heap instead of the stack
#[derive(Debug, Deserialize, Serialize)]
pub struct Data {
    lists: MediaLists,
    schedule: AnimeSchedule,
}

pub struct Sources<'a> {
    anilist_api: AniListAPI<'a>,
    subsplease_scraper: SubsPleaseScraper<'a>,
}

pub struct Aggregator<'a> {
    config: &'a Config,
    db: DB<'a>,
}

impl<'a> Aggregator<'a> {
    pub async fn new(config: &'a Config) -> Aggregator<'a> {
        Aggregator {
            config,
            db: DB::new(config).await,
        }
    }

    async fn extract(&mut self, sources: &mut Sources<'a>) -> Result<Data> {
        let (lists, schedule) = tokio::join!(
            sources.anilist_api.extract(),
            sources.subsplease_scraper.extract()
        );

        let lists = lists?;
        let schedule = schedule?;

        Ok(Data { lists, schedule })
    }

    fn transform(&mut self, sources: Sources<'a>, mut data: Data) -> Result<Data> {
        let anime = data
            .lists
            .anime
            .par_iter()
            .map(|anime| {
                match sources
                    .subsplease_scraper
                    .transform(anime.clone(), &data.schedule.0)
                {
                    Ok(anime) => anime,
                    Err(err) => {
                        eprintln!("Could not transform media: {}", err);
                        anime.clone()
                    }
                }
            })
            .collect();
        data.lists.anime = anime;

        Ok(data)
    }

    async fn load(&self, data: &Data) -> Result<()> {
        let anime_future = self.upsert_documents("anime", "media_id", &data.lists.anime);
        let manga_future = self.upsert_documents("manga", "media_id", &data.lists.manga);

        tokio::try_join!(anime_future, manga_future)?;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<Data> {
        let mut sources = Sources {
            anilist_api: AniListAPI::new(self.config),
            subsplease_scraper: SubsPleaseScraper::new(self.config),
        };

        let data = self.extract(&mut sources).await?;
        let data = self.transform(sources, data)?;
        self.load(&data).await?;

        Ok(data)
    }
}

impl Persist for Aggregator<'_> {
    fn get_client(&self) -> &mongodb::Client {
        &self.db.mongodb.client
    }

    fn get_database(&self) -> &str {
        self.config.db.mongodb.database.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_run() {
        let config = Config::default();
        let mongodb = MongoDB::new(&config).await;
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

        let redis_client = Redis::new(&config).client;
        let mut connection = redis_client.get_connection().unwrap();
        redis::cmd("FLUSHALL").query::<()>(&mut connection).unwrap();

        let mut aggregator = Aggregator::new(&config).await;
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
