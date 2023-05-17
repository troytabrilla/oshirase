use crate::config::Config;
use crate::db::Document;
use crate::CustomError;
use crate::Result;

use async_trait::async_trait;
use futures::future::try_join_all;
use mongodb::{
    bson::doc,
    options::{ClientOptions, FindOneAndUpdateOptions, IndexOptions, ServerAddress},
    IndexModel,
};
use std::{collections::hash_map::DefaultHasher, hash::Hasher};

pub struct MongoDB<'a> {
    pub client: mongodb::Client,
    pub config: &'a Config,
}

impl MongoDB<'_> {
    pub fn new(config: &Config) -> MongoDB {
        let address = ServerAddress::parse(&config.db.mongodb.host).unwrap();
        let hosts = vec![address];
        let options = ClientOptions::builder()
            .hosts(hosts)
            .app_name("oshirase-aggregator".to_owned())
            .build();
        let client = mongodb::Client::with_options(options).unwrap();

        MongoDB { client, config }
    }
}

#[async_trait]
pub trait Persist {
    fn get_client(&self) -> &mongodb::Client;

    fn get_database(&self) -> &str;

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
        let database = self.get_client().database(self.get_database());
        let collection = database.collection::<T>(collection);

        let index_options = IndexOptions::builder().unique(true).build();
        let id_index = IndexModel::builder()
            .keys(doc! { format!("{}", id_key): 1 })
            .options(index_options.clone())
            .build();
        collection.create_index(id_index, None).await?;
        let hash_index = IndexModel::builder()
            .keys(doc! { "hash": 1 })
            .options(index_options)
            .build();
        collection.create_index(hash_index, None).await?;

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

    struct Persister {
        client: mongodb::Client,
    }
    impl Persist for Persister {
        fn get_client(&self) -> &mongodb::Client {
            &self.client
        }

        fn get_database(&self) -> &str {
            "test"
        }
    }

    #[tokio::test]
    async fn test_mongodb_upsert_documents() {
        let config = Config::default();
        let mongo = MongoDB::new(&config);
        let collection = mongo
            .client
            .database(&mongo.config.db.mongodb.database)
            .collection::<Test>("test");
        collection.drop(None).await.unwrap();

        let persister = Persister {
            client: mongo.client,
        };

        persister
            .upsert_documents(
                "test",
                "test",
                &vec![Test {
                    test: "test".to_owned(),
                    extra: 21,
                }],
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

        persister
            .upsert_documents(
                "test",
                "test",
                &vec![Test {
                    test: "test".to_owned(),
                    extra: 42,
                }],
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
