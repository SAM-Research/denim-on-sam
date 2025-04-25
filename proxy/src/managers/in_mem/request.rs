use std::collections::HashMap;

use sam_common::AccountId;

use crate::managers::traits::KeyRequestManager;

#[derive(Clone)]
pub struct InMemoryKeyRequestManager {
    requests: HashMap<AccountId, Vec<AccountId>>,
}

impl KeyRequestManager for InMemoryKeyRequestManager {
    fn store_request(&mut self, sender: AccountId, receiver: AccountId) {
        if let Some(vec) = self.requests.get_mut(&sender) {
            vec.push(receiver);
        }
    }

    fn get_requests(&mut self, receiver: AccountId) -> Option<Vec<AccountId>> {
        self.requests.remove(&receiver)
    }
}

impl InMemoryKeyRequestManager {
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
        }
    }
}
