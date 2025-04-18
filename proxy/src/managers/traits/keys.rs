use async_trait::async_trait;
use derive_more::{Display, Error, From};
use rand_chacha::ChaCha20Rng;
use sam_common::{api::EcPreKey, AccountId, DeviceId};
use sam_server::managers::error::KeyManagerError;

#[derive(Debug, Display, Error, From)]
pub enum DenimKeyManagerError {
    Sam(KeyManagerError),
    NoSeed,
    NoKeyInStore,
    CouldNotGenerateKeyId,
}

#[async_trait]
pub trait DenimEcPreKeyManager: Clone + Send + Sync {
    async fn get_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<EcPreKey, DenimKeyManagerError>;

    async fn get_ec_pre_key_ids(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<Vec<u32>, DenimKeyManagerError>;

    async fn add_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        key: EcPreKey,
    ) -> Result<(), DenimKeyManagerError>;

    async fn remove_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError>;

    async fn next_key_id(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<u32, DenimKeyManagerError>;

    async fn get_csprng_for(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<ChaCha20Rng, DenimKeyManagerError>;

    async fn store_csprng_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        csprng: &ChaCha20Rng,
    ) -> Result<(), DenimKeyManagerError>;
}
