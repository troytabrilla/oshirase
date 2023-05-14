pub mod anilist_api;
pub mod subsplease_scraper;

use crate::ExtractOptions;
use crate::Result;

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait Source {
    type Data: Serialize;

    async fn extract(&mut self, options: Option<ExtractOptions>) -> Result<Self::Data>;
}
