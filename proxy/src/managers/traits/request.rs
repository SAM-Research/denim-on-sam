use async_trait::async_trait;
use sam_common::AccountId;

#[async_trait]
pub trait KeyRequestManager: Send + Sync + Clone {
    async fn store_receiver(&mut self, sender: AccountId, receiver: AccountId);
    async fn get_receivers(&mut self, account_id: AccountId) -> Option<Vec<AccountId>>;
}
