use crate::config::Config;
use mongodb::options::{ClientOptions, ServerAddress};
use redis::{FromRedisValue, ToRedisArgs};
extern crate redis;
use crate::db::redis::AsyncCommands;
#[allow(unused_imports)]
use redis::Commands;
use std::error::Error;

pub struct MongoDB {
    pub client: mongodb::Client,
}

impl Default for MongoDB {
    fn default() -> MongoDB {
        let config = Config::default();
        let address = ServerAddress::parse(config.db.mongodb.host)
            .expect("Could not parse MongoDB host address.");
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name("oshirase-aggregator".to_owned())
            .build();
        let client =
            mongodb::Client::with_options(options).expect("Could not create mongodb client.");
        MongoDB { client }
    }
}

pub struct Redis {
    pub client: redis::Client,
}

impl Default for Redis {
    fn default() -> Redis {
        let config = Config::default();
        let client =
            redis::Client::open(config.db.redis.host).expect("Could not create redis client.");
        Redis { client }
    }
}

impl Redis {
    pub async fn get<T>(&mut self, key: &str) -> Result<T, Box<dyn Error>>
    where
        T: FromRedisValue + std::fmt::Debug,
    {
        let mut connection = self.client.get_async_connection().await?;

        let result: Result<T, redis::RedisError> = connection.get(key).await;
        match result {
            Ok(result) => Ok(result),
            Err(err) => {
                println!("{}", err);
                Err(Box::new(err))
            }
        }
    }

    pub async fn set<T>(&mut self, key: &str, value: &T) -> Result<(), Box<dyn Error>>
    where
        T: ToRedisArgs + std::marker::Sync + std::clone::Clone,
    {
        let mut connection = self.client.get_async_connection().await?;

        connection.set(key, &(*value).clone()).await?;
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
        let redis = Redis::default();
        let mut connection = redis.client.get_connection().unwrap();
        let _: () = connection.set("test", 42).unwrap();
        let actual: i32 = connection.get("test").unwrap();
        let expected = 42;
        assert_eq!(actual, expected);
    }
}
