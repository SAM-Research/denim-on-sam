use async_trait::async_trait;
use denim_sam_common::Seed;
use futures_util::TryFutureExt;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sam_common::{address::DeviceAddress, api::EcPreKey, AccountId, DeviceId};
use sam_server::managers::{
    in_memory::keys::{InMemoryEcPreKeyManager, InMemorySignedPreKeyManager},
    traits::key_manager::EcPreKeyManager,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::managers::{
    default::generate_ec_pre_keys, DenimEcPreKeyManager, DenimKeyManagerError, DenimKeyManagerType,
};

#[derive(Clone)]
pub struct InMemoryDenimEcPreKeyManager {
    keys_generated: usize,
    manager: InMemoryEcPreKeyManager,
    seeds: Arc<Mutex<HashMap<DeviceAddress, (Seed, u128)>>>,
}

impl Default for InMemoryDenimEcPreKeyManager {
    fn default() -> Self {
        Self {
            keys_generated: 10,
            manager: InMemoryEcPreKeyManager::default(),
            seeds: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DenimEcPreKeyManager for InMemoryDenimEcPreKeyManager {
    async fn get_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<EcPreKey, DenimKeyManagerError> {
        if let Some(pk) = self.manager.get_pre_key(account_id, device_id).await? {
            Ok(pk)
        } else {
            generate_ec_pre_keys(self, account_id, device_id, self.keys_generated).await?;
            self.manager
                .get_pre_key(account_id, device_id)
                .await?
                .ok_or(DenimKeyManagerError::NoKeyInStore)
        }
    }

    async fn get_ec_pre_key_ids(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<Vec<u32>, DenimKeyManagerError> {
        Ok(self
            .manager
            .get_pre_key_ids(account_id, device_id)
            .map_err(DenimKeyManagerError::from)
            .await?
            .unwrap_or_default())
    }

    async fn add_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        key: EcPreKey,
    ) -> Result<(), DenimKeyManagerError> {
        self.manager.add_pre_key(account_id, device_id, key).await?;
        Ok(())
    }

    async fn remove_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError> {
        self.manager
            .remove_pre_key(account_id, device_id, id)
            .await?;
        Ok(())
    }
    async fn get_csprng_for(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<ChaCha20Rng, DenimKeyManagerError> {
        let res = self
            .seeds
            .lock()
            .await
            .get(&DeviceAddress::new(account_id, device_id))
            .cloned()
            .ok_or(DenimKeyManagerError::NoSeed)?;
        let (seed, offset) = res;
        let mut csprng = ChaCha20Rng::from_seed(*seed);
        csprng.set_word_pos(offset);
        Ok(csprng)
    }

    async fn store_csprng_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        csprng: &ChaCha20Rng,
    ) -> Result<(), DenimKeyManagerError> {
        let seed = csprng.get_seed();
        let offset = csprng.get_word_pos();

        self.seeds.lock().await.insert(
            DeviceAddress::new(account_id, device_id),
            (seed.into(), offset),
        );

        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryDenimKeyManager;

impl DenimKeyManagerType for InMemoryDenimKeyManager {
    type EcPreKeyManager = InMemoryDenimEcPreKeyManager;

    type SignedPreKeyManager = InMemorySignedPreKeyManager;
}
