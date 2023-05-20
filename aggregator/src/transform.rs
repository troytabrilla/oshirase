use crate::Media;
use crate::Result;

use std::collections::HashMap;

pub trait Transform {
    type Extra: Clone;

    fn get_similarity_threshold(&self) -> f64;

    fn set_media(media: &mut Media, extra: Option<Self::Extra>);

    fn transform(&self, media: &mut Media, extra: &HashMap<String, Self::Extra>) -> Result<Media> {
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

            if extra.contains_key(&title) {
                Self::set_media(media, extra.get(&title).cloned());
                return Ok(std::mem::take(media));
            }

            if extra.contains_key(&english_title) {
                Self::set_media(media, extra.get(&english_title).cloned());
                return Ok(std::mem::take(media));
            }

            for alt_title in alt_titles {
                println!("Alt Title {}", alt_title);
                if extra.contains_key(alt_title) {
                    println!("Match!");
                    Self::set_media(media, extra.get(alt_title).cloned());
                    return Ok(std::mem::take(media));
                }
            }

            let mut score_tuple: (f64, Option<&Self::Extra>) = (-f64::INFINITY, None);
            for (ex_title, ex) in extra {
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
