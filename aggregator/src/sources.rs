pub mod anilist_api;
pub mod subsplease_scraper;

use crate::Result;

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait Source<'a> {
    type Data: Serialize;

    async fn extract(&mut self) -> Result<Self::Data>;
}
