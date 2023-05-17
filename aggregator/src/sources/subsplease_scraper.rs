use crate::config::Config;
use crate::db::Cache;
use crate::sources::Source;
use crate::transform::Transform;
use crate::CustomError;
use crate::ExtractOptions;
use crate::Media;
use crate::Result;

use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash)]
pub enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl FromStr for Day {
    type Err = Box<CustomError>;

    fn from_str(day: &str) -> std::result::Result<Day, Self::Err> {
        match day {
            "Sunday" => Ok(Day::Sunday),
            "Monday" => Ok(Day::Monday),
            "Tuesday" => Ok(Day::Tuesday),
            "Wednesday" => Ok(Day::Wednesday),
            "Thursday" => Ok(Day::Thursday),
            "Friday" => Ok(Day::Friday),
            "Saturday" => Ok(Day::Saturday),
            _ => Err(CustomError::boxed("Invalid day.")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash)]
pub struct AnimeScheduleEntry {
    pub title: String,
    pub day: Day,
    pub time: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnimeSchedule(pub HashMap<String, AnimeScheduleEntry>);

pub struct SubsPleaseScraper<'a> {
    config: &'a Config,
    redis: redis::aio::ConnectionManager,
}

impl SubsPleaseScraper<'_> {
    pub fn new(config: &Config, redis: redis::aio::ConnectionManager) -> SubsPleaseScraper {
        SubsPleaseScraper { config, redis }
    }

    async fn load_schedule_table(&self) -> Result<Html> {
        let mut caps = serde_json::map::Map::new();
        let chrome_opts: Vec<&str> = self
            .config
            .subsplease_scraper
            .chrome_options
            .split_whitespace()
            .collect();
        let chrome_opts = serde_json::json!({ "args": chrome_opts });
        caps.insert("goog:chromeOptions".to_string(), chrome_opts);

        let client = fantoccini::ClientBuilder::native()
            .capabilities(caps)
            .connect(&self.config.subsplease_scraper.webdriver_url)
            .await?;
        client.goto(&self.config.subsplease_scraper.url).await?;
        let locator = fantoccini::Locator::Css(".day-of-week");
        let table = client
            .wait()
            .for_element(locator)
            .await?
            .find(fantoccini::Locator::Id("full-schedule-table"))
            .await?
            .html(false)
            .await?;
        let table = Html::parse_fragment(&table);

        Ok(table)
    }

    fn extract_inner_html(selector: &str, element: ElementRef) -> String {
        let selector = Selector::parse(selector);
        match selector {
            Ok(selector) => match element.select(&selector).next() {
                Some(elem) => elem.inner_html(),
                None => String::new(),
            },
            Err(err) => {
                eprintln!("Could not parse selector: {}", err);
                String::new()
            }
        }
    }

    async fn scrape(&self) -> Result<AnimeSchedule> {
        let table = self.load_schedule_table().await?;

        let mut days: AnimeSchedule = AnimeSchedule(HashMap::new());
        let mut current_day: Option<Day> = None;

        let tr = Selector::parse("tr");

        let tr = match tr {
            Ok(tr) => tr,
            Err(err) => {
                eprintln!("Could not parse selector: {}", err);
                return Err(CustomError::boxed("Could not parse selector."));
            }
        };

        for element in table.select(&tr) {
            if let Some(class) = element.value().attr("class") {
                if class == "day-of-week" {
                    let day = Self::extract_inner_html("h2", element);
                    current_day = Day::from_str(&day).ok();
                } else if class == "all-schedule-item" {
                    let title = Self::extract_inner_html("a", element);
                    let time = Self::extract_inner_html(".all-schedule-time", element);

                    if !title.is_empty() && !time.is_empty() && current_day.is_some() {
                        days.0.insert(
                            title.to_owned(),
                            AnimeScheduleEntry {
                                title,
                                time,
                                day: current_day.clone().unwrap(),
                            },
                        );
                    }
                }
            }
        }

        Ok(days)
    }
}

#[async_trait]
impl Cache for SubsPleaseScraper<'_> {
    fn get_connection_manager(&mut self) -> &mut redis::aio::ConnectionManager {
        &mut self.redis
    }
}

#[async_trait]
impl Source<'_> for SubsPleaseScraper<'_> {
    type Data = AnimeSchedule;

