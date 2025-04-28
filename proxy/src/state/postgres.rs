use denim_sam_common::crypto::ChaChaCryptoProvider;
use sam_server::managers::postgres::{PostgresAccountManager, PostgresDeviceManager};

use crate::managers::{in_mem::InMemoryKeyRequestManager, postgres::PostgresDenimKeyManager};

use super::{InMemoryBufferManagerType, StateType};

#[derive(Clone)]
pub struct PostgresStateType;

impl StateType for PostgresStateType {
    type BufferManager = InMemoryBufferManagerType;

    type DenimKeyManagerType = PostgresDenimKeyManager;

    type AccountManager = PostgresAccountManager;

    type DeviceManger = PostgresDeviceManager;

    type CryptoProvider = ChaChaCryptoProvider;

    type KeyRequestManager = InMemoryKeyRequestManager;
}
