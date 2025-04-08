use crate::DenimClientError;

use super::{DeniableStore, DeniableStoreConfig, DeniableStoreType};
use async_trait::async_trait;
use sam_client::{
    storage::{SqlitePreKeyStore, SqliteSessionStore},
    ClientError,
};
use sqlx::sqlite::SqlitePoolOptions;

pub struct SqliteDeniableStoreType;

impl DeniableStoreType for SqliteDeniableStoreType {
    type SessionStore = SqliteSessionStore;

    type PreKeyStore = SqlitePreKeyStore;
}

pub type SqliteDeniableStore = DeniableStore<SqliteDeniableStoreType>;

#[derive(Debug, Default)]
pub struct SqliteDeniableStoreConfig {
    connection_string: String,
}

impl SqliteDeniableStoreConfig {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }

    pub async fn in_memory() -> Self {
        Self {
            connection_string: "sqlite::memory:".to_owned(),
        }
    }
}

#[async_trait]
impl DeniableStoreConfig for SqliteDeniableStoreConfig {
    type DeniableStoreType = SqliteDeniableStoreType;

    async fn create_store(
        self,
    ) -> Result<DeniableStore<Self::DeniableStoreType>, DenimClientError> {
        let database = SqlitePoolOptions::new()
            .connect(&self.connection_string)
            .await
            .map_err(|err| {
                ClientError::Database(format!(
                    "Could not connect to the database at '{}': {}",
                    self.connection_string, err
                ))
            })?;
        sqlx::migrate!("database/migrations")
            .run(&database)
            .await
            .map_err(|err| {
                ClientError::Database(format!(
                    "Could not run migrations on database at '{}': {}",
                    self.connection_string, err
                ))
            })?;
        Ok(SqliteDeniableStore::builder()
            .session_store(SqliteSessionStore::new(database.clone()))
            .pre_key_store(SqlitePreKeyStore::new(database.clone()))
            .build())
    }
}
