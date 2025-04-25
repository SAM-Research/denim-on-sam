use denim_sam_common::{
    denim_message::{deniable_message::MessageKind, DeniableMessage, KeyResponse},
    Seed,
};

use sam_common::AccountId;
use sam_server::managers::traits::account_manager::AccountManager;

use crate::{
    error::{DenimRouterError, LogicError},
    logic::keys::{get_keys_for, update_seed},
    managers::{default::ClientRequest, error::DenimKeyManagerError, traits::KeyRequestManager},
    state::{DenimState, StateType},
};

pub async fn denim_router<T: StateType>(
    state: &mut DenimState<T>,
    request: ClientRequest,
    account_id: AccountId,
) -> Result<(), DenimRouterError> {
    match request {
        ClientRequest::BlockRequest(_, _block_request) => todo!(),
        ClientRequest::KeyRequest(msg_id, key_request) => {
            let requested_account_id = AccountId::try_from(key_request.account_id)
                .map_err(|_| DenimRouterError::KeyRequestMalformed)?;

            let key_bundle = match get_keys_for(
                state,
                requested_account_id,
                key_request.specific_device_ids[0].into(),
            )
            .await
            {
                Ok(key_bundle) => key_bundle,
                Err(LogicError::KeyManager(DenimKeyManagerError::NoSeed)) => {
                    state
                        .key_request_manager
                        .store_request(requested_account_id, account_id);
                    return Ok(());
                }
                Err(err) => return Err(DenimRouterError::Logic(err)),
            };
            let identity_key = state
                .accounts
                .get_account(requested_account_id)
                .await?
                .identity()
                .to_owned();

            let key_response = KeyResponse::builder()
                .account_id(requested_account_id.into())
                .identity_key(identity_key.public_key().public_key_bytes().to_owned())
                .key_bundle(key_bundle)
                .build();

            state
                .buffer_manager
                .enqueue_message(
                    account_id,
                    DeniableMessage::builder()
                        .message_kind(MessageKind::KeyResponse(key_response))
                        .message_id(msg_id)
                        .build(),
                )
                .await?;

            Ok(())
        }
        ClientRequest::KeyRefillRequest(_, _key_update) => todo!(),
        ClientRequest::SeedUpdateRequest(msg_id, seed_update) => {
            let seed: [u8; 32] = seed_update
                .pre_key_seed
                .try_into()
                .map_err(|_| DenimRouterError::FailedToConvertSeed)?;

            update_seed(state, account_id, 1.into(), Seed::new(seed)).await?;

            if let Some(receivers) = state.key_request_manager.get_requests(account_id) {
                for receiver in receivers {
                    let key_bundle = get_keys_for(state, account_id, 1.into()).await?;
                    let identity_key = state
                        .accounts
                        .get_account(account_id)
                        .await?
                        .identity()
                        .to_owned();

                    state
                        .buffer_manager
                        .enqueue_message(
                            receiver,
                            DeniableMessage::builder()
                                .message_kind(MessageKind::KeyResponse(
                                    KeyResponse::builder()
                                        .account_id(account_id.into())
                                        .identity_key(
                                            identity_key.public_key().public_key_bytes().to_owned(),
                                        )
                                        .key_bundle(key_bundle)
                                        .build(),
                                ))
                                .message_id(msg_id)
                                .build(),
                        )
                        .await?;
                }
            }

            Ok(())
        }
    }
}
