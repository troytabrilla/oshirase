pub mod anilist_api;
pub mod subsplease_scraper;

use crate::Result;

use async_trait::async_trait;

#[async_trait]
pub trait Source {
    type Data;

    async fn extract(&mut self) -> Result<Self::Data>;
}
