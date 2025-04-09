use denim_sam_common::denim_message::{
    deniable_message::MessageKind, DeniableMessage, KeyResponse,
};
use rand::{CryptoRng, Rng};
use sam_client::storage::{MessageStore, Store, StoreType};
use sam_common::AccountId;

use crate::{
    encryption::encrypt::decrypt,
    store::{DeniableStore, DeniableStoreType},
};

use super::error::MessageProcessingError;

pub async fn process_deniable_message<R: Rng + CryptoRng>(
    message: DeniableMessage,
    store: &mut Store<impl StoreType>,
    deniable_store: &mut DeniableStore<impl DeniableStoreType>,
    rng: &mut R,
) -> Result<(), MessageProcessingError> {
    let kind = message
        .message_kind
        .ok_or(MessageProcessingError::MessageKindWasNone)?;
    let envelope = match kind {
        MessageKind::KeyRequest(_)
        | MessageKind::BlockRequest(_)
        | MessageKind::KeyRefill(_)
        | MessageKind::SeedUpdate(_) => Err(MessageProcessingError::MalformedMessage)?,

        MessageKind::DeniableMessage(message) => {
            decrypt(message, store, deniable_store, rng).await?
        }
        MessageKind::KeyResponse(res) => {
            handle_key_response(res, store, deniable_store, rng).await?;
            return Ok(());
        }
        MessageKind::Error(error) => {
            let account_id = AccountId::try_from(error.account_id().to_vec())
                .map_err(|_| MessageProcessingError::MalformedMessage)?;
            return Err(MessageProcessingError::ServerError(format!(
                "AccountId '{}', error '{}'",
                account_id, error.error
            )));
        }
    };

    deniable_store.message_store.store_message(envelope).await?;
    Ok(())
}

async fn handle_key_response<R: Rng + CryptoRng>(
    _response: KeyResponse,
    _store: &mut Store<impl StoreType>,
    _deniable_store: &mut DeniableStore<impl DeniableStoreType>,
    _rng: &mut R,
) -> Result<(), MessageProcessingError> {
    todo!("Implement this when proxy implements denim routing")
}
