use std::error::Error;

mod config;
mod db;
mod sources;

use anilist_api::*;
use serde::{Deserialize, Serialize};
use sources::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Data {
    lists: Box<MediaLists>,
}

pub struct Aggregator {
    anilist_api: AniListAPI,
    data: Option<Box<Data>>,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let anilist_api = AniListAPI::from("config/anilist_api.yaml");

        Aggregator {
            anilist_api,
            data: None,
        }
    }
}

impl Aggregator {
    async fn extract(&mut self) -> Result<&mut Self, Box<dyn Error>> {
        let lists = self.anilist_api.extract().await?;
        self.data = Some(Box::new(Data {
            lists: Box::new(lists),
        }));

        Ok(self)
    }

    async fn transform(&mut self) -> Result<&mut Self, Box<dyn Error>> {
        // @todo Combine data from sources into one result, i.e. update `latest` field
        Ok(self)
    }

    async fn load(&self) -> Result<&Self, Box<dyn Error>> {
        // @todo Save to mongodb, only if there are changes (use aggregation pipelines?)
        // @todo Set up Docker
        println!("{:#?}", self.data);
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
    async fn test_run() {
        let mut aggregator = Aggregator::default();
        aggregator.run().await.unwrap();
        // @todo Check test db
    }
}
