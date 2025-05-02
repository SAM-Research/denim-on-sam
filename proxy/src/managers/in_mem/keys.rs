use async_trait::async_trait;
use denim_sam_common::rng::{chacha::ChaChaRngState, RngState};
use futures_util::TryFutureExt;
use log::debug;
use rand::Rng;
use sam_common::{
    address::DeviceAddress,
    api::{EcPreKey, Key},
    AccountId, DeviceId,
};
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]

struct PendingId {
    pre_key_message_sender: AccountId,
    address: DeviceAddress,
}

impl PendingId {
    fn new(pre_key_message_sender: AccountId, address: DeviceAddress) -> Self {
        Self {
            pre_key_message_sender,
            address,
        }
    }
}

#[derive(Clone)]
pub struct InMemoryDenimEcPreKeyManager<T: RngState> {
    key_generate_amount: usize,
    unused_keys: InMemoryEcPreKeyManager,
    id_seeds: Arc<Mutex<HashMap<DeviceAddress, Option<T>>>>,
    key_seeds: Arc<Mutex<HashMap<DeviceAddress, Option<T>>>>,
    pending_keys: Arc<Mutex<HashMap<PendingId, u32>>>,
    used_keys: InMemoryEcPreKeyManager,
}

impl<T: RngState> InMemoryDenimEcPreKeyManager<T> {
    pub fn new(key_generate_amount: usize) -> Self {
        Self {
            key_generate_amount,
            ..Default::default()
        }
    }
}

impl<T: RngState> Default for InMemoryDenimEcPreKeyManager<T> {
    fn default() -> Self {
        Self {
            key_generate_amount: 10,
            unused_keys: InMemoryEcPreKeyManager::default(),
            id_seeds: Arc::default(),
            key_seeds: Arc::default(),
            used_keys: InMemoryEcPreKeyManager::default(),
            pending_keys: Arc::default(),
        }
    }
}

#[async_trait]
impl<T: RngState> DenimEcPreKeyManager<T> for InMemoryDenimEcPreKeyManager<T> {
    async fn get_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<EcPreKey, DenimKeyManagerError> {
        if let Some(pk) = self.unused_keys.get_pre_key(account_id, device_id).await? {
            self.unused_keys
                .remove_pre_key(account_id, device_id, pk.id())
                .await?;
            self.used_keys
                .add_pre_key(account_id, device_id, pk.clone())
                .await?;
            Ok(pk)
        } else {
            generate_ec_pre_keys(self, account_id, device_id, self.key_generate_amount).await?;
            let pk = self
                .unused_keys
                .get_pre_key(account_id, device_id)
                .await?
                .ok_or(DenimKeyManagerError::NoKeyInStore)?;
            self.unused_keys
                .remove_pre_key(account_id, device_id, pk.id())
                .await?;
            self.used_keys
                .add_pre_key(account_id, device_id, pk.clone())
                .await?;
            Ok(pk)
        }
    }

    async fn get_ec_pre_key_ids(
        &self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<Vec<u32>, DenimKeyManagerError> {
        Ok(self
            .unused_keys
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
        self.unused_keys
            .add_pre_key(account_id, device_id, key)
            .await?;
        Ok(())
    }

    async fn remove_ec_pre_key(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError> {
        self.unused_keys
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
        for _ in 0..32 {
            let key_id = rng.next_u32();
            let reserved = self
                .used_keys
                .get_pre_key_ids(account_id, device_id)
                .await
                .unwrap_or_default()
                .unwrap_or_default()
                .iter()
                .any(|id| *id == key_id);

            if !reserved {
                return Ok(key_id);
            }
        }
        Err(DenimKeyManagerError::CouldNotGenerateKeyId)
    }

    async fn get_key_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<T, DenimKeyManagerError> {
        self.key_seeds
            .lock()
            .await
            .get(&DeviceAddress::new(account_id, device_id))
            .cloned()
            .flatten()
            .ok_or(DenimKeyManagerError::NoSeed)
            .inspect_err(|_| debug!("No key seed found for {account_id}.{device_id}"))
    }

    async fn store_key_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: T,
    ) -> Result<(), DenimKeyManagerError> {
        self.key_seeds
            .lock()
            .await
            .insert(DeviceAddress::new(account_id, device_id), Some(seed));
        Ok(())
    }

    async fn get_key_id_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<T, DenimKeyManagerError> {
        self.id_seeds
            .lock()
            .await
            .get(&DeviceAddress::new(account_id, device_id))
            .cloned()
            .flatten()
            .ok_or(DenimKeyManagerError::NoSeed)
            .inspect_err(|_| debug!("No key id seed found for {account_id}.{device_id}"))
    }

    async fn store_key_id_seed_for(
        &mut self,
        account_id: AccountId,
        device_id: DeviceId,
        seed: T,
    ) -> Result<(), DenimKeyManagerError> {
        self.id_seeds
            .lock()
            .await
            .insert(DeviceAddress::new(account_id, device_id), Some(seed));
        Ok(())
    }

    async fn store_pending_key(
        &mut self,
        pre_key_msg_sender: AccountId,
        account_id: AccountId,
        device_id: DeviceId,
        key_id: u32,
    ) -> Result<(), DenimKeyManagerError> {
        let mut pending_guard = self.pending_keys.lock().await;
        let id = PendingId::new(
            pre_key_msg_sender,
            DeviceAddress::new(account_id, device_id),
        );
        if pending_guard.contains_key(&id) {
            return Err(DenimKeyManagerError::AlreadyPending);
        }
        pending_guard.insert(id, key_id);
        Ok(())
    }

    async fn has_pending_key(
        &self,
        pre_key_msg_sender: AccountId,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> bool {
        let id = PendingId::new(
            pre_key_msg_sender,
            DeviceAddress::new(account_id, device_id),
        );
        self.pending_keys.lock().await.contains_key(&id)
    }

    async fn remove_pending_key(
        &mut self,
        pre_key_msg_sender: AccountId,
        account_id: AccountId,
        device_id: DeviceId,
    ) -> Result<u32, DenimKeyManagerError> {
        let mut pending_guard = self.pending_keys.lock().await;
        let id = PendingId::new(
            pre_key_msg_sender,
            DeviceAddress::new(account_id, device_id),
        );
        if let Some(key_id) = pending_guard.remove(&id) {
            Ok(key_id)
        } else {
            Err(DenimKeyManagerError::NotPending)
        }
    }
}

#[derive(Clone)]
pub struct InMemoryDenimKeyManager;

impl DenimKeyManagerType for InMemoryDenimKeyManager {
    type EcPreKeyManager = InMemoryDenimEcPreKeyManager<ChaChaRngState>;

    type SignedPreKeyManager = InMemorySignedPreKeyManager;
}
