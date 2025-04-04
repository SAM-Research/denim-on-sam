use async_trait::async_trait;
use derive_more::{Display, Error, From};
use libsignal_protocol::IdentityKey;
use sam_common::{
    api::{EcPreKey, PqPreKey, SignedEcPreKey},
    AccountId,
};
use sam_server::managers::error::KeyManagerError;

#[derive(Debug, Display, Error, From)]
pub enum DenimKeyManagerError {
    Sam(KeyManagerError),
}

#[async_trait]
pub trait DenimKeyManager:
    DenimEcKeyManager + DenimPostQuantumKeyManager + DenimSignedPreKeyManager
{
}

#[async_trait]
pub trait DenimSignedPreKeyManager {
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
}

#[async_trait]
pub trait DenimEcKeyManager: Send + Sync + Clone {
    async fn get_ec_pre_key(&self, account_id: AccountId)
        -> Result<EcPreKey, DenimKeyManagerError>;
    async fn remove_ec_pre_key(
        &mut self,
        account_id: AccountId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError>;
}

#[async_trait]
pub trait DenimPostQuantumKeyManager: Send + Sync + Clone {
    async fn get_pq_pre_key(&self, account_id: AccountId)
        -> Result<PqPreKey, DenimKeyManagerError>;
    async fn remove_pq_pre_key(
        &mut self,
        account_id: AccountId,
        id: u32,
    ) -> Result<(), DenimKeyManagerError>;
}
