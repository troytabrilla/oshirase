use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct AggregatorConfig {
    pub ttl: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AniListAPIAuthConfig {
    pub access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AniListAPIConfig {
    pub url: String,
    pub auth: AniListAPIAuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoDBConfig {
    pub host: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub ttl_fallback: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DBConfig {
    pub mongodb: MongoDBConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CombinerConfig {
    pub similarity_threshold: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubsPleaseScraperConfig {
    pub url: String,
    pub webdriver_url: String,
    pub chrome_options: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub aggregator: AggregatorConfig,
    pub anilist_api: AniListAPIConfig,
    pub db: DBConfig,
    pub combiner: CombinerConfig,
    pub subsplease_scraper: SubsPleaseScraperConfig,
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
