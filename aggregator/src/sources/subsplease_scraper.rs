use crate::config::{Config, SubsPleaseScraperConfig};
use crate::sources::Source;
use crate::Result;

use async_trait::async_trait;
use headless_chrome::Browser;
use scraper::{Html, Selector};

pub struct SubsPleaseScraper {
    config: SubsPleaseScraperConfig,
}

impl Default for SubsPleaseScraper {
    fn default() -> SubsPleaseScraper {
        let config = Config::default();

        SubsPleaseScraper::new(config.subsplease_scraper)
    }
}

impl SubsPleaseScraper {
    pub fn new(config: SubsPleaseScraperConfig) -> SubsPleaseScraper {
        SubsPleaseScraper { config }
    }
}

#[derive(Debug, Clone)]
pub struct Anime {
    pub title: String,
    pub time: String,
}

#[derive(Debug, Clone)]
pub struct Day {
    pub name: String,
    pub anime: Vec<Anime>,
}

#[derive(Debug)]
pub struct SubsPleaseSchedule {
    pub days: Vec<Day>,
}

impl SubsPleaseScraper {
    async fn scrape(&self) -> Result<SubsPleaseSchedule> {
        let browser = Browser::default()?;
        let tab = browser.new_tab()?;

        tab.navigate_to(&self.config.url)?
            .wait_until_navigated()?
            .wait_for_element(".day-of-week")?;

        let table = tab.find_element("#full-schedule-table")?.get_content()?;
        let table = Html::parse_fragment(&table);

        let tr = Selector::parse("tr")?;

        let mut days: Vec<Day> = Vec::new();

        for element in table.select(&tr) {
            if let Some(class) = element.value().attr("class") {
                if class == "day-of-week" {
                    let h2 = Selector::parse("h2")?;
                    let h2 = element.select(&h2).next();
                    if let Some(h2) = h2 {
                        days.push(Day {
                            name: h2.inner_html().to_owned(),
                            anime: Vec::new(),
                        });
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

                    if !title.is_empty() && !time.is_empty() && days.last().is_some() {
                        let last = days.len() - 1;
                        let day = &mut days[last];
                        day.anime.push(Anime { title, time });
                    }
                }
            }
        }

        Ok(SubsPleaseSchedule { days })
    }
}

#[async_trait]
impl Source for SubsPleaseScraper {
    type Data = SubsPleaseSchedule;

    async fn extract(&self) -> Result<SubsPleaseSchedule> {
        self.scrape().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn test_subsplease_scraper_new() {
        let scraper: SubsPleaseScraper = SubsPleaseScraper {
            config: SubsPleaseScraperConfig {
                url: "url".to_owned(),
            },
        };
        assert_eq!(scraper.config.url, "url");
    }

    #[test]
    fn test_subsplease_scraper_default() {
        let scraper: SubsPleaseScraper = SubsPleaseScraper::default();
        assert_eq!(scraper.config.url, "https://subsplease.org/schedule/");
    }

    #[tokio::test]
    async fn test_subsplease_scraper_scrape() {
        let scraper = SubsPleaseScraper::default();
        let actual = scraper.scrape().await.unwrap();
        assert!(!actual.days.is_empty());
    }

    #[tokio::test]
    async fn test_subsplease_scraper_extract() {
        let api = SubsPleaseScraper::default();
        let actual = api.extract().await.unwrap();
        assert!(!actual.days.is_empty());
    }
}
