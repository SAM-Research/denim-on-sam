use async_trait::async_trait;
use denim_sam_common::Seed;


use sam_common::{api::EcPreKey, AccountId, DeviceId};

use crate::managers::error::DenimKeyManagerError;

use super::crypto_provider::CryptoProvider;

#[async_trait]
pub trait DenimEcPreKeyManager: Clone + Send + Sync {
    async fn get_ec_pre_key<C: CryptoProvider>(
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
    ) -> Result<(Seed, u128), DenimKeyManagerError>;

    async fn store_csprng_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: Seed,
        offset: u128,
    ) -> Result<(), DenimKeyManagerError>;
}
