mod config;
mod db;
mod emitter;
mod sources;

pub use config::Config;

use anilist_api::*;
use db::*;
use emitter::*;
use sources::*;
use subsplease_scraper::*;

use crossbeam_channel::bounded;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

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

impl Display for CustomError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message)
    }
}

impl Error for CustomError {}

#[derive(Clone)]
pub struct ExtractOptions {
    pub dont_cache: Option<bool>,
}

pub struct RunOptions {
    pub extract_options: Option<ExtractOptions>,
    pub dont_cache: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Data {
    lists: MediaLists,
    schedule: AnimeSchedule,
}

pub struct Sources {
    anilist_api: AniListAPI,
    subsplease_scraper: SubsPleaseScraper,
}

pub struct Aggregator {
    config: Config,
    db: DB,
}

impl Aggregator {
    pub async fn new(config: Config) -> Aggregator {
        let db = DB::new(&config.db).await;

        Aggregator { config, db }
    }

    async fn extract(
        &mut self,
        mut sources: Sources,
        options: Option<ExtractOptions>,
    ) -> Result<Data> {
        let (lists, schedule) = tokio::join!(
            sources.anilist_api.extract(options.clone()),
            sources.subsplease_scraper.extract(options.clone())
        );

        let lists = lists?;
        let schedule = schedule?;

        Ok(Data { lists, schedule })
    }

    fn transform(&mut self, mut data: Data) -> Result<Data> {
        let emitter = Emitter::new(self.config.emitter.clone());
        let (snd, rcv) = bounded::<Emitted>(16);

        let media_lite: Vec<MediaLite> = data
            .lists
            .anime
            .iter()
            .map(|a: &Media| MediaLite {
                title: a.title.clone().unwrap_or(String::new()),
                alt_title: a.alt_title.clone().unwrap_or(String::new()),
                status: a.status.clone().unwrap_or(String::new()),
            })
            .collect();

        let handle = crossbeam::scope(|s| {
            // Future sources should spawn new threads to emit data to combine to main lists
            s.spawn(|_| {
                if let Err(err) = emitter.emit(&media_lite, &data.schedule.0, "schedule", snd) {
                    eprintln!("Error occurred in emitter thread: {}.", err);
                }
            });

            for msg in rcv.iter() {
                let extra = serde_json::from_str(&msg.json);
                match extra {
                    Ok(extra) => {
                        if msg.key == "schedule" {
                            data.lists.anime[msg.index].schedule = extra;
                        }
                    }
                    Err(err) => {
                        eprintln!("{}", err);
                    }
                }
            }
        });

        if let Err(err) = handle {
            eprintln!("{:?}", err);
        }

        Ok(data)
    }

    async fn load(&self, data: &Data) -> Result<()> {
        let mongodb = self.db.mongodb.lock().await;

        let anime_future = mongodb.upsert_documents("anime", "media_id", &data.lists.anime);
        let manga_future = mongodb.upsert_documents("manga", "media_id", &data.lists.manga);

        tokio::try_join!(anime_future, manga_future)?;

        Ok(())
    }

    pub async fn run(&mut self, options: Option<RunOptions>) -> Result<Data> {
        let sources = Sources {
            anilist_api: AniListAPI::new(self.config.anilist_api.clone(), self.db.redis.clone()),
            subsplease_scraper: SubsPleaseScraper::new(
                self.config.subsplease_scraper.clone(),
                self.db.redis.clone(),
            ),
        };

        let (dont_cache, extract_options) = match options {
            Some(options) => (options.dont_cache.unwrap_or(false), options.extract_options),
            None => (false, None),
        };

        let cache_key = sources
            .anilist_api
            .get_cache_key("aggregator:run", None)
            .await?;

        {
            let redis = self.db.redis.clone();
            let mut redis = redis.lock().await;
            if let Some(cached) = redis.get_cached::<Data>(&cache_key, Some(dont_cache)).await {
                println!("Got cached value for cache key: {}.", cache_key);
                return Ok(cached);
            }
        }

        let data = self.extract(sources, extract_options).await?;
        let data = self.transform(data)?;
        self.load(&data).await?;

        {
            let redis = self.db.redis.clone();
            let mut redis = redis.lock().await;
            redis
                .cache_value_expire(
                    &cache_key,
                    &data,
                    self.config.aggregator.ttl,
                    Some(dont_cache),
                )
                .await;
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_run() {
        let config = Config::default();
        let mongodb = MongoDB::new(config.db.mongodb.clone());
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

        let redis_client = Redis::new(config.db.redis.clone()).await.client;
        let mut connection = redis_client.get_connection().unwrap();
        redis::cmd("FLUSHALL").query::<()>(&mut connection).unwrap();

        let mut aggregator = Aggregator::new(config).await;
        let options = RunOptions {
            extract_options: Some(ExtractOptions {
                dont_cache: Some(true),
            }),
            dont_cache: Some(true),
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
