use crate::config::Config;
use crate::Result;
use bson::to_document;
use futures::future::try_join_all;
use mongodb::bson::doc;
use mongodb::options::{ClientOptions, ServerAddress, UpdateOptions};
use redis::{FromRedisValue, ToRedisArgs};
use serde::Serialize;
extern crate redis;
use crate::db::redis::AsyncCommands;
#[allow(unused_imports)]
use redis::Commands;

pub struct MongoDB {
    pub client: mongodb::Client,
    pub database: String,
}

impl Default for MongoDB {
    fn default() -> MongoDB {
        let config = Config::default();
        let address = ServerAddress::parse(config.db.mongodb.host).unwrap();
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name("oshirase-aggregator".to_owned())
            .build();
        let client = mongodb::Client::with_options(options).unwrap();

        MongoDB {
            client,
            database: config.db.mongodb.database,
        }
    }
}

impl MongoDB {
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
        let client = redis::Client::open(config.db.redis.host).unwrap();

        Redis { client }
    }
}

impl Redis {
    pub async fn get<T>(&mut self, key: &str) -> Result<T>
    where
        T: FromRedisValue + std::fmt::Debug,
    {
        let mut connection = self.client.get_async_connection().await?;
        let result: T = connection.get(key).await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn set<T>(&mut self, key: &str, value: &T) -> Result<()>
    where
        T: ToRedisArgs + std::marker::Sync + std::clone::Clone,
    {
        let mut connection = self.client.get_async_connection().await?;
        connection.set(key, &(*value).clone()).await?;

        Ok(())
    }

    pub async fn set_ex<T>(&mut self, key: &str, value: &T, seconds: usize) -> Result<()>
    where
        T: ToRedisArgs + std::marker::Sync + std::clone::Clone,
    {
        let mut connection = self.client.get_async_connection().await?;
        connection.set_ex(key, &(*value).clone(), seconds).await?;

        Ok(())
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
    async fn test_redis_set_and_get() {
        let key = "test_redis_set_and_get";
        let mut redis = Redis::default();
        let expected = 420;
        redis.set(key, &expected).await.unwrap();
        let actual = redis.get::<i32>(key).await.unwrap();
        assert_eq!(actual, expected);
    }
}
