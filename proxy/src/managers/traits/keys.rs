use async_trait::async_trait;
use derive_more::{Display, Error, From};
use libsignal_protocol::IdentityKey;
use rand_chacha::ChaCha20Rng;
use sam_common::{
    api::{EcPreKey, SignedEcPreKey},
    AccountId,
};
use sam_server::managers::error::KeyManagerError;

#[derive(Debug, Display, Error, From)]
pub enum DenimKeyManagerError {
    Sam(KeyManagerError),
    NoSeed,
    NoKeyInStore,
}

#[async_trait]
pub trait DenimEcPreKeyManager: Send + Sync + Clone {
    async fn get_ec_pre_key(&self, account_id: AccountId)
        -> Result<EcPreKey, DenimKeyManagerError>;
    async fn add_ec_pre_key(
        &mut self,
        account_id: AccountId,
        pre_keys: EcPreKey,
    ) -> Result<(), DenimKeyManagerError>;
    async fn get_csprng_for(
        &self,
        account_id: AccountId,
    ) -> Result<ChaCha20Rng, DenimKeyManagerError>;
    async fn store_csprng_for(
        &mut self,
        account_id: AccountId,
        csprng: &ChaCha20Rng,
    ) -> Result<(), DenimKeyManagerError>;
    async fn remove_ec_pre_key(
        &mut self,
        account_id: AccountId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError>;
}

#[async_trait]
pub trait DenimSignedPreKeyManager: Send + Sync + Clone {
    async fn get_signed_pre_key(
        &self,
        account_id: AccountId,
    ) -> Result<SignedEcPreKey, DenimKeyManagerError>;
    async fn set_signed_pre_key(
        &mut self,
        account_id: AccountId,
        identity: &IdentityKey,
        key: SignedEcPreKey,
    ) -> Result<(), DenimKeyManagerError>;
    async fn remove_signed_pre_key(
        &mut self,
        account_id: AccountId,
    ) -> Result<(), DenimKeyManagerError>;
}
