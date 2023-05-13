use crate::config::SubsPleaseScraperConfig;
use crate::db::Redis;
use crate::sources::Source;
use crate::CustomError;
use crate::ExtractOptions;
use crate::Result;

use async_trait::async_trait;
use headless_chrome::Browser;
use scraper::ElementRef;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{error::Error, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

pub struct SubsPleaseScraper {
    config: SubsPleaseScraperConfig,
    redis: Arc<Mutex<Redis>>,
}

impl SubsPleaseScraper {
    pub fn new(config: &SubsPleaseScraperConfig, redis: Arc<Mutex<Redis>>) -> SubsPleaseScraper {
        SubsPleaseScraper {
            config: config.clone(),
            redis,
        }
    }
}

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
    type Err = Box<dyn Error>;

    fn from_str(day: &str) -> Result<Day> {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimeSchedule(pub Vec<AnimeScheduleEntry>);

impl SubsPleaseScraper {
    fn load_schedule_table(&self) -> Result<Html> {
        let browser = Browser::default()?;
        let tab = browser.new_tab()?;

        tab.navigate_to(&self.config.url)?
            .wait_until_navigated()?
            .wait_for_element(".day-of-week")?;

        let table = tab.find_element("#full-schedule-table")?.get_content()?;
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
                println!("Could not parse selector: {}", err);
                String::new()
            }
        }
    }

    async fn scrape(&self) -> Result<AnimeSchedule> {
        let table = self.load_schedule_table()?;

        let mut days: AnimeSchedule = AnimeSchedule(Vec::new());
        let mut current_day: Option<Day> = None;

        let tr = Selector::parse("tr")?;

        for element in table.select(&tr) {
            if let Some(class) = element.value().attr("class") {
                if class == "day-of-week" {
                    let day = Self::extract_inner_html("h2", element);
                    current_day = Day::from_str(&day).ok();
                } else if class == "all-schedule-item" {
                    let title = Self::extract_inner_html("a", element);
                    let time = Self::extract_inner_html(".all-schedule-time", element);

                    if !title.is_empty() && !time.is_empty() && current_day.is_some() {
                        days.0.push(AnimeScheduleEntry {
                            title,
                            time,
                            day: current_day.clone().unwrap(),
                        });
                    }
                }
            }
        }

        Ok(days)
    }
}

#[async_trait]
impl Source for SubsPleaseScraper {
    type Data = AnimeSchedule;

    async fn extract(&mut self, options: Option<&ExtractOptions>) -> Result<Self::Data> {
        let cache_key = "subsplease_scraper:extract";

        let dont_cache = match options {
            Some(options) => options.dont_cache.unwrap_or(false),
            None => false,
        };

        let mut redis = self.redis.lock().await;

        if let Some(cached) = redis.get_cached(cache_key, Some(dont_cache)).await {
            println!("Got cached value for cache key: {}.", cache_key);
            return Ok(cached);
        }

        let data = self.scrape().await?;

        redis
            .cache_value_expire_tomorrow(cache_key, &data, Some(dont_cache))
            .await;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::ExtractOptions;

    #[tokio::test]
    async fn test_scrape() {
        let config = Config::default();
        let redis = Arc::new(Mutex::new(Redis::new(&config.db.redis).await));
        let scraper = SubsPleaseScraper::new(&config.subsplease_scraper, redis);
        let actual = scraper.scrape().await.unwrap();
        assert!(!actual.0.is_empty());
    }

    #[tokio::test]
    async fn test_extract() {
        let config = Config::default();
        let redis = Arc::new(Mutex::new(Redis::new(&config.db.redis).await));
        let mut scraper = SubsPleaseScraper::new(&config.subsplease_scraper, redis);
        let options = ExtractOptions {
            dont_cache: Some(true),
        };
        let actual = scraper.extract(Some(&options)).await.unwrap();
        assert!(!actual.0.is_empty());
    }
}
