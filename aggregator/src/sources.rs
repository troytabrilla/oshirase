use async_trait::async_trait;
use std::error::Error;

pub mod anilist_api;

#[async_trait]
pub trait Source {
    type Data;

    async fn extract(&self) -> Result<Self::Data, Box<dyn Error>>;
}
