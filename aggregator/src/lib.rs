use std::error::Error;

pub mod config;
pub mod db;
pub mod sources;

pub use sources::*;

pub struct Aggregator {
    sources: Vec<sources::Sources>,
}

impl Default for Aggregator {
    fn default() -> Aggregator {
        let anilist_api = AniListAPI::from("config/anilist_api.yaml");
        let sources = vec![Sources::AniListAPI(anilist_api)];

        Aggregator { sources }
    }
}

impl Aggregator {
    pub async fn aggregate(&self) -> Result<(), Box<dyn Error>> {
        for source in &self.sources {
            match source {
                Sources::AniListAPI(anilist_api) => {
                    let lists = anilist_api.aggregate().await?;
                    // @todo Save to mongodb
                    // @todo Set up Docker
                    println!("{:#?}", lists.anime);
                }
            }
        }

        Ok(())
    }
}
