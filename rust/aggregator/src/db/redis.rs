use crate::config::Config;

pub struct Redis<'a> {
    pub config: &'a Config,
    pub client: redis::Client,
}

impl Redis<'_> {
    pub fn new(config: &Config) -> Redis {
        let client = redis::Client::open(config.db.redis.uri.as_str()).unwrap();

        Redis { client, config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use redis::Commands;

    #[test]
    fn test_new() {
        let config = Config::default();
        let redis = Redis::new(&config);
        let mut connection = redis.client.get_connection().unwrap();
        let expected = "test";
        connection
            .set_ex::<&str, &str, ()>("test", expected, 5)
            .unwrap();
        let actual: String = connection.get("test").unwrap();
        assert_eq!(expected, actual);
    }
}
