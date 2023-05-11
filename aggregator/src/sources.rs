pub mod anilist_api;
pub mod subsplease_scraper;

use crate::Result;

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait Source {
    type Data: Serialize;

    async fn extract(&mut self) -> Result<Self::Data>;

    async fn get_cached(&mut self, key: &str) -> Option<Self::Data>;

    async fn cache_value(&mut self, key: &str, lists: &Self::Data);
}
