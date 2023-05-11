use crate::config::SubsPleaseScraperConfig;
use crate::db::DB;
use crate::sources::Source;
use crate::CustomError;
use crate::Result;

use async_trait::async_trait;
use headless_chrome::Browser;
use scraper::{Html, Selector};
use std::{error::Error, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

pub struct SubsPleaseScraper {
    config: SubsPleaseScraperConfig,
    db: Arc<Mutex<DB>>,
}

impl SubsPleaseScraper {
    pub fn new(config: &SubsPleaseScraperConfig, db: Arc<Mutex<DB>>) -> SubsPleaseScraper {
        SubsPleaseScraper {
            config: config.clone(),
            db,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct AnimeSchedule {
    pub title: String,
    pub day: Day,
    pub time: String,
}

impl SubsPleaseScraper {
    async fn scrape(&self) -> Result<Vec<AnimeSchedule>> {
        let browser = Browser::default()?;
        let tab = browser.new_tab()?;

        tab.navigate_to(&self.config.url)?
            .wait_until_navigated()?
            .wait_for_element(".day-of-week")?;

        let table = tab.find_element("#full-schedule-table")?.get_content()?;
        let table = Html::parse_fragment(&table);

        let tr = Selector::parse("tr")?;

        let mut days: Vec<AnimeSchedule> = Vec::new();
        let mut current_day: Option<Day> = None;

        for element in table.select(&tr) {
            if let Some(class) = element.value().attr("class") {
                if class == "day-of-week" {
                    let h2 = Selector::parse("h2")?;
                    let h2 = element.select(&h2).next();
                    if let Some(h2) = h2 {
                        current_day = Day::from_str(&h2.inner_html()).ok();
                    }
                } else if class == "all-schedule-item" {
                    let a = Selector::parse("a")?;
                    let a = element.select(&a).next();

                    let title = match a {
                        Some(a) => a.inner_html(),
                        None => String::new(),
                    };

                    let td = Selector::parse(".all-schedule-time")?;
                    let td = element.select(&td).next();

                    let time = match td {
                        Some(td) => td.inner_html(),
                        None => String::new(),
                    };

                    if !title.is_empty() && !time.is_empty() && current_day.is_some() {
                        days.push(AnimeSchedule {
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
    type Data = Vec<AnimeSchedule>;

    async fn extract(&mut self) -> Result<Vec<AnimeSchedule>> {
        // @todo Add caching (1 day)
        // @todo Add option to skip cache
        self.scrape().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[tokio::test]
    async fn test_subsplease_scraper_new() {
        let db = Arc::new(Mutex::new(DB::default()));
        let scraper: SubsPleaseScraper = SubsPleaseScraper {
            config: SubsPleaseScraperConfig {
                url: "url".to_owned(),
            },
            db,
        };
        assert_eq!(scraper.config.url, "url");
    }

    #[tokio::test]
    async fn test_subsplease_scraper_default() {
        let config = Config::default();
        let db = Arc::new(Mutex::new(DB::default()));
        let scraper: SubsPleaseScraper = SubsPleaseScraper::new(&config.subsplease_scraper, db);
        assert_eq!(scraper.config.url, "https://subsplease.org/schedule/");
    }

    #[tokio::test]
    async fn test_subsplease_scraper_scrape() {
        let config = Config::default();
        let db = Arc::new(Mutex::new(DB::default()));
        let scraper = SubsPleaseScraper::new(&config.subsplease_scraper, db);
        let actual = scraper.scrape().await.unwrap();
        assert!(!actual.is_empty());
    }

    #[tokio::test]
    async fn test_subsplease_scraper_extract() {
        let config = Config::default();
        let db = Arc::new(Mutex::new(DB::default()));
        let mut scraper = SubsPleaseScraper::new(&config.subsplease_scraper, db);
        let actual = scraper.extract().await.unwrap();
        assert!(!actual.is_empty());
    }
}
