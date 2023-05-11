use crate::config::{Config, DBConfig, MongoDBConfig, RedisConfig};
use crate::Result;

use bson::to_document;
use futures::future::try_join_all;
use mongodb::{
    bson::doc,
    options::{ClientOptions, FindOneAndUpdateOptions, ServerAddress},
};
#[allow(unused_imports)]
use redis::{AsyncCommands, Commands, FromRedisValue, ToRedisArgs};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Debug)]
pub struct MongoDB {
    pub client: mongodb::Client,
    pub config: MongoDBConfig,
}

pub trait Document: DeserializeOwned + Serialize + Hash + Unpin + Send + Sync {}

impl MongoDB {
    pub fn new(config: &MongoDBConfig) -> MongoDB {
        let address = ServerAddress::parse(&config.host).unwrap();
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name("oshirase-aggregator".to_owned())
            .build();
        let client = mongodb::Client::with_options(options).unwrap();

        MongoDB {
            client,
            config: config.clone(),
        }
    }

    fn hash_document<T>(document: &T) -> String
    where
        T: Document,
    {
        let mut hasher = DefaultHasher::new();
        document.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:x}", hash)
    }

    pub async fn upsert_documents<T>(&self, collection: &str, documents: &Vec<T>) -> Result<()>
    where
        T: Document,
    {
        let database = self.client.database(&self.config.database);
        let collection = database.collection::<T>(collection);

        let mut futures = Vec::new();

        for document in documents {
            let hash = Self::hash_document(document);

            let filter = doc! { "hash": &hash };
            let existing = collection.find_one(filter.clone(), None).await?;

            if existing.is_none() {
                let mut document = to_document(document)?;
                document.extend(doc! { "modified": bson::DateTime::now(), "hash": &hash });

                futures.push(collection.find_one_and_update(
                    filter.clone(),
                    doc! { "$set": document },
                    FindOneAndUpdateOptions::builder().upsert(true).build(),
                ));
            }
        }

        try_join_all(futures).await?;

        Ok(())
    }
}

impl Default for MongoDB {
    fn default() -> MongoDB {
        let config = Config::default();

        Self::new(&config.db.mongodb)
    }
}

#[derive(Debug)]
pub struct Redis {
    pub client: redis::Client,
    pub config: RedisConfig,
}

impl Redis {
    pub fn new(config: &RedisConfig) -> Redis {
        let client = redis::Client::open(config.host.as_str()).unwrap();

        Redis {
            client,
            config: config.clone(),
        }
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

    pub async fn get_cached<T>(&mut self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        match self.get::<String>(key).await {
            Ok(cached) => match serde_json::from_str::<T>(&cached) {
                Ok(cached) => Some(cached),
                Err(err) => {
                    println!("Could not parse cached value: {}", err);
                    None
                }
            },
            Err(err) => {
                println!("Could not get cached value: {}", err);
                None
            }
        }
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

impl Default for Redis {
    fn default() -> Redis {
        let config = Config::default();

        Self::new(&config.db.redis)
    }
}

#[derive(Debug)]
pub struct DB {
    pub mongodb: MongoDB,
    pub redis: Redis,
}

impl DB {
    pub fn new(config: &DBConfig) -> DB {
        DB {
            mongodb: MongoDB::new(&config.mongodb),
            redis: Redis::new(&config.redis),
        }
    }
}

impl Default for DB {
    fn default() -> DB {
        let config = Config::default();

        DB::new(&config.db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[tokio::test]
    async fn test_mongodb_new() {
        let mongo = MongoDB::new(&MongoDBConfig {
            host: "127.0.0.1".to_owned(),
            database: "database".to_owned(),
        });
        assert_eq!(mongo.config.database, "database");
    }

    #[tokio::test]
    async fn test_mongodb_default() {
        let mongo = MongoDB::default();
        let actual = mongo.client.list_database_names(None, None).await.unwrap();
        assert!(actual.contains(&"admin".to_owned()));
        assert!(actual.contains(&"config".to_owned()));
        assert!(actual.contains(&"local".to_owned()));
    }

    #[derive(Debug, Hash, PartialEq, Serialize, Deserialize)]
    struct Test {
        test: String,
    }

    impl Document for Test {}

    #[tokio::test]
    async fn test_mongodb_upsert_documents() {
        let mongo = MongoDB::default();
        let collection = mongo
            .client
            .database(&mongo.config.database)
            .collection::<Test>("test");
        collection.drop(None).await.unwrap();

        mongo
            .upsert_documents(
                "test",
                &vec![Test {
                    test: "test".to_owned(),
                }],
            )
            .await
            .unwrap();

        let count = collection
            .count_documents(doc! { "test": "test" }, None)
            .await
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_redis_new() {
        let redis = Redis::new(&RedisConfig {
            host: "redis://localhost/".to_owned(),
        });
        assert_eq!(redis.config.host, "redis://localhost/");
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
        let key = "test_redis_cache";
        let mut redis = Redis::default();
        let expected = 420;
        redis.cache_value_ex(key, &expected, 10).await;
        let actual: i32 = redis.get_cached(key).await.unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_db_new() {
        let db = DB::new(&DBConfig {
            mongodb: MongoDBConfig {
                host: "host".to_owned(),
                database: "database".to_owned(),
            },
            redis: RedisConfig {
                host: "redis://localhost/".to_owned(),
            },
        });
        assert_eq!(db.mongodb.config.host, "host");
        assert_eq!(db.redis.config.host, "redis://localhost/");
    }

    #[tokio::test]
    async fn test_db_default() {
        let db = DB::default();
        assert_eq!(db.mongodb.config.host, "127.0.0.1");
        assert_eq!(db.redis.config.host, "redis://127.0.0.1/");
    }
}
