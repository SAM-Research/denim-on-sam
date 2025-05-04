use async_trait::async_trait;
use denim_sam_common::rng::chacha::ChaChaRngState;
use sam_client::storage::{
    sqlite::sqlite_connector::SqliteConnector, SqliteContactStore, SqliteMessageStore,
    SqlitePreKeyStore, SqliteSessionStore,
};

use crate::DenimClientError;

use super::{DeniableStore, DeniableStoreConfig, DeniableStoreType, InMemoryPreKeySeedStore};

pub struct SqliteDeniableStoreType;

impl DeniableStoreType for SqliteDeniableStoreType {
    type ContactStore = SqliteContactStore;
    type MessageStore = SqliteMessageStore;
    type SessionStore = SqliteSessionStore;
    type PreKeyStore = SqlitePreKeyStore;
    type SeedStore = InMemoryPreKeySeedStore<ChaChaRngState>;
}

pub type SqliteDeniableStore = DeniableStore<SqliteDeniableStoreType>;

pub struct SqliteDeniableStoreConfig {
    buffer_size: usize,
    connector: SqliteConnector,
}

impl SqliteDeniableStoreConfig {
    pub fn new(connector: SqliteConnector, buffer_size: usize) -> Self {
        Self {
            buffer_size,
            connector,
        }
    }
}

#[async_trait]
impl DeniableStoreConfig for SqliteDeniableStoreConfig {
    type DeniableStoreType = SqliteDeniableStoreType;

    async fn create_store(
        self,
    ) -> Result<DeniableStore<Self::DeniableStoreType>, DenimClientError> {
        Ok(SqliteDeniableStore::builder()
            .session_store(SqliteSessionStore::new(self.connector.pool()))
            .pre_key_store(SqlitePreKeyStore::new(self.connector.pool()))
            .contact_store(SqliteContactStore::new(self.connector.pool()))
            .message_store(SqliteMessageStore::new(
                self.connector.pool(),
                self.buffer_size,
            ))
            .seed_store(InMemoryPreKeySeedStore::default())
            .build())
    }
}
