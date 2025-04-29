use async_trait::async_trait;
use denim_sam_common::rng::RngState;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum SeedStoreError {}

#[async_trait]
pub trait DenimPreKeySeedStore<T: RngState> {
    async fn get_key_id_seed(&self) -> Result<T, SeedStoreError>;
    async fn get_key_seed(&self) -> Result<T, SeedStoreError>;
    async fn set_key_id_seed(&mut self, record: T) -> Result<(), SeedStoreError>;
    async fn set_key_seed(&mut self, record: T) -> Result<(), SeedStoreError>;
}

#[derive(Debug, Default)]
pub struct InMemoryPreKeySeedStore<T: RngState> {
    key_seed: T,
    key_id_seed: T,
}

#[async_trait]
impl<T: RngState> DenimPreKeySeedStore<T> for InMemoryPreKeySeedStore<T> {
    async fn get_key_id_seed(&self) -> Result<T, SeedStoreError> {
        Ok(self.key_id_seed.clone())
    }
    async fn get_key_seed(&self) -> Result<T, SeedStoreError> {
        Ok(self.key_seed.clone())
    }
    async fn set_key_id_seed(&mut self, record: T) -> Result<(), SeedStoreError> {
        self.key_id_seed = record;
        Ok(())
    }
    async fn set_key_seed(&mut self, record: T) -> Result<(), SeedStoreError> {
        self.key_seed = record;
        Ok(())
    }
}
