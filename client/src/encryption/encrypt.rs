use std::time::SystemTime;

use denim_sam_common::denim_message::{MessageType, UserMessage};
use libsignal_protocol::{
    message_decrypt, message_encrypt, CiphertextMessage, PlaintextContent, PreKeySignalMessage,
    ProtocolAddress, SenderKeyMessage, SignalMessage,
};

use log::debug;
use rand::{CryptoRng, Rng};
use sam_client::{
    encryption::DecryptedEnvelope,
    storage::{Store, StoreType},
};
use sam_common::AccountId;

use crate::store::{DeniableStore, DeniableStoreType};

use super::{error::EncryptionError, key::generate_key};

pub async fn encrypt(
    message: Vec<u8>,
    recipient: AccountId,
    store: &mut Store<impl StoreType>,
    deniable_store: &mut DeniableStore<impl DeniableStoreType>,
) -> Result<UserMessage, EncryptionError> {
    let addr = ProtocolAddress::new(recipient.to_string(), 1.into());

    let cipher = message_encrypt(
        &message,
        &addr,
        &mut deniable_store.session_store,
        &mut store.identity_key_store,
        SystemTime::now(),
    )
    .await?;
    Ok(UserMessage::builder()
        .account_id(recipient.into())
        .message_type(MessageType::from(cipher.message_type()).into())
        .content(cipher.serialize().into())
        .build())
}

pub async fn decrypt<R: Rng + CryptoRng>(
    message: UserMessage,
    store: &mut Store<impl StoreType>,
    deniable_store: &mut DeniableStore<impl DeniableStoreType>,
    rng: &mut R,
) -> Result<DecryptedEnvelope, EncryptionError> {
    let cipher = match message.message_type() {
        MessageType::SignalMessage => {
            CiphertextMessage::SignalMessage(SignalMessage::try_from(message.content.as_slice())?)
        }
        MessageType::PreKeySignalMessage => CiphertextMessage::PreKeySignalMessage(
            PreKeySignalMessage::try_from(message.content.as_slice())?,
        ),
        MessageType::SenderKeyMessage => CiphertextMessage::SenderKeyMessage(
            SenderKeyMessage::try_from(message.content.as_slice())?,
        ),
        MessageType::PlaintextContent => CiphertextMessage::PlaintextContent(
            PlaintextContent::try_from(message.content.as_slice())?,
        ),
    };

    let source = AccountId::try_from(message.account_id)
        .inspect_err(|e| debug!("{e}"))
        .map_err(|_| EncryptionError::InvalidAccountId)?;

    let addr = ProtocolAddress::new(source.to_string(), 1.into());

    // A deniable message must contain a PreKey.
    match cipher {
        CiphertextMessage::PreKeySignalMessage(ref prekey_message) => {
            let key_id = prekey_message
                .pre_key_id()
                .ok_or(EncryptionError::NoPreKeyInMessage)?;
            generate_key(key_id, deniable_store).await?;
        }
        _ => {}
    }

    let bytes = message_decrypt(
        &cipher,
        &addr,
        &mut deniable_store.session_store,
        &mut store.identity_key_store,
        &mut deniable_store.pre_key_store,
        &store.signed_pre_key_store,
        &mut store.kyber_pre_key_store,
        rng,
    )
    .await?;

    Ok(DecryptedEnvelope::builder()
        .source_account_id(source)
        .source_device_id(1.into())
        .content(bytes)
        .build())
}

#[cfg(test)]
mod test {
    use std::time::SystemTime;

    use denim_sam_common::denim_message::{KeyBundle, UserMessage};
    use libsignal_core::ProtocolAddress;
    use libsignal_protocol::{
        process_prekey_bundle, IdentityKeyPair, IdentityKeyStore, PreKeyBundle, PreKeyId,
        PreKeyStore,
    };
    use rand::{rngs::OsRng, CryptoRng, Rng};
    use rstest::rstest;
    use sam_client::storage::{
        key_generation::{KyberKeyGenerator, SignedPreKeyGenerator},
        AccountStore, InMemoryStoreConfig, InMemoryStoreType, Store, StoreConfig, StoreType,
    };
    use sam_common::{
        address::RegistrationId,
        api::{EcPreKey, Encode, SignedEcPreKey},
        AccountId,
    };
    use sam_security::key_gen::generate_ec_pre_key;

    use crate::{
        encryption::{decrypt, encrypt, key::into_libsignal_bundle},
        store::{
            inmem::InMemoryDeniableStoreType, DeniableStore, DeniableStoreConfig,
            DeniableStoreType, InMemoryDeniableStoreConfig,
        },
    };

    async fn stores<R: Rng + CryptoRng>(
        csprng: &mut R,
    ) -> (
        Store<InMemoryStoreType>,
        DeniableStore<InMemoryDeniableStoreType>,
    ) {
        let key_pair = IdentityKeyPair::generate(csprng);
        let account_id = AccountId::generate();
        let mut sam = InMemoryStoreConfig::default()
            .create_store(key_pair, RegistrationId::generate(csprng))
            .await
            .expect("can create sam store");
        sam.account_store
            .set_account_id(account_id)
            .await
            .expect("can set account id");
        sam.account_store
            .set_device_id(1.into())
            .await
            .expect("can set device id");
        let denim = InMemoryDeniableStoreConfig::default()
            .create_store()
            .await
            .expect("can create denim store");

        (sam, denim)
    }

