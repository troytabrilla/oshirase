#[cfg(test)]
pub mod helpers {
    use crate::db::{MongoDB, Redis};
    use crate::Config;
    use crate::User;

    use bson::doc;
    use serde::Deserialize;
    use std::fs;
    use tokio::sync::OnceCell;

    pub static ONCE: OnceCell<()> = OnceCell::const_new();

    #[derive(Debug, Deserialize)]
    pub struct Fixtures {
        pub user: User,
    }

    impl Fixtures {
        pub fn from_file(filename: &str) -> Fixtures {
            let fixtures = fs::read_to_string(filename).unwrap();
            let fixtures: Fixtures = toml::from_str(&fixtures).unwrap();
            fixtures
        }
    }

    impl Default for Fixtures {
        fn default() -> Fixtures {
            Self::from_file("fixtures/fixtures.toml")
        }
    }

    pub async fn init() {
        let config = Config::default();
        let mongodb = MongoDB::init(&config).await;
        let redis = Redis::new(&config);
        let fixtures = Fixtures::default();

        let database = mongodb.client.database(&config.db.mongodb.database);
        database.drop(None).await.unwrap();

        let mut connection = redis.client.get_connection().unwrap();
        redis::cmd("FLUSHALL").query::<()>(&mut connection).unwrap();

        database
            .collection("users")
            .insert_one(
                doc! {
                    "id": fixtures.user.id as i64,
                    "name": &fixtures.user.name
                },
                None,
            )
            .await
            .unwrap();
    }

    pub async fn reset_db() {
        let config = Config::default();
        let mongodb = MongoDB::init(&config).await;
        let database = mongodb.client.database(&config.db.mongodb.database);
        database.collection::<()>("anime").drop(None).await.unwrap();
        database.collection::<()>("manga").drop(None).await.unwrap();
        database
            .collection::<()>("alt_titles")
            .drop(None)
            .await
            .unwrap();
        database.collection::<()>("test").drop(None).await.unwrap();
    }
}
