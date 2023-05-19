mod oa_mongodb;
mod oa_redis;

pub use oa_mongodb::MongoDB;
pub use oa_mongodb::Persist;
pub use oa_redis::Redis;

use crate::config::Config;

use serde::{de::DeserializeOwned, Serialize};
use std::hash::Hash;

pub trait Document: DeserializeOwned + Serialize + Hash + Unpin + Send + Sync {}

pub struct DB<'a> {
    pub mongodb: MongoDB<'a>,
    pub redis: Redis<'a>,
}

impl<'a> DB<'_> {
    pub async fn new(config: &'a Config) -> DB {
        DB {
            mongodb: MongoDB::init(config).await,
            redis: Redis::new(config),
        }
    }
}
