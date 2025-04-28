use denim_sam_common::ChaChaRngState;
use sam_server::managers::postgres::keys::PostgresSignedPreKeyManager;

use super::{in_mem::InMemoryDenimEcPreKeyManager, DenimKeyManagerType};

#[derive(Clone)]
pub struct PostgresDenimKeyManager;

impl DenimKeyManagerType for PostgresDenimKeyManager {
    type EcPreKeyManager = InMemoryDenimEcPreKeyManager<ChaChaRngState>;

    type SignedPreKeyManager = PostgresSignedPreKeyManager;
}
