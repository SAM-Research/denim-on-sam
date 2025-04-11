use denim_sam_common::denim_message::KeyBundle;
use libsignal_protocol::{IdentityKey, PreKeyBundle, PublicKey};
use sam_common::api::{Decode, EcPreKey, Key, SignedEcPreKey, SignedKey};

use super::error::KeyError;

pub fn into_libsignal_bundle(
    identity_key: &IdentityKey,
    key_bundle: KeyBundle,
) -> Result<PreKeyBundle, KeyError> {
    let pre_key = EcPreKey::decode(&key_bundle.pre_key)?;
    let signed_pre_key = SignedEcPreKey::decode(&key_bundle.signed_pre_key)?;

    Ok(PreKeyBundle::new(
        key_bundle.registration_id,
        key_bundle.device_id.into(),
        Some((
            pre_key.id().into(),
            PublicKey::deserialize(pre_key.public_key())?,
        )),
        (signed_pre_key.id()).into(),
        PublicKey::deserialize(&signed_pre_key.public_key())?,
        signed_pre_key.signature().to_vec(),
        identity_key.clone(),
    )?)
}
