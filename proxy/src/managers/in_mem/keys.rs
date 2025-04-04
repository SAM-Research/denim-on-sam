use denim_sam_common::Seed;
use libsignal_protocol::{InMemKyberPreKeyStore, InMemPreKeyStore};
use sam_common::AccountId;
use sam_server::managers::in_memory::keys::{
    InMemoryEcPreKeyManager, InMemoryPqPreKeyManager, InMemorySignedPreKeyManager,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::managers::DenimKeyManagerType;

const DEFAULT_DEVICE: u32 = 0u32;

type ArcMutexHashmap<K, V> = Arc<Mutex<HashMap<K, Arc<Mutex<V>>>>>;

#[derive(Default, Clone)]
struct EcKeyManager {
    _stores: ArcMutexHashmap<AccountId, InMemPreKeyStore>,
    _seeds: Arc<Mutex<HashMap<AccountId, Arc<Seed>>>>,
}

#[derive(Default, Clone)]
struct PqKeyManager {
    _stores: ArcMutexHashmap<AccountId, InMemKyberPreKeyStore>,
    _seeds: Arc<Mutex<HashMap<AccountId, Arc<Seed>>>>,
}

#[derive(Clone)]
pub struct InMemoryDenimKeyManager;

impl DenimKeyManagerType for InMemoryDenimKeyManager {
    type EcPreKeyManager = InMemoryEcPreKeyManager;

    type PqPreKeyManager = InMemoryPqPreKeyManager;

    type SignedPreKeyManager = InMemorySignedPreKeyManager;
}
