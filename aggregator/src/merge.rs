use crate::sources::anilist_api::Media;
use crate::sources::subsplease_scraper::AnimeScheduleEntry;
use crate::CustomError;
use crate::Result;

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
        schema_builder.add_text_field("media_index", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("alt_title", TEXT | STORED);

        let schema = schema_builder.build();

        let index = Index::create_in_dir(&index_path, schema.clone())?;

        let mut index_writer = index.writer(50_000_000)?;

        let media_index: Field = Self::get_field(&schema, "media_index")?;
        let title = Self::get_field(&schema, "title")?;
        let alt_title = Self::get_field(&schema, "alt_title")?;

        for (i, entry) in (*media).iter_mut().enumerate() {
            if let Some(status) = &entry.status {
                if status != "CURRENT" {
                    continue;
                }
            }
            let mut document = Document::default();
            document.add_text(media_index, i);
            if let Some(media_title) = &entry.title {
                document.add_text(title, media_title);
            }
            if let Some(media_alt_title) = &entry.alt_title {
                document.add_text(alt_title, media_alt_title);
            }
            if !document.is_empty() {
                index_writer.add_document(document)?;
            }
        }

        index_writer.commit()?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;
        let searcher = reader.searcher();

        let schema = index.schema();
        let title = Self::get_field(&schema, "title")?;
        let alt_title = Self::get_field(&schema, "alt_title")?;

        let query_parser = QueryParser::for_index(&index, vec![title, alt_title]);

        for schedule in schedules {
            let query = query_parser.parse_query(&schedule.title);

            let query = match query {
                Ok(query) => query,
                Err(err) => {
                    println!("Could not parse query: {}", err);
                    continue;
                }
            };

            let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

            if let Some((_score, doc_address)) = top_docs.first() {
                let doc = searcher.doc(*doc_address)?;
                let field = schema.get_field("media_index");
                if let Some(field) = field {
                    let media_index = doc
                        .get_first(field)
                        .ok_or(CustomError::boxed("Could not get media index."))?;
                    if let Value::Str(media_index) = media_index {
                        let media_index = media_index.parse::<usize>()?;
                        media[media_index].schedule = Some(schedule.clone());
                    }
                }
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
