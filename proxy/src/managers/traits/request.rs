use sam_common::AccountId;

pub trait KeyRequestManager: Send + Sync + Clone {
    fn store_request(&mut self, sender: AccountId, receiver: AccountId);
    fn get_requests(&mut self, account_id: AccountId) -> Option<Vec<AccountId>>;
}
