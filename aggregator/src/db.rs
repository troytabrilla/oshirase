use crate::config::{Config, MongoDBConfig, RedisConfig};
use crate::Result;

use bson::to_document;
use futures::future::try_join_all;
use mongodb::{
    bson::doc,
    options::{ClientOptions, ServerAddress, UpdateOptions},
};
use redis::AsyncCommands;
#[allow(unused_imports)]
use redis::Commands;
use redis::{FromRedisValue, ToRedisArgs};
use serde::{de::DeserializeOwned, Serialize};

pub struct MongoDB {
    pub client: mongodb::Client,
    pub database: String,
}

impl Default for MongoDB {
    fn default() -> MongoDB {
        let config = Config::default();

        Self::new(config.db.mongodb)
    }
}

impl MongoDB {
    pub fn new(config: MongoDBConfig) -> MongoDB {
        let address = ServerAddress::parse(config.host).unwrap();
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name("oshirase-aggregator".to_owned())
            .build();
        let client = mongodb::Client::with_options(options).unwrap();

        MongoDB {
            client,
            database: config.database,
        }
    }

    pub async fn upsert_documents<T, F>(
        &self,
        collection: &str,
        documents: &Vec<T>,
        query: F,
    ) -> Result<()>
    where
        T: Serialize,
        F: Fn(&bson::Document) -> bson::Document,
    {
        let database = self.client.database(&self.database);
        let collection = database.collection::<T>(collection);

        let mut futures = Vec::new();

        for document in documents {
            let document = to_document(document)?;
            let query = query(&document);
            futures.push(collection.update_one(
                query.clone(),
                doc! { "$set": document },
                UpdateOptions::builder().upsert(true).build(),
            ));
        }

        try_join_all(futures).await?;

        Ok(())
    }
}

pub struct Redis {
    pub client: redis::Client,
}

impl Default for Redis {
    fn default() -> Redis {
        let config = Config::default();

        Self::new(config.db.redis)
    }
}

impl Redis {
    pub fn new(config: RedisConfig) -> Redis {
        let client = redis::Client::open(config.host).unwrap();

        Redis { client }
    }

    async fn get<T>(&mut self, key: &str) -> Result<T>
    where
        T: FromRedisValue + std::fmt::Debug,
    {
        let mut connection = self.client.get_async_connection().await?;
        let result: T = connection.get(key).await?;

        Ok(result)
    }

    async fn set_ex<T>(&mut self, key: &str, value: &T, seconds: usize) -> Result<()>
    where
        T: ToRedisArgs + std::marker::Sync + std::clone::Clone,
    {
        let mut connection = self.client.get_async_connection().await?;
        connection.set_ex(key, &(*value).clone(), seconds).await?;

        Ok(())
    }

    pub async fn check_cache<T>(&mut self, key: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let cached = self.get::<String>(key).await?;
        let cached = serde_json::from_str::<T>(&cached)?;

        Ok(cached)
    }

    pub async fn cache_value_ex<T>(&mut self, key: &str, value: &T, seconds: usize)
    where
        T: Serialize,
    {
        let serialized = match serde_json::to_string(value) {
            Ok(serialized) => serialized,
            Err(err) => {
                println!("Could not stringify results: {}.", err);
                return;
            }
        };

        if let Err(err) = self.set_ex::<String>(key, &serialized, seconds).await {
            println!("Could not cache value for key {}: {}", key, err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mongodb_default() {
        let mongo = MongoDB::default();
        let actual = mongo.client.list_database_names(None, None).await.unwrap();
        assert!(actual.contains(&"admin".to_owned()));
        assert!(actual.contains(&"config".to_owned()));
        assert!(actual.contains(&"local".to_owned()));
    }

    #[test]
    fn test_redis_default() {
        let key = "test_redis_default";
        let redis = Redis::default();
        let mut connection = redis.client.get_connection().unwrap();
        let _: () = connection.set(key, 42).unwrap();
        let actual: i32 = connection.get(key).unwrap();
        let expected = 42;
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_redis_cache() {
        let key = "test_redis_set_and_get";
        let mut redis = Redis::default();
        let expected = 420;
        redis.cache_value_ex(key, &expected, 10).await;
        let actual: i32 = redis.check_cache(key).await.unwrap();
        assert_eq!(actual, expected);
    }
}
