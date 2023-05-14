use crate::config::{CombinerConfig, Config};
use crate::sources::anilist_api::Media;
use crate::sources::subsplease_scraper::AnimeScheduleEntry;
use crate::Result;

pub struct Combiner {
    config: CombinerConfig,
}

impl Combiner {
    pub fn new(config: CombinerConfig) -> Combiner {
        Combiner { config }
    }

    pub fn combine<'a>(
        &self,
        anime: &'a mut [Media],
        schedules: &[AnimeScheduleEntry],
    ) -> Result<&'a [Media]> {
        for entry in &mut *anime {
            if let Some(status) = &entry.status {
                if status == "CURRENT" {
                    let anime_title = match &entry.title {
                        Some(title) => title,
                        None => "",
                    };
                    let anime_alt_title = match &entry.alt_title {
                        Some(alt_title) => alt_title,
                        None => "",
                    };

                    let mut score_schedule_tuple: (f64, Option<&AnimeScheduleEntry>) =
                        (-f64::INFINITY, None);
                    for schedule in schedules {
                        let score = strsim::normalized_levenshtein(anime_title, &schedule.title);
                        let alt_score =
                            strsim::normalized_levenshtein(anime_alt_title, &schedule.title);
                        if score > self.config.similarity_threshold
                            && score > score_schedule_tuple.0
                        {
                            score_schedule_tuple = (score, Some(schedule));
                        }
                        if alt_score > self.config.similarity_threshold
                            && alt_score > score_schedule_tuple.0
                        {
                            score_schedule_tuple = (alt_score, Some(schedule));
                        }
                    }

                    entry.schedule = score_schedule_tuple.1.cloned();
                }
            }
        }

        Ok(anime)
    }
}

impl Default for Combiner {
    fn default() -> Self {
        let config = Config::default();

        Combiner::new(config.combiner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::anilist_api::Media;
    use crate::sources::subsplease_scraper::Day;

    #[test]
    fn test_combine() {
        let mut media = vec![Media {
            media_id: Some(1),
            media_type: None,
            status: Some("CURRENT".to_owned()),
            format: None,
            season: None,
            season_year: None,
            title: Some("Gintama".to_owned()),
            alt_title: Some("Gin Tama".to_owned()),
            image: None,
            episodes: None,
            score: None,
            progress: None,
            latest: None,
            schedule: None,
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

        let combiner = Combiner::default();
        let media = combiner.combine(&mut media, &schedules).unwrap();

        let actual = media[0].schedule.as_ref().unwrap();
        let expected = &schedules[0];

        assert_eq!(actual, expected);
    }
}
