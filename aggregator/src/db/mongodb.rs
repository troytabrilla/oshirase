use crate::config::Config;
use crate::sources::Document;
use crate::AltTitlesEntry;
use crate::CustomError;
use crate::Media;
use crate::Result;
use crate::User;

use futures::future::try_join_all;
use mongodb::{
    bson::doc,
    options::{ClientOptions, FindOneAndUpdateOptions, IndexOptions},
    IndexModel,
};
use std::{collections::hash_map::DefaultHasher, hash::Hasher};

pub struct MongoDB<'a> {
    pub client: mongodb::Client,
    pub config: &'a Config,
}

impl MongoDB<'_> {
    pub async fn new(config: &Config) -> MongoDB {
        let mut options = ClientOptions::parse(&config.db.mongodb.uri).await.unwrap();
        options.app_name = Some("oshirase-aggregator".to_owned());
        let client = mongodb::Client::with_options(options).unwrap();

        MongoDB { client, config }
    }

    pub async fn init(config: &Config) -> MongoDB {
        let mongodb = MongoDB::new(config).await;

        tokio::try_join!(
            mongodb.create_unique_index::<Media>("anime", "media_id"),
            mongodb.create_unique_index::<Media>("manga", "media_id"),
            mongodb.create_unique_index::<Media>("anime", "hash"),
            mongodb.create_unique_index::<Media>("manga", "hash"),
            mongodb.create_unique_index::<User>("users", "id"),
            mongodb.create_unique_index::<AltTitlesEntry>("alt_titles", "media_id"),
        )
        .unwrap();

        mongodb
    }

    async fn create_unique_index<T>(&self, collection: &str, key: &str) -> Result<()>
    where
        T: Document,
    {
        let database = self.client.database(&self.config.db.mongodb.database);
        let collection = database.collection::<T>(collection);

        let index_options = IndexOptions::builder().unique(true).build();
        let index = IndexModel::builder()
            .keys(doc! { format!("{}", key): 1 })
            .options(index_options)
            .build();
        collection.create_index(index, None).await?;

        Ok(())
    }

    pub fn hash_document<T>(document: &T) -> String
    where
        T: Document,
    {
        let mut hasher = DefaultHasher::new();
        document.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:x}", hash)
    }

    pub async fn upsert_documents<T>(
        &self,
        collection: &str,
        documents: &[T],
        id_key: &str,
    ) -> Result<()>
    where
        T: Document,
    {
        let database = self.client.database(&self.config.db.mongodb.database);
        let collection = database.collection::<T>(collection);

        let mut futures = Vec::new();

        for document in documents {
            let hash = Self::hash_document(document);

            let mut document = bson::to_document(document)?;
            document.extend(doc! { "modified": bson::DateTime::now(), "hash": &hash });

            let id = document
                .get(id_key)
                .ok_or(CustomError::boxed(&format!("Could not find {}.", id_key)))?;

            futures.push(collection.find_one_and_update(
                doc! { format!("{}", id_key): id },
                doc! { "$set": document },
                FindOneAndUpdateOptions::builder().upsert(true).build(),
            ));
        }

        try_join_all(futures).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use futures::TryStreamExt;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Hash, PartialEq, Serialize, Deserialize)]
    struct Test {
        test: String,
        extra: u8,
    }
    impl Document for Test {}

    #[tokio::test]
    async fn test_mongodb_upsert_documents() {
        let config = Config::default();
        let mongo = MongoDB::init(&config).await;
        let collection = mongo
            .client
            .database(&mongo.config.db.mongodb.database)
            .collection::<Test>("test");
        collection.drop(None).await.unwrap();

        mongo
            .upsert_documents(
                "test",
                &[Test {
                    test: "test".to_owned(),
                    extra: 21,
                }],
                "test",
            )
            .await
            .unwrap();
        let docs = collection
            .find(doc! { "test": "test" }, None)
            .await
            .unwrap();
        let docs: Vec<Test> = docs.try_collect().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].extra, 21);

        mongo
            .upsert_documents(
                "test",
                &[Test {
                    test: "test".to_owned(),
                    extra: 42,
                }],
                "test",
            )
            .await
            .unwrap();
        let docs = collection
            .find(doc! { "test": "test" }, None)
            .await
            .unwrap();
        let docs: Vec<Test> = docs.try_collect().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].extra, 42);
    }
}