    async fn pre_key_bundle<R: Rng + CryptoRng>(
        sam_store: &mut Store<impl StoreType>,
        denim_store: &mut DeniableStore<impl DeniableStoreType>,
        pre_key_id: PreKeyId,
        quantum: bool,
        csprng: &mut R,
    ) -> PreKeyBundle {
        let pair = sam_store
            .identity_key_store
            .get_identity_key_pair()
            .await
            .expect("Can get identity");
        let registration_id = sam_store
            .identity_key_store
            .get_local_registration_id()
            .await
            .expect("Can get reg id");

        // proxy generates this
        let ec_rec = generate_ec_pre_key(pre_key_id, csprng).await;
        denim_store
            .pre_key_store
            .save_pre_key(pre_key_id, &ec_rec)
            .await
            .expect("Can save ec pre key from KDC");

        let signed_ec_rec = sam_store
            .signed_pre_key_store
            .generate_key(csprng, pair.private_key())
            .await
            .expect("Can generate signed");
        if quantum {
            sam_store
                .kyber_pre_key_store
                .generate_key(pair.private_key())
                .await
                .expect("can generate kyber");
        }

        let ec_key = EcPreKey::from(ec_rec);
        let signed_ec_key = SignedEcPreKey::from(signed_ec_rec);

        let bundle = KeyBundle {
            device_id: 1u32,
            registration_id,
            pre_key: ec_key.encode().expect("Can encode ec"),
            signed_pre_key: signed_ec_key.encode().expect("Can encode signed ec"),
        };
        into_libsignal_bundle(pair.identity_key(), bundle).expect("Can create bundle")
    }

    fn rand_string(y: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..y).map(|_| rng.gen::<char>()).collect()
    }

    async fn encrypt_message(
        sam_store: &mut Store<impl StoreType>,
        denim_store: &mut DeniableStore<impl DeniableStoreType>,
        sender: AccountId,
        receiver: AccountId,
    ) -> (String, UserMessage) {
        let expected = rand_string(12);
        let mut cipher = encrypt(
            expected.clone().into_bytes(),
            receiver,
            sam_store,
            denim_store,
        )
        .await
        .expect("can encrypt");
        // proxy changes account id to sender
        cipher.account_id = sender.into();

        (expected, cipher)
    }

    async fn decrypt_message<R: CryptoRng + Rng>(
        sam_store: &mut Store<impl StoreType>,
        denim_store: &mut DeniableStore<impl DeniableStoreType>,
        cipher: UserMessage,
        csprng: &mut R,
    ) -> String {
        let env = decrypt(cipher, sam_store, denim_store, csprng)
            .await
            .expect("can decrypt message from alice");
        String::from_utf8(env.content_bytes().to_vec()).expect("Can decode")
    }

    #[rstest]
    #[case(false, 1)]
    #[case(false, 10)]
    #[case(true, 1)]
    #[case(true, 10)]
    #[tokio::test]
    async fn can_encrypt_denim(#[case] quantum: bool, #[case] message_count: usize) {
        let mut csprng = OsRng;
        let (mut a_sam, mut a_denim) = stores(&mut csprng).await;
        let (mut b_sam, mut b_denim) = stores(&mut csprng).await;

        let a_acid = a_sam
            .account_store
            .get_account_id()
            .await
            .expect("can get acid");
        let b_acid = b_sam
            .account_store
            .get_account_id()
            .await
            .expect("can get acid");
        let b_addr = ProtocolAddress::new(b_acid.to_string(), 1.into());
        let b_bundle =
            pre_key_bundle(&mut b_sam, &mut b_denim, 1.into(), quantum, &mut csprng).await;

        process_prekey_bundle(
            &b_addr,
            &mut a_denim.session_store,
            &mut a_sam.identity_key_store,
            &b_bundle,
            SystemTime::now(),
            &mut csprng,
        )
        .await
        .expect("Alice can process bob bundle");

        for i in 0..message_count {
            let (sender, ids, receiver) = if i & 1 == 0 {
                (
                    (&mut a_sam, &mut a_denim),
                    (a_acid, b_acid),
                    (&mut b_sam, &mut b_denim),
                )
            } else {
                (
                    (&mut b_sam, &mut b_denim),
                    (b_acid, a_acid),
                    (&mut a_sam, &mut a_denim),
                )
            };

            let (s_sam, s_denim) = sender;
            let (s_acid, r_acid) = ids;
            let (r_sam, r_denim) = receiver;

            let (expected_message, cipher) = encrypt_message(s_sam, s_denim, s_acid, r_acid).await;
            let actual_message = decrypt_message(r_sam, r_denim, cipher, &mut csprng).await;
            assert_eq!(
                actual_message, expected_message,
                "Expected '{}' got '{}",
                expected_message, actual_message
            );
        }
    }
}
