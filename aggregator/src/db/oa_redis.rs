use crate::config::RedisConfig;
use crate::Result;

use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client, FromRedisValue, ToRedisArgs};
use serde::{de::DeserializeOwned, Serialize};
use time::{Duration, OffsetDateTime, Time};

const TTL_FALLBACK: usize = 86400;

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
}

#[async_trait]
pub trait Cache {
    fn get_connection_manager(&mut self) -> &mut redis::aio::ConnectionManager;

    async fn get<T>(&mut self, key: &str) -> Result<T>
    where
        T: FromRedisValue + Send + Sync,
    {
        let connection_manager = self.get_connection_manager();
        let result = connection_manager.get(key).await?;

        Ok(result)
    }

    async fn set_ex<T>(&mut self, key: &str, value: &T, seconds: usize) -> Result<()>
    where
        T: ToRedisArgs + Sync,
    {
        let connection_manager = self.get_connection_manager();
        connection_manager.set_ex(key, value, seconds).await?;

        Ok(())
    }

    async fn set_ex_at<T>(&mut self, key: &str, value: &T, expire_at: usize) -> Result<()>
    where
        T: ToRedisArgs + Sync,
    {
        let mut cmd = redis::Cmd::new();
        let cmd = cmd
            .arg("SET")
            .arg(key)
            .arg(value)
            .arg("EXAT")
            .arg(expire_at);
        let connection_manager = self.get_connection_manager();
        connection_manager.send_packed_command(cmd).await?;

        Ok(())
    }

    async fn get_cached<T>(&mut self, key: &str, dont_cache: Option<bool>) -> Option<T>
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

    async fn cache_value_expire<T>(
        &mut self,
        key: &str,
        value: &T,
        seconds: usize,
        dont_cache: Option<bool>,
    ) where
        T: Serialize + Sync,
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

    async fn cache_value_expire_at<T>(
        &mut self,
        key: &str,
        value: &T,
        expire_at: usize,
        dont_cache: Option<bool>,
    ) where
        T: Serialize + Sync,
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

    async fn cache_value_expire_tomorrow<T>(
        &mut self,
        key: &str,
        value: &T,
        dont_cache: Option<bool>,
    ) where
        T: Serialize + Sync,
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
                        TTL_FALLBACK
                    }
                }
            }
            None => TTL_FALLBACK,
        };

        self.cache_value_expire_at(key, value, expire_at, dont_cache)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    struct Cacher {
        connection_manager: redis::aio::ConnectionManager,
    }
    impl Cache for Cacher {
        fn get_connection_manager(&mut self) -> &mut redis::aio::ConnectionManager {
            &mut self.connection_manager
        }
    }

    async fn del(key: &str) {
        let config = Config::default();
        let redis = Redis::new(config.db.redis).await;
        let mut connection = redis.client.get_connection().unwrap();
        redis::cmd("DEL")
            .arg(key)
            .query::<()>(&mut connection)
            .unwrap();
    }

    #[tokio::test]
    async fn test_redis_cache() {
        let key = "test_redis_cache";
        del(key).await;

        let config = Config::default();
        let redis = Redis::new(config.db.redis).await;

        let expected = 420;
        let expire = 10;
        let expected_expire_higher = expire;
        let expected_expire_lower = expected_expire_higher - 5;

        let mut cacher = Cacher {
            connection_manager: redis.connection_manager.clone(),
        };
        cacher
            .cache_value_expire(key, &expected, expire, None)
            .await;

        let actual: usize = cacher.get_cached(key, None).await.unwrap();
        assert_eq!(actual, expected);

        let actual_expire: usize = redis::cmd("TTL")
            .arg(key)
            .query(&mut redis.client.get_connection().unwrap())
            .unwrap();
        assert!(actual_expire >= expected_expire_lower && actual_expire <= expected_expire_higher);
    }

    #[tokio::test]
    async fn test_redis_cache_at() {
        let key = "test_redis_cache_at";
        del(key).await;

        let config = Config::default();
        let redis = Redis::new(config.db.redis).await;

        let expected = 420;
        let expire_at =
            usize::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap() + 10;
        let expected_expire_higher = 10;
        let expected_expire_lower = expected_expire_higher - 5;

        let mut cacher = Cacher {
            connection_manager: redis.connection_manager.clone(),
        };
        cacher
            .cache_value_expire_at(key, &expected, expire_at, None)
            .await;

        let actual: usize = cacher.get_cached(key, None).await.unwrap();
        assert_eq!(actual, expected);

        let actual_expire: usize = redis::cmd("TTL")
            .arg(key)
            .query(&mut redis.client.get_connection().unwrap())
            .unwrap();
        assert!(actual_expire >= expected_expire_lower && actual_expire <= expected_expire_higher);
    }

    #[tokio::test]
    async fn test_redis_cache_tomorrow() {
        let key = "test_redis_cache_tomorrow";
        del(key).await;

        let config = Config::default();
        let redis = Redis::new(config.db.redis).await;

        let now = time::OffsetDateTime::now_utc();
        let day = now.day();
        let tomorrow = now
            .replace_day(day + 1)
            .unwrap()
            .replace_time(Time::MIDNIGHT);

        let expected = 420;
        let expected_expire_higher = usize::try_from(tomorrow.unix_timestamp()).unwrap()
            - usize::try_from(now.unix_timestamp()).unwrap();
        let expected_expire_lower = expected_expire_higher - 5;

        let mut cacher = Cacher {
            connection_manager: redis.connection_manager.clone(),
        };
        cacher
            .cache_value_expire_tomorrow(key, &expected, None)
            .await;

        let actual: usize = cacher.get_cached(key, None).await.unwrap();
        assert_eq!(actual, expected);

        let actual_expire: usize = redis::cmd("TTL")
            .arg(key)
            .query(&mut redis.client.get_connection().unwrap())
            .unwrap();
        assert!(actual_expire >= expected_expire_lower && actual_expire <= expected_expire_higher);
    }
}
