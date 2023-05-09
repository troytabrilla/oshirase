use mongodb::bson::doc;
use std::{error::Error, fmt};
use tokio::try_join;

mod config;
mod db;
mod sources;

use anilist_api::*;
use sources::*;

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
    data: Option<Data>,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let anilist_api = AniListAPI::default();

        Aggregator {
            anilist_api,
            data: None,
        }
    }
}

impl Aggregator {
    async fn extract(&mut self) -> Result<&mut Self, Box<dyn Error>> {
        let lists = self.anilist_api.extract().await?;
        self.data = Some(Data { lists });

        Ok(self)
    }

    async fn transform(&mut self) -> Result<&mut Self, Box<dyn Error>> {
        // @todo Combine data from sources into one result, i.e. update `latest` field
        Ok(self)
    }

    async fn load(&self) -> Result<&Self, Box<dyn Error>> {
        // @todo Set up Docker
        let mongodb = db::MongoDB::default();

        let lists = match &self.data {
            Some(data) => &data.lists,
            None => return Err(AggregatorError::boxed("No lists to persist.")),
        };

        let query = |doc: &bson::Document| doc! { "media_id": doc.get("media_id") };

        let anime_future = mongodb.upsert_documents("anime", &lists.anime, query);

        let manga_future = mongodb.upsert_documents("manga", &lists.manga, query);

        try_join!(anime_future, manga_future)?;

        Ok(self)
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.extract().await?.transform().await?.load().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
