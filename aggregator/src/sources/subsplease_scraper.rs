use crate::anilist_api::Media;
use crate::config::Config;
use crate::error::CustomError;
use crate::result::Result;
use crate::sources::{Extract, ExtractOptions, Similar, Transform};

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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AnimeSchedule(pub HashMap<String, AnimeScheduleEntry>);

#[derive(Clone)]
pub struct SubsPleaseScraper<'a> {
    config: &'a Config,
}

impl SubsPleaseScraper<'_> {
    pub fn new(config: &Config) -> SubsPleaseScraper {
        SubsPleaseScraper { config }
    }

    async fn load_schedule_table(&self) -> Result<Html> {
        let mut caps = serde_json::map::Map::new();
        let chrome_opts: Vec<&str> = self
            .config
            .subsplease
            .scraper
            .chrome_options
            .split_whitespace()
            .collect();
        let chrome_opts = serde_json::json!({ "args": chrome_opts });
        caps.insert("goog:chromeOptions".to_string(), chrome_opts);

        let client = fantoccini::ClientBuilder::native()
            .capabilities(caps)
            .connect(&self.config.subsplease.scraper.webdriver_url)
            .await?;
        client.goto(&self.config.subsplease.scraper.url).await?;
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
impl Extract<'_> for SubsPleaseScraper<'_> {
    type Data = AnimeSchedule;

    async fn extract(&self, _options: Option<ExtractOptions>) -> Result<Self::Data> {
        let mut data = self.scrape().await?;

        Ok(std::mem::take(&mut data))
    }
}

impl Transform for SubsPleaseScraper<'_> {
    type Extra = AnimeScheduleEntry;

    fn set_media(mut media: &mut Media, extra: Option<Self::Extra>) {
        media.schedule = extra;
    }

    fn transform(&self, media: &mut Media, extras: &HashMap<String, Self::Extra>) -> Result<Media> {
        self.match_similar(media, extras)
    }
}

impl Similar for SubsPleaseScraper<'_> {
    fn get_similarity_threshold(&self) -> f64 {
        self.config.transform.similarity_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_extract() {
        let config = Config::default();
        let scraper = SubsPleaseScraper::new(&config);
        let actual = scraper.extract(None).await.unwrap();
        assert!(!actual.0.is_empty());
    }

    #[test]
    fn test_transform() {
        let mut media = [
            Media {
                media_id: Some(1),
                status: Some("CURRENT".to_owned()),
                title: Some("Gintama".to_owned()),
                english_title: Some("Gin Tama".to_owned()),
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
                alt_titles: None,
            },
            Media {
                media_id: Some(1),
                status: Some("CURRENT".to_owned()),
                title: Some("naruto".to_owned()),
                english_title: None,
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
                alt_titles: None,
            },
            Media {
                media_id: Some(1),
                status: Some("CURRENT".to_owned()),
                title: None,
                english_title: Some("tamako market".to_owned()),
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
                alt_titles: None,
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
        let subsplease_scraper = SubsPleaseScraper::new(&config);

        let transformed = subsplease_scraper
            .transform(&mut media[0], &schedules)
            .unwrap();
        assert_eq!(transformed.schedule, schedules.get("gintama").cloned());

        let transformed = subsplease_scraper
            .transform(&mut media[1], &schedules)
            .unwrap();
        assert_eq!(transformed.schedule, schedules.get("naruto").cloned());

        let transformed = subsplease_scraper
            .transform(&mut media[2], &schedules)
            .unwrap();
        assert_eq!(
            transformed.schedule,
            schedules.get("tamako market").cloned()
        );
    }
}
