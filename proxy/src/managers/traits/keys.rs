use async_trait::async_trait;
use denim_sam_common::RngState;

use rand::Rng;
use sam_common::{api::EcPreKey, AccountId, DeviceId};

use crate::managers::error::DenimKeyManagerError;

#[async_trait]
pub trait DenimEcPreKeyManager<T: RngState>: Clone + Send + Sync {
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

    async fn next_key_id<R: Rng + Send>(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        rng: &mut R,
    ) -> Result<u32, DenimKeyManagerError>;

    async fn get_key_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<T, DenimKeyManagerError>;

    async fn store_key_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: T,
    ) -> Result<(), DenimKeyManagerError>;

    async fn get_key_id_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<T, DenimKeyManagerError>;

    async fn store_key_id_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: T,
    ) -> Result<(), DenimKeyManagerError>;
}
