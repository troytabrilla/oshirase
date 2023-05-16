use crate::config::EmitterConfig;
use crate::Result;

use serde::Serialize;

pub struct Emitter<'a> {
    config: &'a EmitterConfig,
}

pub struct MediaLite {
    pub title: String,
    pub alt_title: String,
    pub status: String,
}

pub trait Extra {
    fn get_title(&self) -> &String;
}

#[derive(Debug)]
pub struct Emitted {
    pub index: usize,
    pub key: String,
    pub json: String,
}

impl Emitter<'_> {
    pub fn new(config: &EmitterConfig) -> Emitter {
        Emitter { config }
    }

    pub fn emit<T>(
        &self,
        media: &[MediaLite],
        extra: &[T],
        key: &str,
        snd: crossbeam_channel::Sender<Emitted>,
    ) -> Result<()>
    where
        T: Extra + Serialize + Send + Sync + std::fmt::Debug,
    {
        for (index, entry) in media.iter().enumerate() {
            if entry.status == "CURRENT" {
                let title = &entry.title;
                let alt_title = &entry.alt_title;

                let mut score_tuple: (f64, Option<&T>) = (-f64::INFINITY, None);
                for ex in extra {
                    let score = strsim::normalized_levenshtein(title, &ex.get_title());
                    let alt_score = strsim::normalized_levenshtein(alt_title, &ex.get_title());
                    if score > self.config.similarity_threshold && score > score_tuple.0 {
                        score_tuple = (score, Some(ex));
                    }
                    if alt_score > self.config.similarity_threshold && alt_score > score_tuple.0 {
                        score_tuple = (alt_score, Some(ex));
                    }
                }

                if score_tuple.1.is_some() {
                    snd.send(Emitted {
                        index,
                        key: key.to_owned(),
                        json: serde_json::to_string(score_tuple.1.unwrap())?,
                    })?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::subsplease_scraper::{AnimeScheduleEntry, Day};
    use crate::Config;
    use crossbeam_channel::bounded;

    #[test]
    fn test_combine() {
        let media = vec![MediaLite {
            status: "CURRENT".to_owned(),
            title: "Gintama".to_owned(),
            alt_title: "Gin Tama".to_owned(),
        }];
        let schedules = vec![
            AnimeScheduleEntry {
                title: "gintama".to_owned(),
                day: Day::Saturday,
                time: "00:00".to_owned(),
            },
            AnimeScheduleEntry {
                title: "naruto".to_owned(),
                day: Day::Monday,
                time: "00:00".to_owned(),
            },
            AnimeScheduleEntry {
                title: "tamako market".to_owned(),
                day: Day::Friday,
                time: "00:00".to_owned(),
            },
        ];

        let (snd, rcv) = bounded(4);

        let config = Config::default();
        let emitter = Emitter::new(&config.emitter);
        emitter.emit(&media, &schedules, "schedule", snd).unwrap();

        let mut msgs = Vec::new();
        for msg in rcv.iter() {
            msgs.push(msg);
        }

        assert_eq!(msgs[0].index, 0);
        assert_eq!(msgs[0].key, "schedule");
        assert_eq!(
            serde_json::from_str::<AnimeScheduleEntry>(&msgs[0].json).unwrap(),
            schedules[0]
        );
    }
}
