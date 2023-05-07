use async_trait::async_trait;
use std::error::Error;

pub mod anilist_api;

pub use anilist_api::AniListAPI;

#[async_trait]
pub trait Source {
    async fn aggregate(&self) -> Result<(), Box<dyn Error>>;
}
