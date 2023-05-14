pub mod anilist_api;
pub mod subsplease_scraper;

use crate::ExtractOptions;
use crate::Result;

use async_trait::async_trait;

#[async_trait]
pub trait Source {
    type Data: serde::Serialize + Send + Sync;

    async fn extract(&mut self, options: Option<ExtractOptions>) -> Result<Self::Data>;
}
