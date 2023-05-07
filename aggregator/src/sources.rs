use async_trait::async_trait;
use std::error::Error;

pub mod anilist_api;

pub use anilist_api::AniListAPI;

pub enum Sources {
    AniListAPI(AniListAPI),
}

#[async_trait]
pub trait Source {
    type Data;

    async fn aggregate(&self) -> Result<Self::Data, Box<dyn Error>>;
}
