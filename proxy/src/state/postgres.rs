use sam_server::managers::postgres::{PostgresAccountManager, PostgresDeviceManager};

use crate::managers::{in_mem::InMemoryKeyRequestManager, postgres::PostgresDenimKeyManager};

use super::{DenimStateType, InMemoryBufferManagerType};

#[derive(Clone)]
pub struct PostgresDenimStateType;

impl DenimStateType for PostgresDenimStateType {
    type BufferManager = InMemoryBufferManagerType;

    type DenimKeyManagerType = PostgresDenimKeyManager;

    type AccountManager = PostgresAccountManager;

    type DeviceManger = PostgresDeviceManager;

    type KeyRequestManager = InMemoryKeyRequestManager;
}