    async fn extract(&mut self, options: Option<&ExtractOptions>) -> Result<Self::Data> {
        let cache_key = "subsplease_scraper:extract";

        let skip_cache = match options {
            Some(options) => options.skip_cache.unwrap_or(false),
            None => false,
        };

        if let Some(cached) = self.get_cached(cache_key, Some(skip_cache)).await {
            println!("Got cached value for cache key: {}.", cache_key);
            return Ok(cached);
        }

        let data = self.scrape().await?;

        self.cache_value_expire_tomorrow(cache_key, &data).await;

        Ok(data)
    }
}

impl Transform for SubsPleaseScraper<'_> {
    type Extra = AnimeScheduleEntry;

    fn get_similarity_threshold(&self) -> f64 {
        self.config.transform.similarity_threshold
    }

    fn set_media(mut media: &mut Media, extra: Option<Self::Extra>) {
        media.schedule = extra;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::db::DB;
    use crate::ExtractOptions;

    #[tokio::test]
    async fn test_scrape() {
        let config = Config::default();
        let db = DB::new(&config).await;
        let scraper = SubsPleaseScraper::new(&config, db.redis.connection_manager);
        let actual = scraper.scrape().await.unwrap();
        assert!(!actual.0.is_empty());
    }

    #[tokio::test]
    async fn test_extract() {
        let config = Config::default();
        let db = DB::new(&config).await;
        let mut scraper = SubsPleaseScraper::new(&config, db.redis.connection_manager);
        let options = ExtractOptions {
            skip_cache: Some(true),
        };
        let actual = scraper.extract(Some(&options)).await.unwrap();
        assert!(!actual.0.is_empty());
    }

    #[tokio::test]
    async fn test_transform() {
        let media = [
            Media {
                media_id: Some(1),
                status: Some("CURRENT".to_owned()),
                title: Some("Gintama".to_owned()),
                alt_title: Some("Gin Tama".to_owned()),
                media_type: None,
                format: None,
                season: None,
                season_year: None,
                image: None,
                episodes: None,
                score: None,
                progress: None,
                latest: None,
                schedule: None,
            },
            Media {
                media_id: Some(1),
                status: Some("CURRENT".to_owned()),
                title: Some("naruto".to_owned()),
                alt_title: None,
                media_type: None,
                format: None,
                season: None,
                season_year: None,
                image: None,
                episodes: None,
                score: None,
                progress: None,
                latest: None,
                schedule: None,
            },
            Media {
                media_id: Some(1),
                status: Some("CURRENT".to_owned()),
                title: None,
                alt_title: Some("tamako market".to_owned()),
                media_type: None,
                format: None,
                season: None,
                season_year: None,
                image: None,
                episodes: None,
                score: None,
                progress: None,
                latest: None,
                schedule: None,
            },
        ];
        let schedules = HashMap::from([
            (
                "gintama".to_owned(),
                AnimeScheduleEntry {
                    title: "gintama".to_owned(),
                    day: Day::Saturday,
                    time: "00:00".to_owned(),
                },
            ),
            (
                "naruto".to_owned(),
                AnimeScheduleEntry {
                    title: "naruto".to_owned(),
                    day: Day::Monday,
                    time: "00:00".to_owned(),
                },
            ),
            (
                "tamako market".to_owned(),
                AnimeScheduleEntry {
                    title: "tamako market".to_owned(),
                    day: Day::Friday,
                    time: "00:00".to_owned(),
                },
            ),
        ]);

        let config = Config::default();
        let db = DB::new(&config).await;
        let subsplease_scraper = SubsPleaseScraper::new(&config, db.redis.connection_manager);

        let transformed = subsplease_scraper
            .transform(media[0].clone(), &schedules)
            .unwrap();
        assert_eq!(transformed.schedule, schedules.get("gintama").cloned());

        let transformed = subsplease_scraper
            .transform(media[1].clone(), &schedules)
            .unwrap();
        assert_eq!(transformed.schedule, schedules.get("naruto").cloned());

        let transformed = subsplease_scraper
            .transform(media[2].clone(), &schedules)
            .unwrap();
        assert_eq!(
            transformed.schedule,
            schedules.get("tamako market").cloned()
        );
    }
}
