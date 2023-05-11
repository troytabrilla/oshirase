pub mod anilist_api;
pub mod subsplease_scraper;

use crate::ExtractOptions;
use crate::Result;

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait Source {
    type Data: Serialize + Send + Sync;

    async fn extract(&mut self, options: Option<&ExtractOptions>) -> Result<Self::Data> {
        let cache_key = self.get_key();
        let cache_key = cache_key.await;

        let dont_cache = match options {
            Some(options) => options.dont_cache && !cache_key.is_empty(),
            None => false,
        };

        if !dont_cache {
            if let Some(cached) = self.get_cached(&cache_key).await {
                println!("Got cached value for cache key: {}.", cache_key);
                return Ok(cached);
            }
        }

        let data = self.get_data();
        let data = data.await?;

        if !dont_cache {
            self.cache_value(&cache_key, &data).await;
        }

        Ok(data)
    }

    async fn get_key(&self) -> String;

    async fn get_data(&self) -> Result<Self::Data>;

    async fn get_cached(&mut self, key: &str) -> Option<Self::Data>;

    async fn cache_value(&mut self, key: &str, lists: &Self::Data);
}
