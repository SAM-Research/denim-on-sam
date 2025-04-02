use std::{collections::HashMap, sync::Arc};

use denim_sam_common::Seed;
use sam_common::{api::SignedEcPreKey, AccountId};

use tokio::sync::Mutex;

use crate::managers::DenimKeyManager;

#[derive(Debug, Default, Clone)]
pub struct InMemoryDenimKeyManager {
    _seeds: Arc<Mutex<HashMap<AccountId, Seed>>>,
    _signed_pre_keys: Arc<Mutex<HashMap<AccountId, SignedEcPreKey>>>,
}

impl DenimKeyManager for InMemoryDenimKeyManager {}
