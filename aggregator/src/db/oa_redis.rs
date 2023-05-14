use crate::config::RedisConfig;
use crate::Result;

use redis::{aio::ConnectionManager, AsyncCommands, Client, FromRedisValue, ToRedisArgs};
use serde::{de::DeserializeOwned, Serialize};
use time::{Duration, OffsetDateTime, Time};

pub struct Redis {
    pub client: Client,
    pub connection_manager: ConnectionManager,
    pub config: RedisConfig,
}

impl Redis {
    pub async fn new(config: RedisConfig) -> Redis {
        let client = Client::open(config.host.as_str()).unwrap();
        let connection_manager = client.get_tokio_connection_manager().await.unwrap();

        Redis {
            client,
            connection_manager,
            config,
        }
    }

    async fn get<T>(&mut self, key: &str) -> Result<T>
    where
        T: FromRedisValue,
    {
        let result: T = self.connection_manager.get(key).await?;

        Ok(result)
    }

    async fn set_ex<T>(&mut self, key: &str, value: &T, seconds: usize) -> Result<()>
    where
        T: ToRedisArgs + Sync + Clone,
    {
        self.connection_manager.set_ex(key, value, seconds).await?;

        Ok(())
    }

    async fn set_ex_at<T>(&mut self, key: &str, value: &T, expire_at: usize) -> Result<()>
    where
        T: ToRedisArgs + Sync + Clone,
    {
        self.connection_manager.set(key, value).await?;
        self.connection_manager.expire_at(key, expire_at).await?;

        Ok(())
    }

    pub async fn get_cached<T>(&mut self, key: &str, dont_cache: Option<bool>) -> Option<T>
    where
        T: DeserializeOwned,
    {
        if let Some(dont_cache) = dont_cache {
            if dont_cache {
                return None;
            }
        }

        match self.get::<String>(key).await {
            Ok(cached) => match serde_json::from_str::<T>(&cached) {
                Ok(cached) => Some(cached),
                Err(err) => {
                    eprintln!("Could not parse cached value for {}: {}", key, err);
                    None
                }
            },
            Err(err) => {
                eprintln!("Could not get cached value for {}: {}", key, err);
                None
            }
        }
    }

    pub async fn cache_value_expire<T>(
        &mut self,
        key: &str,
        value: &T,
        seconds: usize,
        dont_cache: Option<bool>,
    ) where
        T: Serialize,
    {
        if let Some(dont_cache) = dont_cache {
            if dont_cache {
                return;
            }
        }

        let serialized = match serde_json::to_string(value) {
            Ok(serialized) => serialized,
            Err(err) => {
                eprintln!("Could not serialize results for key {}: {}.", key, err);
                return;
            }
        };

        if let Err(err) = self.set_ex::<String>(key, &serialized, seconds).await {
            eprintln!("Could not cache value for key {}: {}", key, err);
        }
    }

    pub async fn cache_value_expire_at<T>(
        &mut self,
        key: &str,
        value: &T,
        expire_at: usize,
        dont_cache: Option<bool>,
    ) where
        T: Serialize,
    {
        if let Some(dont_cache) = dont_cache {
            if dont_cache {
                return;
            }
        }

        let serialized = match serde_json::to_string(value) {
            Ok(serialized) => serialized,
            Err(err) => {
                eprintln!("Could not serialize results for key {}: {}.", key, err);
                return;
            }
        };

        if let Err(err) = self.set_ex_at::<String>(key, &serialized, expire_at).await {
            eprintln!("Could not cache value for key {}: {}", key, err);
        }
    }

    pub async fn cache_value_expire_tomorrow<T>(
        &mut self,
        key: &str,
        value: &T,
        dont_cache: Option<bool>,
    ) where
        T: Serialize,
    {
        let expire_at = match OffsetDateTime::now_utc().checked_add(Duration::DAY) {
            Some(date) => {
                let date = date.replace_time(Time::MIDNIGHT);
                match usize::try_from(date.unix_timestamp()) {
                    Ok(ts) => ts,
                    Err(err) => {
                        eprintln!(
                            "Could not get unix timestamp for tomorrow for key {}: {}",
                            key, err
                        );
                        self.config.ttl_fallback
                    }
                }
            }
            None => self.config.ttl_fallback,
        };

        self.cache_value_expire_at(key, value, expire_at, dont_cache)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_redis_cache() {
        let key = "test_redis_cache";
        let config = Config::default();
        let mut redis = Redis::new(config.db.redis).await;
        let expected = 420;
        redis.cache_value_expire(key, &expected, 10, None).await;
        let actual: i32 = redis.get_cached(key, None).await.unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_redis_cache_at() {
        let key = "test_redis_cache_at";
        let config = Config::default();
        let mut redis = Redis::new(config.db.redis).await;
        let expected = 420;
        let expire_at =
            usize::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap() + 10;

        redis
            .cache_value_expire_at(key, &expected, expire_at, None)
            .await;
        let actual: i32 = redis.get_cached(key, None).await.unwrap();
        assert_eq!(actual, expected);
    }
}
