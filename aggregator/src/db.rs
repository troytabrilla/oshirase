mod oa_mongodb;
mod oa_redis;

pub use oa_mongodb::MongoDB;
pub use oa_mongodb::Persist;
pub use oa_redis::Cache;
pub use oa_redis::Redis;

use crate::config::DBConfig;

use serde::{de::DeserializeOwned, Serialize};
use std::hash::Hash;

pub trait Document: DeserializeOwned + Serialize + Hash + Unpin + Send + Sync {}

pub struct DB {
    pub mongodb: MongoDB,
    pub redis: Redis,
}

impl DB {
    pub async fn new(config: &DBConfig) -> DB {
        DB {
            mongodb: MongoDB::new(config.mongodb.clone()),
            redis: Redis::new(config.redis.clone()).await,
        }
    }
}
