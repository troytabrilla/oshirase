#[cfg(test)]
pub mod helpers {
    use crate::db::{Persist, DB};
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

    struct Initializer<'a> {
        db: &'a DB<'a>,
        config: &'a Config,
    }

    impl Persist for Initializer<'_> {
        fn get_client(&self) -> &mongodb::Client {
            &self.db.mongodb.client
        }

        fn get_database(&self) -> &str {
            self.config.db.mongodb.database.as_str()
        }
    }

    pub async fn init() {
        let config = Config::default();
        let db = DB::new(&config).await;
        let fixtures = Fixtures::default();
        let initializer = Initializer {
            db: &db,
            config: &config,
        };

        let database = initializer
            .get_client()
            .database(initializer.get_database());
        database.drop(None).await.unwrap();

        let mut connection = db.redis.client.get_connection().unwrap();
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
}
