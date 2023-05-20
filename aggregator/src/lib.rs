mod config;
mod db;
mod error;
mod sources;
mod test;
mod transform;
mod worker;

pub use config::Config;
pub use error::CustomError;
pub use worker::Worker;

use alt_titles_db::*;
use anilist_api::*;
use db::*;
use sources::*;
use subsplease_scraper::*;
use transform::*;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Data {
    lists: MediaLists,
    schedule: AnimeSchedule,
    alt_titles: AltTitles,
}

// @todo Sources should be a list of Source enums, extract and transform should iterate through sources
pub struct Sources<'a> {
    anilist_api: AniListAPI<'a>,
    subsplease_scraper: SubsPleaseScraper<'a>,
    alt_titles_db: AltTitlesDB<'a>,
}

pub struct Aggregator<'a> {
    config: &'a Config,
    db: DB<'a>,
    data: Option<Data>,
}

impl<'a> Aggregator<'a> {
    pub async fn new(config: &'a Config) -> Aggregator<'a> {
        Aggregator {
            config,
            db: DB::new(config).await,
            data: None,
        }
    }

    async fn extract(
        &mut self,
        sources: &mut Sources<'a>,
        options: Option<&ExtractOptions>,
    ) -> Result<&mut Aggregator<'a>> {
        let (lists, schedule, alt_titles) = tokio::join!(
            sources.anilist_api.extract(options),
            sources.subsplease_scraper.extract(options),
            sources.alt_titles_db.extract(options),
        );

        let lists = lists?;
        let schedule = schedule?;
        let alt_titles = alt_titles?;

        self.data = Some(Data {
            lists,
            schedule,
            alt_titles,
        });

        Ok(self)
    }

    fn transform(&mut self, sources: Sources<'a>) -> Result<&mut Aggregator<'a>> {
        if let Some(mut data) = self.data.as_mut() {
            let anime = data
                .lists
                .anime
                .par_iter_mut()
                .map(|anime| {
                    let mut transformed =
                        match sources.alt_titles_db.transform(anime, &data.alt_titles.0) {
                            Ok(anime) => anime,
                            Err(err) => {
                                eprintln!("Could not transform media with alt titles: {}", err);
                                std::mem::take(anime)
                            }
                        };
                    *anime = std::mem::take(&mut transformed);
                    anime
                })
                .map(|anime| {
                    let mut transformed = match sources
                        .subsplease_scraper
                        .transform(anime, &data.schedule.0)
                    {
                        Ok(anime) => anime,
                        Err(err) => {
                            eprintln!("Could not transform media with schedule: {}", err);
                            std::mem::take(anime)
                        }
                    };
                    *anime = std::mem::take(&mut transformed);
                    anime
                })
                .map(std::mem::take)
                .collect();
            data.lists.anime = anime;
            self.data = Some(std::mem::take(data));
        }

        Ok(self)
    }

    async fn load(&mut self) -> Result<&mut Aggregator<'a>> {
        if let Some(data) = &self.data {
            let anime_future = self.upsert_documents("anime", &data.lists.anime, "media_id");
            let manga_future = self.upsert_documents("manga", &data.lists.manga, "media_id");

            tokio::try_join!(anime_future, manga_future)?;
        }

        Ok(self)
    }

    pub async fn run(&mut self, user_id: Option<u64>) -> Result<Data> {
        let mut sources = Sources {
            anilist_api: AniListAPI::new(self.config, self.db.mongodb.client.clone()),
            subsplease_scraper: SubsPleaseScraper::new(self.config),
            alt_titles_db: AltTitlesDB::new(self.config, self.db.mongodb.client.clone()),
        };

        let extract_options = ExtractOptions { user_id };

        let data = self
            .extract(&mut sources, Some(&extract_options))
            .await?
            .transform(sources)?
            .load()
            .await?
            .data
            .as_mut();

        match data {
            Some(data) => Ok(std::mem::take(data)),
            None => Err(CustomError::boxed("Could not unwrap data.")),
        }
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
    use test::helpers::{init, ONCE};

    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_run() {
        ONCE.get_or_init(init).await;
        let config = Config::default();
        let mut aggregator = Aggregator::new(&config).await;
        aggregator.run(None).await.unwrap();

        let database = aggregator
            .db
            .mongodb
            .client
            .database(&config.db.mongodb.database);

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
