use std::{collections::HashMap, sync::Arc};

use sam_common::{api::SignedEcPreKey, AccountId};
use tokio::sync::Mutex;

use crate::managers::{entities::Seed, KeyDistributionCenter};

#[derive(Debug, Default, Clone)]
pub struct InMemoryKeyDistributionCenter {
    _seeds: Arc<Mutex<HashMap<AccountId, Seed>>>,
    _signed_pre_keys: Arc<Mutex<HashMap<AccountId, SignedEcPreKey>>>,
}

impl KeyDistributionCenter for InMemoryKeyDistributionCenter {}
