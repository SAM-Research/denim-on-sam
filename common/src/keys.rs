use sam_common::api::{EcPreKey, SignedEcPreKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreKeyBundle {
    pub device_id: u32,
    pub registration_id: u32,
    pub pre_key: EcPreKey,
    pub signed_pre_key: SignedEcPreKey,
}

impl PreKeyBundle {
    pub fn new(
        device_id: impl Into<u32>,
        registration_id: impl Into<u32>,
        pre_key: EcPreKey,
        signed_pre_key: SignedEcPreKey,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            registration_id: registration_id.into(),
            pre_key,
            signed_pre_key,
        }
    }
}
