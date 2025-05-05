use denim_sam_common::{denim_message::KeyBundle, rng::RngState as _};
use libsignal_protocol::{IdentityKey, PreKeyBundle, PreKeyId, PreKeyStore, PublicKey};
use log::debug;
use rand::RngCore;
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

pub async fn generate_key<T: DeniableStoreType>(
    rng_counter: u64,
    prekey_id: PreKeyId,
    store: &mut DeniableStore<T>,
) -> Result<(), KeyError> {
    debug!("Searching for prekey {prekey_id}. Counter from server: {rng_counter}");

    while store.seed_store.get_rng_offset().await? <= rng_counter as u128 {
        let mut id_rng = store.seed_store.get_key_id_seed().await?.into_rng();
        let key_id = id_rng.next_u32().into();
        store.seed_store.set_key_id_seed(id_rng.into()).await?;

        let mut key_rng = store.seed_store.get_key_seed().await?.into_rng();
        let pre_key = generate_ec_pre_key(key_id, &mut key_rng).await;
        store.pre_key_store.save_pre_key(key_id, &pre_key).await?;
        store.seed_store.set_key_seed(key_rng.into()).await?;
        debug!("Successfully stored new prekey '{key_id}'.");
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use denim_sam_common::rng::{chacha::ChaChaRngState, RngState as _};
    use libsignal_protocol::PreKeyStore;
    use rand::{rngs::OsRng, RngCore};
    use rstest::rstest;

    use crate::store::DeniableStoreConfig;
    use crate::store::DenimPreKeySeedStore;
    use crate::store::InMemoryDeniableStoreConfig;

    use super::generate_key;

    #[rstest]
    #[case(InMemoryDeniableStoreConfig::default())]
    #[tokio::test]
    async fn generate_key_generates_key_with_right_id_eventually(
        #[case] store_config: impl DeniableStoreConfig,
    ) {
        let id_rng_state_1 = ChaChaRngState::random(&mut OsRng);
        let id_rng_state_2 = id_rng_state_1.clone();

        let mut id_rng_1 = id_rng_state_1.into_rng();

        let mut store = store_config.create_store().await.expect("Can create store");

        store
            .seed_store
            .set_key_id_seed(id_rng_state_2)
            .await
            .expect("can save key seed");

        store
            .seed_store
            .set_key_seed(ChaChaRngState::random(&mut OsRng))
            .await
            .expect("can save key id seed");

        for _ in 0..10 {
            let _ = id_rng_1.next_u32();
        }

        let key_id = id_rng_1.next_u32().into();
        assert!(generate_key(11, key_id, &mut store).await.is_ok());

        assert!(store.pre_key_store.get_pre_key(key_id).await.is_ok())
    }
}
