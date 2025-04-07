use async_trait::async_trait;
use denim_sam_common::Seed;
use libsignal_protocol::IdentityKey;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sam_common::{
    api::{EcPreKey, SignedEcPreKey},
    AccountId,
};
use sam_server::managers::{
    in_memory::keys::{InMemoryEcPreKeyManager, InMemorySignedPreKeyManager},
    traits::key_manager::{EcPreKeyManager as _, SignedPreKeyManager},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::managers::{
    DenimEcPreKeyManager, DenimKeyManagerError, DenimKeyManagerType, DenimSignedPreKeyManager,
    DEFAULT_DEVICE,
};

#[derive(Default, Clone)]
pub struct InMemoryDenimEcPreKeyManager {
    manager: InMemoryEcPreKeyManager,
    seeds: Arc<Mutex<HashMap<AccountId, (Seed, u128)>>>,
}

#[async_trait]
impl DenimEcPreKeyManager for InMemoryDenimEcPreKeyManager {
    async fn get_ec_pre_key(
        &self,
        account_id: AccountId,
    ) -> Result<EcPreKey, DenimKeyManagerError> {
        let pk = self
            .manager
            .get_pre_key(account_id, DEFAULT_DEVICE.into())
            .await?
            .ok_or(DenimKeyManagerError::NoKeyInStore)?;

        Ok(pk)
    }

    async fn add_ec_pre_key(
        &mut self,
        account_id: AccountId,
        pre_key: EcPreKey,
    ) -> Result<(), DenimKeyManagerError> {
        Ok(self
            .manager
            .add_pre_key(account_id, DEFAULT_DEVICE.into(), pre_key)
            .await?)
    }

    async fn get_csprng_for(
        &self,
        account_id: AccountId,
    ) -> Result<ChaCha20Rng, DenimKeyManagerError> {
        let res = self
            .seeds
            .lock()
            .await
            .get(&account_id)
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
        csprng: &ChaCha20Rng,
    ) -> Result<(), DenimKeyManagerError> {
        let seed = csprng.get_seed();
        let offset = csprng.get_word_pos();

        self.seeds
            .lock()
            .await
            .insert(account_id, (seed.into(), offset));

        Ok(())
    }

    async fn remove_ec_pre_key(
        &mut self,
        account_id: AccountId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError> {
        Ok(self
            .manager
            .remove_pre_key(account_id, DEFAULT_DEVICE.into(), id)
            .await?)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryDenimSignedPreKeyManager {
    manager: InMemorySignedPreKeyManager,
}

#[async_trait]
impl DenimSignedPreKeyManager for InMemoryDenimSignedPreKeyManager {
    async fn get_signed_pre_key(
        &self,
        account_id: AccountId,
    ) -> Result<SignedEcPreKey, DenimKeyManagerError> {
        Ok(self
            .manager
            .get_signed_pre_key(account_id, DEFAULT_DEVICE.into())
            .await?)
    }
    async fn set_signed_pre_key(
        &mut self,
        account_id: AccountId,
        identity: &IdentityKey,
        key: SignedEcPreKey,
    ) -> Result<(), DenimKeyManagerError> {
        Ok(self
            .manager
            .set_signed_pre_key(account_id, DEFAULT_DEVICE.into(), identity, key)
            .await?)
    }
    async fn remove_signed_pre_key(
        &mut self,
        account_id: AccountId,
    ) -> Result<(), DenimKeyManagerError> {
        self.manager
            .remove_signed_pre_key(account_id, DEFAULT_DEVICE.into())
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryDenimKeyManager;

impl DenimKeyManagerType for InMemoryDenimKeyManager {
    type EcPreKeyManager = InMemoryDenimEcPreKeyManager;

    type SignedPreKeyManager = InMemoryDenimSignedPreKeyManager;
}
