mod oa_mongodb;
mod oa_redis;

pub use oa_mongodb::MongoDB;
pub use oa_redis::Redis;

use crate::config::DBConfig;

use serde::{de::DeserializeOwned, Serialize};
use std::{hash::Hash, sync::Arc};
use tokio::sync::Mutex;

pub trait Document: DeserializeOwned + Serialize + Hash + Unpin + Send + Sync {}

pub struct DB {
    pub mongodb: Arc<Mutex<MongoDB>>,
    pub redis: Arc<Mutex<Redis>>,
}

impl DB {
    pub async fn new(config: &DBConfig) -> DB {
        DB {
            mongodb: Arc::new(Mutex::new(MongoDB::new(config.mongodb.clone()))),
            redis: Arc::new(Mutex::new(Redis::new(config.redis.clone()).await)),
        }
    }
}
