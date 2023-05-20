pub mod alt_titles_db;
pub mod anilist_api;
pub mod subsplease_scraper;

use crate::Result;

use async_trait::async_trait;
use serde::Serialize;

pub struct ExtractOptions {
    pub user_id: Option<u64>,
}

// @todo Add nyaa and mangadex sources
// @todo Add enum for sources
#[async_trait]
pub trait Source<'a> {
    type Data: Serialize;

    async fn extract(&mut self, options: Option<&ExtractOptions>) -> Result<Self::Data>;
}
