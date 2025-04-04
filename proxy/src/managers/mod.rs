pub mod in_mem;
mod traits;

use sam_server::managers::traits::key_manager::{
    EcPreKeyManager, PqPreKeyManager, SignedPreKeyManager,
};
pub use traits::BufferManager;
pub use traits::{
    DenimEcKeyManager, DenimKeyManagerError, DenimPostQuantumKeyManager, DenimSignedPreKeyManager,
};

pub trait DenimKeyManagerType: Clone + Send + Sync {
    type EcPreKeyManager: EcPreKeyManager;
    type PqPreKeyManager: PqPreKeyManager;
    type SignedPreKeyManager: SignedPreKeyManager;
}

#[derive(Debug, Clone)]
pub struct DenimKeyManager<T: DenimKeyManagerType> {
    pre_keys: T::EcPreKeyManager,
    pq_pre_keys: T::PqPreKeyManager,
    signed_pre_keys: T::SignedPreKeyManager,
}

impl<T: DenimKeyManagerType> DenimKeyManager<T> {
    pub fn new(
        pre_keys: T::EcPreKeyManager,
        pq_pre_keys: T::PqPreKeyManager,
        signed_pre_keys: T::SignedPreKeyManager,
    ) -> Self {
        Self {
            pre_keys,
            pq_pre_keys,
            signed_pre_keys,
        }
    }
}
