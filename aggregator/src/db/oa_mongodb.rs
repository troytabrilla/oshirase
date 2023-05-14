use crate::config::{Config, MongoDBConfig};
use crate::db::Document;
use crate::CustomError;
use crate::Result;

use async_trait::async_trait;
use futures::future::try_join_all;
use mongodb::{
    bson::doc,
    options::{ClientOptions, FindOneAndUpdateOptions, ServerAddress},
};
use std::{collections::hash_map::DefaultHasher, hash::Hasher};

pub struct MongoDB {
    pub client: mongodb::Client,
    pub config: MongoDBConfig,
}

impl MongoDB {
    pub fn new(config: MongoDBConfig) -> MongoDB {
        let address = ServerAddress::parse(&config.host).unwrap();
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name("oshirase-aggregator".to_owned())
            .build();
        let client = mongodb::Client::with_options(options).unwrap();

        MongoDB { client, config }
    }
}

impl Default for MongoDB {
    fn default() -> MongoDB {
        let config = Config::default();

        Self::new(config.db.mongodb)
    }
}

#[async_trait]
pub trait Persist {
    fn get_client(&self) -> &mongodb::Client;

    fn get_database(&self) -> String;

    fn hash_document<T>(document: &T) -> String
    where
        T: Document,
    {
        let mut hasher = DefaultHasher::new();
        document.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:x}", hash)
    }

    async fn upsert_documents<T>(
        &self,
        collection: &str,
        id_key: &str,
        documents: &Vec<T>,
    ) -> Result<()>
    where
        T: Document,
    {
        let database = self.get_client().database(&self.get_database());
        let collection = database.collection::<T>(collection);

        let mut futures = Vec::new();

        for document in documents {
            let hash = Self::hash_document(document);

            // @todo Use atomic operations
            let existing = collection.find_one(doc! { "hash": &hash }, None).await?;
            if existing.is_none() {
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
        }

        try_join_all(futures).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Hash, PartialEq, Serialize, Deserialize)]
    struct Test {
        test: String,
    }
    impl Document for Test {}

    struct Persister {
        client: mongodb::Client,
    }
    impl Persist for Persister {
        fn get_client(&self) -> &mongodb::Client {
            &self.client
        }

        fn get_database(&self) -> String {
            "test".to_owned()
        }
    }

    #[tokio::test]
    async fn test_mongodb_upsert_documents() {
        let mongo = MongoDB::default();
        let collection = mongo
            .client
            .database(&mongo.config.database)
            .collection::<Test>("test");
        collection.drop(None).await.unwrap();

        let persister = Persister {
            client: mongo.client.clone(),
        };

        persister
            .upsert_documents(
                "test",
                "test",
                &vec![Test {
                    test: "test".to_owned(),
                }],
            )
            .await
            .unwrap();

        let count = collection
            .count_documents(doc! { "test": "test" }, None)
            .await
            .unwrap();

        assert_eq!(count, 1);
    }
}
