use std::error::Error;

pub mod config;
pub mod db;
pub mod sources;

pub use sources::*;

pub struct Aggregator {
    anilist_api: AniListAPI,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let anilist_api = AniListAPI::from("config/anilist_api.yaml");

        Aggregator { anilist_api }
    }
}

impl Aggregator {
    pub async fn aggregate(&self) -> Result<(), Box<dyn Error>> {
        let data = self.anilist_api.aggregate().await?;
        // @todo Save to mongodb
        // @todo Set up Docker
        println!("{:#?}", data);

        Ok(())
    }
}
