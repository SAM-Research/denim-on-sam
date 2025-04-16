use std::time::SystemTime;

use denim_sam_common::denim_message::{
    deniable_message::MessageKind, DeniableMessage, KeyResponse,
};
use libsignal_core::ProtocolAddress;
use libsignal_protocol::{process_prekey_bundle, IdentityKey};
use log::debug;
use rand::{CryptoRng, Rng};
use sam_client::storage::{MessageStore, Store, StoreType};
use sam_common::AccountId;

use crate::{
    encryption::{encrypt::decrypt, into_libsignal_bundle},
    store::{DeniableStore, DeniableStoreType},
};

use super::error::MessageProcessingError;

pub enum DenimResponse {
    KeyResponse(AccountId),
}

pub async fn process_deniable_message<R: Rng + CryptoRng>(
    message: DeniableMessage,
    store: &mut Store<impl StoreType>,
    deniable_store: &mut DeniableStore<impl DeniableStoreType>,
    rng: &mut R,
) -> Result<Option<DenimResponse>, MessageProcessingError> {
    let kind = message
        .message_kind
        .ok_or(MessageProcessingError::MessageKindWasNone)?;

    let envelope = match kind {
        MessageKind::DeniableMessage(message) => {
            decrypt(message, store, deniable_store, rng).await?
        }
        MessageKind::KeyResponse(res) => {
            return handle_key_response(res, store, deniable_store, rng)
                .await
                .map(Some);
        }
        MessageKind::Error(error) => {
            let account_id = AccountId::try_from(error.account_id().to_vec())
                .map_err(|_| MessageProcessingError::MalformedMessage)?;
            return Err(MessageProcessingError::ServerError(format!(
                "AccountId '{}', error '{}'",
                account_id, error.error
            )));
        }
        _ => Err(MessageProcessingError::MalformedMessage)?,
    };

    deniable_store.message_store.store_message(envelope).await?;
    Ok(None)
}

async fn handle_key_response<R: Rng + CryptoRng>(
    response: KeyResponse,
    store: &mut Store<impl StoreType>,
    deniable_store: &mut DeniableStore<impl DeniableStoreType>,
    rng: &mut R,
) -> Result<DenimResponse, MessageProcessingError> {
    let account_id = AccountId::try_from(response.account_id)
        .inspect_err(|e| debug!("{e}"))
        .map_err(|_| MessageProcessingError::MalformedMessage)?;
    let id_key = IdentityKey::decode(&response.identity_key)?;
    let device_id = response.key_bundle.device_id;

    let signal_bundle = into_libsignal_bundle(&id_key, response.key_bundle)?;
    let addr = ProtocolAddress::new(account_id.to_string(), device_id.into());
    process_prekey_bundle(
        &addr,
        &mut deniable_store.session_store,
        &mut store.identity_key_store,
        &signal_bundle,
        SystemTime::now(),
        rng,
    )
    .await?;
    Ok(DenimResponse::KeyResponse(account_id))
}
