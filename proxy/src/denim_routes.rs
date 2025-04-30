use denim_sam_common::{
    denim_message::{
        deniable_message::MessageKind, BlockRequest, DeniableMessage, KeyRequest, KeyResponse,
        SeedUpdate,
    },
    rng::seed::{KeyIdSeed, KeySeed},
};

use log::debug;
use sam_common::AccountId;
use sam_server::managers::traits::account_manager::AccountManager;

use crate::{
    error::{DenimRouterError, LogicError},
    logic::keys::{get_keys_for, update_seed},
    managers::{default::ClientRequest, error::DenimKeyManagerError, traits::KeyRequestManager},
    state::{DenimState, DenimStateType},
};

pub async fn denim_router<T: DenimStateType>(
    state: &mut DenimState<T>,
    request: ClientRequest,
    account_id: AccountId,
) -> Result<(), DenimRouterError> {
    match request {
        ClientRequest::BlockRequest(_, block_request) => {
            handle_block_request(state, block_request, account_id).await
        }
        ClientRequest::KeyRequest(msg_id, key_request) => {
            handle_key_request(state, msg_id, key_request, account_id).await
        }
        ClientRequest::SeedUpdateRequest(msg_id, seed_update) => {
            handle_seed_update(state, msg_id, seed_update, account_id).await
        }
    }
}

pub async fn handle_block_request<T: DenimStateType>(
    state: &mut DenimState<T>,
    request: BlockRequest,
    sender_account_id: AccountId,
) -> Result<(), DenimRouterError> {
    let blocked_account_id = AccountId::try_from(request.account_id)
        .map_err(|_| DenimRouterError::KeyRequestMalformed)?;
    state
        .buffer_manager
        .block_user(sender_account_id, blocked_account_id)
        .await;
    Ok(())
}

pub async fn handle_key_request<T: DenimStateType>(
    state: &mut DenimState<T>,
    msg_id: u32,
    request: KeyRequest,
    sender_account_id: AccountId,
) -> Result<(), DenimRouterError> {
    let requested_account_id = AccountId::try_from(request.account_id)
        .map_err(|_| DenimRouterError::KeyRequestMalformed)?;

    let requested_device_id = request
        .specific_device_ids
        .first()
        .ok_or(DenimRouterError::NoDeviceIdInRequest)?;

    let key_bundle = match get_keys_for(
        state,
        requested_account_id,
        requested_device_id.to_owned().into(),
    )
    .await
    {
        Ok(key_bundle) => key_bundle,
        Err(LogicError::KeyManager(DenimKeyManagerError::NoSeed)) => {
            debug!("{requested_account_id}.{requested_device_id} has not uploaded a key seed yet. Request will be defered.");
            state
                .key_request_manager
                .store_receiver(requested_account_id, sender_account_id)
                .await;
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

    let key_response = MessageKind::KeyResponse(
        KeyResponse::builder()
            .account_id(requested_account_id.into())
            .identity_key(identity_key.serialize().to_vec())
            .key_bundle(key_bundle)
            .build(),
    );

    enqueue_message(state, msg_id, key_response, sender_account_id).await?;

    Ok(())
}

pub async fn handle_seed_update<T: DenimStateType>(
    state: &mut DenimState<T>,
    msg_id: u32,
    request: SeedUpdate,
    sender_account_id: AccountId,
) -> Result<(), DenimRouterError> {
    let key_seed = KeySeed::try_from(request.pre_key_seed)?;
    let key_id_seed = KeyIdSeed::try_from(request.pre_key_id_seed)?;

    update_seed(state, sender_account_id, 1.into(), key_seed, key_id_seed).await?;

    if let Some(receivers) = state
        .key_request_manager
        .get_receivers(sender_account_id)
        .await
    {
        for receiver in receivers {
            let key_bundle = get_keys_for(state, sender_account_id, 1.into()).await?;
            let identity_key = state
                .accounts
                .get_account(sender_account_id)
                .await?
                .identity()
                .to_owned();

            let key_response = MessageKind::KeyResponse(
                KeyResponse::builder()
                    .account_id(sender_account_id.into())
                    .identity_key(identity_key.serialize().to_vec())
                    .key_bundle(key_bundle)
                    .build(),
            );

            enqueue_message(state, msg_id, key_response, receiver).await?;
        }
    }

    Ok(())
}

pub async fn enqueue_message<T: DenimStateType>(
    state: &mut DenimState<T>,
    msg_id: u32,
    message: MessageKind,
    receiver: AccountId,
) -> Result<(), DenimRouterError> {
    debug!("Enqueuing {:?} for {receiver}", message);
    state
        .buffer_manager
        .enqueue_message(
            receiver,
            DeniableMessage::builder()
                .message_kind(message)
                .message_id(msg_id)
                .build(),
        )
        .await?;
    Ok(())
}
