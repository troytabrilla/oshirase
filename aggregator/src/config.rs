use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct AggregatorConfig {
    pub ttl: usize,
}

#[derive(Debug, Deserialize)]
pub struct AniListAPIConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct MongoDBConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct DBConfig {
    pub mongodb: MongoDBConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Deserialize)]
pub struct MangaDexAPIConfig {
    pub url: String,
    pub manga_agg_url: String,
    pub rate_limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct TransformConfig {
    pub similarity_threshold: f64,
}

#[derive(Debug, Deserialize)]
pub struct SubsPleaseRSSConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct SubsPleaseScraperConfig {
    pub url: String,
    pub webdriver_url: String,
    pub chrome_options: String,
}

#[derive(Debug, Deserialize)]
pub struct SubsPleaseConfig {
    pub rss: SubsPleaseRSSConfig,
    pub scraper: SubsPleaseScraperConfig,
}

#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    pub retry_timeout: usize,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub aggregator: AggregatorConfig,
    pub anilist_api: AniListAPIConfig,
    pub db: DBConfig,
    pub mangadex_api: MangaDexAPIConfig,
    pub subsplease: SubsPleaseConfig,
    pub transform: TransformConfig,
    pub worker: WorkerConfig,
}

impl Config {
    pub fn from_file(filename: &str) -> Config {
        let config = fs::read_to_string(filename).unwrap();
        let config: Config = toml::from_str(&config).unwrap();
        config
    }
}

impl Default for Config {
    fn default() -> Config {
        Self::from_file("config/config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_file() {
        let config = Config::from_file("config/config.toml");
        assert_eq!(config.aggregator.ttl, 600);
    }

    #[test]
    #[should_panic]
    fn test_from_file_failure() {
        Config::from_file("should_fail.toml");
    }
}
