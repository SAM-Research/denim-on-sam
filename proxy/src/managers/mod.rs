pub mod default;
pub mod error;
pub mod in_mem;
pub mod postgres;
pub mod traits;

pub use default::BufferManager;
use denim_sam_common::rng::chacha::ChaChaRngState;
pub use in_mem::InMemoryMessageIdProvider;
use sam_server::managers::traits::key_manager::SignedPreKeyManager;
pub use traits::DenimEcPreKeyManager;

pub trait DenimKeyManagerType: Clone + Send + Sync {
    type EcPreKeyManager: DenimEcPreKeyManager<ChaChaRngState>;
    type SignedPreKeyManager: SignedPreKeyManager;
}

#[derive(Debug, Clone)]
pub struct DenimKeyManager<T: DenimKeyManagerType> {
    pub pre_keys: T::EcPreKeyManager,
    pub signed_pre_keys: T::SignedPreKeyManager,
}

impl<T: DenimKeyManagerType> DenimKeyManager<T> {
    pub fn new(pre_keys: T::EcPreKeyManager, signed_pre_keys: T::SignedPreKeyManager) -> Self {
        Self {
            pre_keys,
            signed_pre_keys,
        }
    }
}
