use crate::Media;
use crate::Result;

use std::collections::HashMap;

pub trait Transform {
    type Extra: Clone;

    fn get_similarity_threshold(&self) -> f64;

    fn set_media(media: &mut Media, extra: Option<Self::Extra>);

    fn transform(&self, mut media: Media, extra: &HashMap<String, Self::Extra>) -> Result<Media> {
        if media.status == Some("CURRENT".to_string()) {
            let title = match media.title.to_owned() {
                Some(title) => title,
                None => String::new(),
            };
            let alt_title = match media.alt_title.to_owned() {
                Some(alt_title) => alt_title,
                None => String::new(),
            };

            if extra.contains_key(&title) {
                Self::set_media(&mut media, extra.get(&title).cloned());
                return Ok(media);
            }

            if extra.contains_key(&alt_title) {
                Self::set_media(&mut media, extra.get(&alt_title).cloned());
                return Ok(media);
            }

            let mut score_tuple: (f64, Option<&Self::Extra>) = (-f64::INFINITY, None);
            for (ex_title, ex) in extra {
                let score = strsim::normalized_levenshtein(&title, ex_title);
                let alt_score = strsim::normalized_levenshtein(&alt_title, ex_title);
                if score > self.get_similarity_threshold() && score > score_tuple.0 {
                    score_tuple = (score, Some(ex));
                }
                if alt_score > self.get_similarity_threshold() && alt_score > score_tuple.0 {
                    score_tuple = (alt_score, Some(ex));
                }
            }

            if score_tuple.1.is_some() {
                Self::set_media(&mut media, score_tuple.1.cloned());
            }
        }

        Ok(media)
    }
}
