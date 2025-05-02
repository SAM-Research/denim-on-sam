use async_trait::async_trait;
use sam_common::AccountId;

#[async_trait]
pub trait KeyRequestManager: Send + Sync + Clone {
    // requested is an account that have yet to upload seed
    // requester is an account that want keys from an account that have not uploaded seed

    async fn store_requester(&mut self, requested: AccountId, requester: AccountId);
    async fn remove_requesters(&mut self, requested: AccountId) -> Option<Vec<AccountId>>;
}
