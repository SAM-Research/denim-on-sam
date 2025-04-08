use async_trait::async_trait;
use bon::Builder;
use libsignal_protocol::{PreKeyId, PreKeyStore, SessionStore};
use sam_client::storage::{key_generation::PreKeyGenerator, ProvidesKeyId};

use crate::DenimClientError;

pub mod inmem;
pub mod sqlite;

#[async_trait]
pub trait DeniableStoreConfig {
    type DeniableStoreType: DeniableStoreType;

    async fn create_store(self)
        -> Result<DeniableStore<Self::DeniableStoreType>, DenimClientError>;
}

pub trait DeniableStoreType {
    type SessionStore: SessionStore;
    type PreKeyStore: PreKeyStore + ProvidesKeyId<PreKeyId> + PreKeyGenerator;
}

#[derive(Builder)]
pub struct DeniableStore<T: DeniableStoreType> {
    pub session_store: T::SessionStore,
    pub pre_key_store: T::PreKeyStore,
}
