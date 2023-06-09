mod config;
mod db;
mod error;
mod options;
mod result;
mod sources;
mod test;
mod worker;

use alt_titles_db::*;
use anilist_api::*;
use db::MongoDB;
use mangadex_api::*;
use options::*;
use sources::*;
use subsplease_rss::*;
use subsplease_scraper::*;

pub use config::Config;
pub use error::CustomError;
pub use result::Result;
pub use worker::Worker;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Data {
    lists: MediaLists,
    schedule: AnimeSchedule,
    alt_titles: AltTitles,
    anime_latest: AnimeLatest,
    manga_latest: MangaLatest,
}

pub struct Aggregator<'a> {
    config: &'a Config,
}

impl<'a> Aggregator<'a> {
    pub fn new(config: &'a Config) -> Aggregator<'a> {
        Aggregator { config }
    }

    async fn extract(
        &self,
        sources: &Sources<'a>,
        options: Option<ExtractOptions>,
    ) -> Result<Data> {
        let (lists, alt_titles, schedule, anime_latest, manga_latest) = tokio::try_join!(
            sources.anilist_api.extract(options.clone()),
            sources.alt_titles_db.extract(options.clone()),
            sources.subsplease_scraper.extract(options.clone()),
            sources.subsplease_rss.extract(options.clone()),
            sources.mangadex_api.extract(options)
        )?;

        Ok(Data {
            lists,
            alt_titles,
            schedule,
            anime_latest,
            manga_latest,
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
                let extras = [
                    Extras::SubsPleaseScraper(sources.subsplease_scraper.clone()),
                    Extras::SubsPleaseRSS(sources.subsplease_rss.clone()),
                ];

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
                        Extras::SubsPleaseRSS(extra) => {
                            let mut transformed = match extra.transform(anime, &data.anime_latest.0)
                            {
                                Ok(anime) => anime,
                                Err(err) => {
                                    eprintln!("Could not transform media: {}", err);
                                    std::mem::take(anime)
                                }
                            };
                            *anime = std::mem::take(&mut transformed);
                        }
                        _ => {}
                    }
                }

                anime
            })
            .map(std::mem::take)
            .collect();
        data.lists.anime = anime;

        let manga = data
            .lists
            .manga
            .par_iter_mut()
            .map(|manga| {
                let mut transformed =
                    match sources.alt_titles_db.transform(manga, &data.alt_titles.0) {
                        Ok(manga) => manga,
                        Err(err) => {
                            eprintln!("Could not add alt titles: {}", err);
                            std::mem::take(manga)
                        }
                    };
                *manga = std::mem::take(&mut transformed);
                manga
            })
            .map(|manga| {
                let extras = [Extras::MangaDexAPI(sources.mangadex_api.clone())];

                for extra in extras {
                    if let Extras::MangaDexAPI(extra) = extra {
                        let mut transformed = match extra.transform(manga, &data.manga_latest.0) {
                            Ok(manga) => manga,
                            Err(err) => {
                                eprintln!("Could not transform media: {}", err);
                                std::mem::take(manga)
                            }
                        };
                        *manga = std::mem::take(&mut transformed);
                    }
                }

                manga
            })
            .map(std::mem::take)
            .collect();
        data.lists.manga = manga;

        Ok(data)
    }

    async fn load(&self, data: &'a mut Data, mongodb: &'a MongoDB<'a>) -> Result<&'a mut Data> {
        tokio::try_join!(
            mongodb.upsert_documents("anime", &data.lists.anime, "media_id"),
            mongodb.upsert_documents("manga", &data.lists.manga, "media_id")
        )?;

        Ok(data)
    }

    pub async fn run(&self) -> Result<Data> {
        let sources = Sources {
            anilist_api: AniListAPI::new(self.config),
            subsplease_scraper: SubsPleaseScraper::new(self.config),
            subsplease_rss: SubsPleaseRSS::new(self.config),
            mangadex_api: MangaDexAPI::new(self.config),
            alt_titles_db: AltTitlesDB::new(self.config),
        };

        let mongodb = MongoDB::init(self.config).await;

        let extract_options = ExtractOptions {
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
    use test::helpers::{init, reset_db, ONCE};

    use mongodb::bson::doc;

    #[tokio::test]
    async fn test_run() {
        ONCE.get_or_init(init).await;
        reset_db().await;

        let config = Config::default();
        let mongodb = MongoDB::new(&config).await;
        let aggregator = Aggregator::new(&config);
        aggregator.run().await.unwrap();

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
