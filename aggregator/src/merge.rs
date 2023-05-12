use crate::sources::anilist_api::Media;
use crate::sources::subsplease_scraper::AnimeScheduleEntry;
use crate::CustomError;
use crate::Result;

use regex::Regex;
use tantivy::DocAddress;
use tantivy::ReloadPolicy;
use tantivy::{collector::TopDocs, query::QueryParser, schema::*, Index};
use tempfile::TempDir;
pub struct Merge;

impl Merge {
    pub fn merge<'a>(
        media: &'a mut [Media],
        schedules: &[AnimeScheduleEntry],
    ) -> Result<&'a [Media]> {
        let index_path = TempDir::new()?;

        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("schedule_index", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);

        let schema = schema_builder.build();

        let index = Index::create_in_dir(&index_path, schema.clone())?;

        let mut index_writer = index.writer(50_000_000)?;

        let schedule_index = Self::get_field(&schema, "schedule_index")?;
        let title = Self::get_field(&schema, "title")?;

        for (i, entry) in (*schedules).iter().enumerate() {
            let mut document = Document::default();
            document.add_text(schedule_index, i);
            document.add_text(title, &entry.title);
            index_writer.add_document(document)?;
        }

        index_writer.commit()?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;
        let searcher = reader.searcher();

        let schema = index.schema();
        let title = Self::get_field(&schema, "title")?;

        let query_parser = QueryParser::for_index(&index, vec![title]);

        for entry in &mut *media {
            if let Some(status) = &entry.status {
                if status != "CURRENT" {
                    continue;
                }
            }

            let replacement = Regex::new(r"[-:]")?;

            let query_title = match &entry.title {
                Some(title) => {
                    let title = replacement.replace_all(title, "");
                    match query_parser.parse_query(title.to_string().as_str()) {
                        Ok(query) => query,
                        Err(err) => {
                            println!("Could not parse title query: {}", err);
                            continue;
                        }
                    }
                }
                None => {
                    println!("No title to search for.");
                    continue;
                }
            };

            let query_alt_title = match &entry.alt_title {
                Some(alt_title) => {
                    let alt_title = replacement.replace_all(alt_title, "");
                    match query_parser.parse_query(alt_title.to_string().as_str()) {
                        Ok(query) => query,
                        Err(err) => {
                            println!("Could not parse title query: {}", err);
                            continue;
                        }
                    }
                }
                None => {
                    println!("No alt title to search for.");
                    continue;
                }
            };

            let top_docs_title = searcher.search(&query_title, &TopDocs::with_limit(1))?;
            let top_docs_alt_title = searcher.search(&query_alt_title, &TopDocs::with_limit(1))?;

            let title_doc = top_docs_title.first();
            let alt_title_doc = top_docs_alt_title.first();

            let get_doc_score_tuple =
                |doc: Option<&(f32, DocAddress)>| -> Result<(f32, Option<AnimeScheduleEntry>)> {
                    if let Some((score, doc_address)) = doc {
                        let doc = searcher.doc(*doc_address)?;

                        let field = schema.get_field("schedule_index");
                        let schedule = if let Some(field) = field {
                            let schedule_index = doc
                                .get_first(field)
                                .ok_or(CustomError::boxed("Could not get media index."))?;
                            if let Value::Str(schedule_index) = schedule_index {
                                let schedule_index = schedule_index.parse::<usize>()?;
                                let schedule = &schedules[schedule_index];
                                Some(schedule)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        Ok((*score, schedule.cloned()))
                    } else {
                        Ok((0.0, None))
                    }
                };

            let title_score_tuple = get_doc_score_tuple(title_doc)?;
            let alt_title_score_tuple = get_doc_score_tuple(alt_title_doc)?;

            if title_score_tuple.1.is_some() && alt_title_score_tuple.1.is_some() {
                if title_score_tuple.0 > alt_title_score_tuple.0 {
                    entry.schedule = title_score_tuple.1;
                } else {
                    entry.schedule = alt_title_score_tuple.1;
                }
            } else if title_score_tuple.1.is_some() {
                entry.schedule = title_score_tuple.1;
            } else if alt_title_score_tuple.1.is_some() {
                entry.schedule = alt_title_score_tuple.1;
            }
        }

        Ok(media)
    }

    fn get_field(schema: &Schema, field: &str) -> Result<Field> {
        schema.get_field(field).ok_or(CustomError::boxed(
            format!("Could not get {} field for index.", field).as_str(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::anilist_api::Media;
    use crate::sources::subsplease_scraper::Day;

    #[test]
    fn test_merge() {
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
        let schedules = vec![AnimeScheduleEntry {
            title: "gintama".to_owned(),
            day: Day::Saturday,
            time: "00:00".to_owned(),
        }];

        let media = Merge::merge(&mut media, &schedules).unwrap();

        let actual = media[0].schedule.as_ref().unwrap();
        let expected = &schedules[0];

        assert_eq!(actual, expected);
    }
}
