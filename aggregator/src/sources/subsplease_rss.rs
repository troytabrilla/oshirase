use crate::anilist_api::{Latest, Media};
use crate::config::Config;
use crate::options::ExtractOptions;
use crate::result::Result;
use crate::sources::{Extract, Similar, Transform};

use async_trait::async_trait;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use serde_xml_rs::from_str;
use std::{collections::HashMap, hash::Hash};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Clone, Hash)]
struct PubDate(OffsetDateTime);

struct PubDateVisitor;

impl<'de> Visitor<'de> for PubDateVisitor {
    type Value = PubDate;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an OffsetDateTime with well-known format RFC2822"
        )
    }

    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        OffsetDateTime::parse(&v, &time::format_description::well_known::Rfc2822)
            .map(PubDate)
            .map_err(serde::de::Error::custom)
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        OffsetDateTime::parse(v, &time::format_description::well_known::Rfc2822)
            .map(PubDate)
            .map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for PubDate {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(PubDateVisitor)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash)]
pub struct AnimeRss {
    title: String,
    link: String,
    #[serde(rename = "pubDate")]
    pub_date: PubDate,
    category: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash)]
pub struct Channel {
    #[serde(rename = "item")]
    rss_items: Vec<AnimeRss>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AnimeLatest(pub HashMap<String, Latest>);

#[derive(Debug, Deserialize, Serialize, Clone, Hash)]
pub struct Rss {
    channel: Channel,
}

#[derive(Debug, Clone)]
pub struct SubsPleaseRSS<'a> {
    config: &'a Config,
}

impl SubsPleaseRSS<'_> {
    pub fn new(config: &Config) -> SubsPleaseRSS {
        SubsPleaseRSS { config }
    }

    pub async fn fetch(&self) -> Result<AnimeLatest> {
        let client = reqwest::Client::new();
        let xml = client
            .get(self.config.subsplease.rss.url.as_str())
            .send()
            .await?
            .text()
            .await?;

        let rss: Rss = from_str(&xml)?;

        let mut latest: HashMap<String, Latest> = HashMap::new();

        for item in &rss.channel.rss_items {
            let title = str::replace(&item.category, " - 720", "");
            let re = regex::Regex::new(r#"(?P<episode>\d+) \(720p\)"#)?;
            let caps = re.captures(&item.title);
            let episode = match caps {
                Some(caps) => {
                    let episode = &caps["episode"];
                    let episode = episode.parse::<u64>();
                    match episode {
                        Ok(episode) => episode,
                        Err(err) => {
                            eprintln!("Could not parse episode number: {}", err);
                            0
                        }
                    }
                }
                None => 0,
            };

            if !latest.contains_key(&title) || latest.get(&title).unwrap().episode < episode {
                latest.insert(
                    title.clone(),
                    Latest {
                        title,
                        episode,
                        url: item.link.clone(),
                    },
                );
            }
        }

        Ok(AnimeLatest(latest))
    }
}

#[async_trait]
impl Extract<'_> for SubsPleaseRSS<'_> {
    type Data = AnimeLatest;

    async fn extract(&self, _options: Option<ExtractOptions>) -> Result<Self::Data> {
        let latest = self.fetch().await?;

        Ok(latest)
    }
}

impl Transform for SubsPleaseRSS<'_> {
    type Extra = Latest;

    fn set_media(media: &mut Media, extra: Option<Self::Extra>) {
        media.latest = extra;
    }

    fn transform(&self, media: &mut Media, extras: &HashMap<String, Self::Extra>) -> Result<Media> {
        self.match_similar(media, extras)
    }
}

impl Similar for SubsPleaseRSS<'_> {
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
        let rss = SubsPleaseRSS::new(&config);
        let latest = rss.extract(None).await.unwrap();
        println!("{:#?}", latest);
        assert!(!latest.0.is_empty());
    }

    #[test]
    fn test_transform() {
        let mut media = [Media {
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
        }];
        let latest = HashMap::from([(
            "gintama".to_owned(),
            Latest {
                title: "gintama".to_owned(),
                episode: 1,
                url: "http:://www.test.nyaa".to_owned(),
            },
        )]);

        let config = Config::default();
        let subsplease_rss = SubsPleaseRSS::new(&config);

        let transformed = subsplease_rss.transform(&mut media[0], &latest).unwrap();
        assert_eq!(transformed.latest, latest.get("gintama").cloned());
    }
}
