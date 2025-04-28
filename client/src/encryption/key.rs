use denim_sam_common::{denim_message::KeyBundle, rng::RngState as _};
use libsignal_protocol::{IdentityKey, PreKeyBundle, PreKeyId, PreKeyStore, PublicKey};
use log::debug;
use rand::{CryptoRng, Rng};
use sam_common::api::{Decode, EcPreKey, Key, SignedEcPreKey, SignedKey};
use sam_security::key_gen::generate_ec_pre_key;

use crate::store::{DeniableStore, DeniableStoreType, DenimPreKeySeedStore};

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
        PublicKey::deserialize(signed_pre_key.public_key())?,
        signed_pre_key.signature().to_vec(),
        *identity_key,
    )?)
}

async fn generate_key_id(csprng: &mut (impl Rng + CryptoRng)) -> PreKeyId {
    csprng.next_u32().into()
}

pub async fn generate_key<T: DeniableStoreType>(
    prekey_id: PreKeyId,
    store: &mut DeniableStore<T>,
) -> Result<(), KeyError> {
    while store.pre_key_store.get_pre_key(prekey_id).await.is_err() {
        let mut id_rng = store.seed_store.get_key_id_seed().await?.into_rng();
        let key_id = generate_key_id(&mut id_rng).await;

        let mut key_rng = store.seed_store.get_key_seed().await?.into_rng();
        let pre_key = generate_ec_pre_key(key_id, &mut key_rng).await;
        store.pre_key_store.save_pre_key(key_id, &pre_key).await?;
        debug!("Successfully stored new prekey.");
    }
    Ok(())
}
