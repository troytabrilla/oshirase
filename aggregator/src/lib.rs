mod config;
mod db;
mod error;
mod sources;
mod test;
mod worker;

pub use config::Config;
pub use error::CustomError;
pub use worker::Worker;

use alt_titles_db::*;
use anilist_api::*;
use db::*;
use sources::*;
use subsplease_scraper::*;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub struct RunOptions {
    pub user_id: Option<u64>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Data {
    lists: MediaLists,
    schedule: AnimeSchedule,
    alt_titles: AltTitles,
}

pub struct Sources<'a> {
    anilist_api: AniListAPI<'a>,
    subsplease_scraper: SubsPleaseScraper<'a>,
    alt_titles_db: AltTitlesDB<'a>,
}

pub struct Aggregator<'a> {
    config: &'a Config,
}

impl<'a> Aggregator<'a> {
    pub async fn new(config: &'a Config) -> Aggregator<'a> {
        Aggregator { config }
    }

    async fn extract(
        &self,
        sources: &Sources<'a>,
        options: Option<ExtractOptions>,
    ) -> Result<Data> {
        let (lists, schedule, alt_titles) = tokio::try_join!(
            sources.anilist_api.extract(options.clone()),
            sources.subsplease_scraper.extract(options.clone()),
            sources.alt_titles_db.extract(options),
        )?;

        Ok(Data {
            lists,
            schedule,
            alt_titles,
        })
    }

    fn transform(&self, sources: &Sources<'a>, data: &'a mut Data) -> Result<&'a mut Data> {
        let anime = data
            .lists
            .anime
            .par_iter_mut()
            .map(|anime| {
                let mut transformed =
                    match sources.alt_titles_db.transform(anime, &data.alt_titles.0) {
                        Ok(anime) => anime,
                        Err(err) => {
                            eprintln!("Could not add alt titles: {}", err);
                            std::mem::take(anime)
                        }
                    };
                *anime = std::mem::take(&mut transformed);
                anime
            })
            .map(|anime| {
                let extras = [Extras::SubsPleaseScraper(
                    sources.subsplease_scraper.clone(),
                )];

                for extra in extras {
                    match extra {
                        Extras::SubsPleaseScraper(extra) => {
                            let mut transformed = match extra.transform(anime, &data.schedule.0) {
                                Ok(anime) => anime,
                                Err(err) => {
                                    eprintln!("Could not transform media: {}", err);
                                    std::mem::take(anime)
                                }
                            };
                            *anime = std::mem::take(&mut transformed);
                        }
                    }
                }

                anime
            })
            .map(std::mem::take)
            .collect();
        data.lists.anime = anime;

        Ok(data)
    }

    async fn load(&self, data: &'a mut Data, mongodb: &'a MongoDB<'a>) -> Result<&'a mut Data> {
        let anime_future = mongodb.upsert_documents("anime", &data.lists.anime, "media_id");
        let manga_future = mongodb.upsert_documents("manga", &data.lists.manga, "media_id");

        tokio::try_join!(anime_future, manga_future)?;

        Ok(data)
    }

    pub async fn run(&self, options: Option<RunOptions>) -> Result<Data> {
        let sources = Sources {
            anilist_api: AniListAPI::new(self.config),
            subsplease_scraper: SubsPleaseScraper::new(self.config),
            alt_titles_db: AltTitlesDB::new(self.config),
        };

        let user_id = match options {
            Some(options) => options.user_id,
            None => None,
        };

        let mongodb = MongoDB::init(self.config).await;

        let extract_options = ExtractOptions {
            user_id,
            mongodb_client: Some(mongodb.client.clone()),
        };

        let mut data = self.extract(&sources, Some(extract_options)).await?;
        let data = self.transform(&sources, &mut data)?;
        let data = self.load(data, &mongodb).await?;

        Ok(std::mem::take(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::helpers::{init, ONCE};

    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_run() {
        ONCE.get_or_init(init).await;
        let config = Config::default();
        let mongodb = MongoDB::new(&config).await;
        let aggregator = Aggregator::new(&config).await;
        aggregator.run(None).await.unwrap();

        let database = mongodb.client.database(&config.db.mongodb.database);

        let anime: bson::Document = database
            .collection("anime")
            .find_one(doc! { "media_id": 918 }, None)
            .await
            .unwrap()
            .unwrap();

        let manga: bson::Document = database
            .collection("manga")
            .find_one(doc! { "media_id": 30044 }, None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(anime.get("title").unwrap().as_str().unwrap(), "Gintama");
        assert_eq!(manga.get("title").unwrap().as_str().unwrap(), "Gintama");
    }
}
