pub mod in_mem;
mod traits;

pub use traits::BufferManager;
pub use traits::{DenimEcPreKeyManager, DenimKeyManagerError, DenimSignedPreKeyManager};

pub const DEFAULT_DEVICE: u32 = 0u32;

pub trait DenimKeyManagerType: Clone + Send + Sync {
    type EcPreKeyManager: DenimEcPreKeyManager;
    type SignedPreKeyManager: DenimSignedPreKeyManager;
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
