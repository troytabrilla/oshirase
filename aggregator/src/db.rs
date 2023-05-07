use crate::config::Conf;
use mongodb::{
    options::{ClientOptions, ServerAddress},
    Client,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    mongodb_host: String,
}

pub struct MongoDB {
    pub client: Client,
}

impl Conf for MongoDB {
    type Config = Config;
}

impl Default for MongoDB {
    fn default() -> MongoDB {
        let config = Self::get_config("config/db.yaml").expect("Could not load db config.");
        let address = ServerAddress::parse(config.mongodb_host)
            .expect("Could not parse MongoDB host address.");
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name(String::from("oshirase-aggregator"))
            .build();
        let client = Client::with_options(options).expect("Could not create mongodb client.");
        MongoDB { client }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default() {
        let mongo = MongoDB::default();
        let actual = mongo.client.list_database_names(None, None).await.unwrap();
        let expected = vec!["admin", "config", "local"];
        assert_eq!(actual, expected);
    }
}
