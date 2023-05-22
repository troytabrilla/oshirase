pub mod alt_titles_db;
pub mod anilist_api;
pub mod mangadex_api;
pub mod subsplease_rss;
pub mod subsplease_scraper;

use crate::anilist_api::Media;
use crate::options::ExtractOptions;
use crate::result::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, hash::Hash};

pub trait Document: DeserializeOwned + Serialize + Hash + Unpin + Send + Sync {}

pub struct Sources<'a> {
    pub anilist_api: anilist_api::AniListAPI<'a>,
    pub subsplease_scraper: subsplease_scraper::SubsPleaseScraper<'a>,
    pub subsplease_rss: subsplease_rss::SubsPleaseRSS<'a>,
    pub alt_titles_db: alt_titles_db::AltTitlesDB<'a>,
}

// @todo Add mangadex api source
pub enum Extras<'a> {
    SubsPleaseScraper(subsplease_scraper::SubsPleaseScraper<'a>),
    SubsPleaseRSS(subsplease_rss::SubsPleaseRSS<'a>),
}

#[async_trait]
pub trait Extract<'a> {
    type Data: Serialize;

    async fn extract(&self, options: Option<ExtractOptions>) -> Result<Self::Data>;
}

pub trait Transform {
    type Extra: Clone;

    fn set_media(media: &mut Media, extra: Option<Self::Extra>);

    fn transform(&self, media: &mut Media, extras: &HashMap<String, Self::Extra>) -> Result<Media>;
}

pub trait Similar: Transform {
    fn get_similarity_threshold(&self) -> f64;

    fn match_similar(
        &self,
        media: &mut Media,
        extras: &HashMap<String, Self::Extra>,
    ) -> Result<Media> {
        if media.status == Some("CURRENT".to_string()) {
            let title = match media.title.to_owned() {
                Some(title) => title,
                None => String::new(),
            };
            let english_title = match media.english_title.to_owned() {
                Some(english_title) => english_title,
                None => String::new(),
            };
            let empty_vec = Vec::new();
            let alt_titles = match &media.alt_titles {
                Some(alt_titles) => &alt_titles.alt_titles,
                None => &empty_vec,
            };

            if extras.contains_key(&title) {
                Self::set_media(media, extras.get(&title).cloned());
                return Ok(std::mem::take(media));
            }

            if extras.contains_key(&english_title) {
                Self::set_media(media, extras.get(&english_title).cloned());
                return Ok(std::mem::take(media));
            }

            for alt_title in alt_titles {
                if extras.contains_key(alt_title) {
                    Self::set_media(media, extras.get(alt_title).cloned());
                    return Ok(std::mem::take(media));
                }
            }

            let mut score_tuple: (f64, Option<&Self::Extra>) = (-f64::INFINITY, None);
            for (ex_title, ex) in extras {
                let score = strsim::normalized_levenshtein(&title, ex_title);
                let alt_score = strsim::normalized_levenshtein(&english_title, ex_title);
                if score > self.get_similarity_threshold() && score > score_tuple.0 {
                    score_tuple = (score, Some(ex));
                }
                if alt_score > self.get_similarity_threshold() && alt_score > score_tuple.0 {
                    score_tuple = (alt_score, Some(ex));
                }
            }

            if score_tuple.1.is_some() {
                Self::set_media(media, score_tuple.1.cloned());
            }
        }

        Ok(std::mem::take(media))
    }
}
