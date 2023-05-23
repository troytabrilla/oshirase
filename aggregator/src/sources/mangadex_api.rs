use crate::config::Config;
use crate::error::CustomError;
use crate::options::ExtractOptions;
use crate::result::Result;
use crate::sources::anilist_api::{Latest, Media, MediaType};
use crate::sources::{Extract, Similar, Transform};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{
    task::JoinSet,
    time::{sleep, Duration},
};

#[derive(Debug, Deserialize)]
struct MangaListAttributes {
    title: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct MangaListRelationship {
    id: String,
    r#type: String,
    attributes: Option<MangaListAttributes>,
}

#[derive(Debug, Deserialize)]
struct MangaListData {
    relationships: Vec<MangaListRelationship>,
}

#[derive(Debug, Deserialize)]
struct MangaList {
    result: String,
    data: MangaListData,
}

#[derive(Debug, Deserialize)]
struct MangaAggregateChapter {
    id: String,
}

#[derive(Debug, Deserialize)]
struct MangaAggregateVolumes {
    chapters: HashMap<String, MangaAggregateChapter>,
}

#[derive(Debug, Deserialize)]
struct MangaAggregate {
    result: String,
    volumes: HashMap<String, MangaAggregateVolumes>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MangaLatest(pub HashMap<String, Latest>);

#[derive(Debug, Clone)]
pub struct MangaDexAPI<'a> {
    pub config: &'a Config,
}

impl MangaDexAPI<'_> {
    pub fn new(config: &Config) -> MangaDexAPI {
        MangaDexAPI { config }
    }

    pub async fn fetch(&self) -> Result<MangaLatest> {
        let client = reqwest::Client::new();
        let results = client
            .get(self.config.mangadex_api.url.as_str())
            .send()
            .await?
            .json::<MangaList>()
            .await?;

        let mut manga_latest: HashMap<String, Latest> = HashMap::new();
        let mut batches: Vec<Vec<(String, String)>> = Vec::new();
        let mut current_batch: Vec<(String, String)> = Vec::new();

        if results.result != "ok" {
            return Err(CustomError::boxed("Could not fetch manga list."));
        }

        for rel in results.data.relationships {
            if rel.r#type == "manga" {
                if let Some(attributes) = rel.attributes {
                    let id = rel.id;

                    let title = if let Some(ro_title) = attributes.title.get("ja-ro") {
                        ro_title.to_owned()
                    } else if let Some(en_title) = attributes.title.get("en") {
                        en_title.to_owned()
                    } else {
                        String::new()
                    };

                    if !title.is_empty() {
                        if current_batch.len() < self.config.mangadex_api.rate_limit {
                            current_batch.push((title.to_owned(), id.to_owned()));
                        } else {
                            batches.push(current_batch);
                            current_batch = vec![(title.to_owned(), id.to_owned())];
                        }
                    }
                }
            }
        }

        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        for batch in batches {
            let mut futures = JoinSet::new();

            for (title, id) in batch {
                let url = self.config.mangadex_api.manga_agg_url.replace("{id}", &id);
                let builder = client.get(url);
                let future = || async {
                    let res = builder.send().await;
                    (title, res)
                };
                futures.spawn(async move { future().await });
            }

            while let Some(future) = futures.join_next().await {
                let (title, res) = future?;
                let manga_agg = res?.json::<MangaAggregate>().await?;

                let mut latest: (u64, String) = (0, String::new());
                if manga_agg.result == "ok" {
                    for volume in manga_agg.volumes.iter() {
                        let chapters = &volume.1.chapters;

                        for manga_chapter in chapters {
                            let chapter = manga_chapter.0.parse::<f64>();
                            match chapter {
                                Ok(chapter) => {
                                    let chapter = chapter as u64;
                                    if chapter >= latest.0 {
                                        latest = (
                                            chapter,
                                            format!(
                                                "https://mangadex.org/chapter/{}",
                                                manga_chapter.1.id.to_owned()
                                            ),
                                        );
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Could not parse chapter: {}", err);
                                }
                            }
                        }
                    }
                }

                manga_latest.insert(
                    title.to_owned(),
                    Latest {
                        title: title.to_owned(),
                        episode: latest.0,
                        url: latest.1,
                    },
                );
            }

            // Rate limited to 5 requests per second
            sleep(Duration::from_secs(1)).await;
        }

        Ok(MangaLatest(manga_latest))
    }
}

#[async_trait]
impl Extract<'_> for MangaDexAPI<'_> {
    type Data = MangaLatest;

    async fn extract(&self, _options: Option<ExtractOptions>) -> Result<Self::Data> {
        let manga_latest = self.fetch().await?;

        Ok(manga_latest)
    }
}

impl Transform for MangaDexAPI<'_> {
    type Extra = Latest;

    fn set_media(mut media: &mut Media, extra: Option<Self::Extra>) {
        media.latest = extra;
    }

    fn transform(&self, media: &mut Media, extras: &HashMap<String, Self::Extra>) -> Result<Media> {
        self.match_similar(media, MediaType::Manga, extras)
    }
}

impl Similar for MangaDexAPI<'_> {
    fn get_similarity_threshold(&self) -> f64 {
        self.config.transform.similarity_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract() {
        let config = Config::default();
        let mangadex_api = MangaDexAPI::new(&config);
        let latest = mangadex_api.extract(None).await.unwrap();
        assert!(!latest.0.is_empty());
    }

    #[test]
    fn test_transform() {
        let mut media = [Media {
            media_id: Some(1),
            status: Some("CURRENT".to_owned()),
            title: Some("Gintama".to_owned()),
            english_title: Some("Gin Tama".to_owned()),
            media_type: Some(MediaType::Manga),
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
                url: "http://www.test.nyaa".to_owned(),
            },
        )]);

        let config = Config::default();
        let subsplease_rss = MangaDexAPI::new(&config);

        let transformed = subsplease_rss.transform(&mut media[0], &latest).unwrap();
        assert_eq!(transformed.latest, latest.get("gintama").cloned());
    }
}
