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

use super::error::EncryptionError;

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
        .message_type(MessageType::from(cipher.message_type().into()).into())
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
    let bytes = message_decrypt(
        &cipher,
        &addr,
        &mut deniable_store.session_store,
        &mut store.identity_key_store,
        &mut deniable_store.pre_key_store,
        &mut store.signed_pre_key_store,
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
