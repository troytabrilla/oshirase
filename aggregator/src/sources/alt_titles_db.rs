use crate::anilist_api::Media;
use crate::config::Config;
use crate::db::Document;
use crate::sources::{ExtractOptions, Source};
use crate::transform::Transform;
use crate::Result;

use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Hash)]
pub struct AltTitlesEntry {
    pub media_id: u64,
    pub alt_titles: Vec<String>,
}

impl Document for AltTitlesEntry {}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AltTitles(pub HashMap<String, AltTitlesEntry>);

pub struct AltTitlesDB<'a> {
    config: &'a Config,
    mongodb: mongodb::Client,
}

impl AltTitlesDB<'_> {
    pub fn new(config: &Config, mongodb: mongodb::Client) -> AltTitlesDB {
        AltTitlesDB { config, mongodb }
    }
}

#[async_trait]
impl Source<'_> for AltTitlesDB<'_> {
    type Data = AltTitles;

    async fn extract(&mut self, _options: Option<&ExtractOptions>) -> Result<Self::Data> {
        let collection = self
            .mongodb
            .database(&self.config.db.mongodb.database)
            .collection::<AltTitlesEntry>("alt_titles");

        let mut cursor = collection.find(None, None).await?;
        let mut alt_titles = AltTitles(HashMap::new());

        while let Some(item) = cursor.next().await {
            match item {
                Ok(alt_title) => {
                    alt_titles
                        .0
                        .insert(alt_title.media_id.to_string(), alt_title.clone());
                }
                Err(err) => {
                    eprintln!("Could not get alt title entry: {}", err);
                }
            }
        }

        Ok(alt_titles)
    }
}

impl Transform for AltTitlesDB<'_> {
    type Extra = AltTitlesEntry;

    fn get_similarity_threshold(&self) -> f64 {
        1.0
    }

    fn set_media(media: &mut Media, extra: Option<Self::Extra>) {
        media.alt_titles = extra;
    }

    fn transform(&self, media: &mut Media, extra: &HashMap<String, Self::Extra>) -> Result<Media> {
        if media.status == Some("CURRENT".to_string()) {
            let media_id = match media.media_id {
                Some(media_id) => media_id.to_string(),
                None => return Ok(std::mem::take(media)),
            };

            if extra.contains_key(&media_id) {
                Self::set_media(media, extra.get(&media_id).cloned());
                return Ok(std::mem::take(media));
            }
        }

        Ok(std::mem::take(media))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::db::DB;
    use crate::test::helpers::{init, ONCE};

    use bson::doc;

    #[tokio::test]
    async fn test_extract() {
        ONCE.get_or_init(init).await;

        let config = Config::default();
        let db = DB::new(&config).await;

        let client = db.mongodb.client;
        let collection = client
            .database(&config.db.mongodb.database)
            .collection("alt_titles");
        let expected = vec!["alt", "title"];
        collection
            .insert_one(doc! { "media_id": 1, "alt_titles": &expected }, None)
            .await
            .unwrap();

        let mut alt_titles_db = AltTitlesDB::new(&config, client);
        let actual = alt_titles_db.extract(None).await.unwrap();

        println!("{:#?}", actual);
        assert_eq!(actual.0.len(), 1);
        assert_eq!(actual.0.get("1").unwrap().media_id, 1);
        assert_eq!(actual.0.get("1").unwrap().alt_titles, expected);
    }

    #[tokio::test]
    async fn test_transform() {
        let mut media = [Media {
            media_id: Some(1),
            status: Some("CURRENT".to_owned()),
            title: Some("Gintama".to_owned()),
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
        }];
        let alt_titles = HashMap::from([(
            1.to_string(),
            AltTitlesEntry {
                media_id: 1,
                alt_titles: vec!["Gin Tama".to_owned()],
            },
        )]);

        let config = Config::default();
        let db = DB::new(&config).await;
        let alt_title_db = AltTitlesDB::new(&config, db.mongodb.client);

        let transformed = alt_title_db.transform(&mut media[0], &alt_titles).unwrap();
        assert_eq!(transformed.alt_titles, alt_titles.get("1").cloned());
    }
}
