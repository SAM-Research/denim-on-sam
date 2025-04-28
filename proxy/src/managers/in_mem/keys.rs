use async_trait::async_trait;
use denim_sam_common::{crypto::CryptoProvider, ChaChaRngState};
use futures_util::TryFutureExt;
use log::debug;
use rand::Rng;
use sam_common::{address::DeviceAddress, api::EcPreKey, AccountId, DeviceId};
use sam_server::managers::{
    in_memory::keys::{InMemoryEcPreKeyManager, InMemorySignedPreKeyManager},
    traits::key_manager::EcPreKeyManager,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::managers::{
    default::generate_ec_pre_keys, error::DenimKeyManagerError, DenimEcPreKeyManager,
    DenimKeyManagerType,
};

#[derive(Clone)]
pub struct InMemoryDenimEcPreKeyManager {
    key_generate_amount: usize,
    manager: InMemoryEcPreKeyManager,
    id_seeds: Arc<Mutex<HashMap<DeviceAddress, ChaChaRngState>>>,
    key_seeds: Arc<Mutex<HashMap<DeviceAddress, ChaChaRngState>>>,
    used_keys: Arc<Mutex<HashMap<DeviceAddress, Vec<u32>>>>,
}

impl InMemoryDenimEcPreKeyManager {
    pub fn new(key_generate_amount: usize) -> Self {
        Self {
            key_generate_amount,
            ..Default::default()
        }
    }
}

impl Default for InMemoryDenimEcPreKeyManager {
    fn default() -> Self {
        Self {
            key_generate_amount: 10,
            manager: InMemoryEcPreKeyManager::default(),
            id_seeds: Arc::default(),
            key_seeds: Arc::default(),
            used_keys: Arc::default(),
        }
    }
}

#[async_trait]
impl DenimEcPreKeyManager for InMemoryDenimEcPreKeyManager {
    async fn get_ec_pre_key<C: CryptoProvider>(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<EcPreKey, DenimKeyManagerError> {
        if let Some(pk) = self.manager.get_pre_key(account_id, device_id).await? {
            Ok(pk)
        } else {
            generate_ec_pre_keys::<C>(self, account_id, device_id, self.key_generate_amount)
                .await?;
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

    async fn next_key_id<R: Rng + Send>(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        rng: &mut R,
    ) -> Result<u32, DenimKeyManagerError> {
        let entry = DeviceAddress::new(account_id, device_id);

        for _ in 0..32 {
            // TODO: This should use an rng that the user has specified so that client and server
            // get same key ids.
            let key_id = rng.next_u32();
            let reserved = self
                .used_keys
                .lock()
                .await
                .get(&entry)
                .is_some_and(|ids| ids.contains(&key_id));

            if !reserved {
                self.used_keys
                    .lock()
                    .await
                    .entry(entry)
                    .or_default()
                    .push(key_id);
                return Ok(key_id);
            }
        }
        Err(DenimKeyManagerError::CouldNotGenerateKeyId)
    }

    async fn get_key_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<ChaChaRngState, DenimKeyManagerError> {
        self.key_seeds
            .lock()
            .await
            .get(&DeviceAddress::new(account_id, device_id))
            .ok_or(DenimKeyManagerError::NoSeed)
            .inspect_err(|_| debug!("No key seed found for {account_id}.{device_id}"))
            .cloned()
    }

    async fn store_key_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: ChaChaRngState,
    ) -> Result<(), DenimKeyManagerError> {
        self.key_seeds
            .lock()
            .await
            .insert(DeviceAddress::new(account_id, device_id), seed);
        Ok(())
    }

    async fn get_key_id_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<ChaChaRngState, DenimKeyManagerError> {
        self.id_seeds
            .lock()
            .await
            .get(&DeviceAddress::new(account_id, device_id))
            .ok_or(DenimKeyManagerError::NoSeed)
            .inspect_err(|_| debug!("No key id seed found for {account_id}.{device_id}"))
            .cloned()
    }

    async fn store_key_id_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: ChaChaRngState,
    ) -> Result<(), DenimKeyManagerError> {
        self.id_seeds
            .lock()
            .await
            .insert(DeviceAddress::new(account_id, device_id), seed);
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryDenimKeyManager;

impl DenimKeyManagerType for InMemoryDenimKeyManager {
    type EcPreKeyManager = InMemoryDenimEcPreKeyManager;

    type SignedPreKeyManager = InMemorySignedPreKeyManager;
}
