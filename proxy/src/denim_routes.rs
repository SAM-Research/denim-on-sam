use denim_sam_common::{
    denim_message::{
        deniable_message::MessageKind, BlockRequest, DeniableMessage, KeyRequest, KeyResponse,
        MessageType, SeedUpdate, UserMessage,
    },
    rng::{
        seed::{KeyIdSeed, KeySeed},
        RngState,
    },
};

use libsignal_protocol::CiphertextMessage;
use log::{debug, error};
use sam_common::{address::DEFAULT_DEVICE_ID, AccountId};
use sam_server::managers::traits::account_manager::AccountManager;

use crate::managers::DenimEcPreKeyManager;
use crate::{
    error::{DenimRouterError, LogicError},
    logic::keys::{get_keys_for, remove_pending_key, store_pending_key, update_seed},
    managers::{
        default::ClientRequest,
        error::DenimKeyManagerError,
        traits::{BlockList, KeyRequestManager, MessageIdProvider},
    },
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
        ClientRequest::UserMessage(_, message) => {
            handle_user_message(state, message, account_id).await
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
        .block_list
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
                .store_requester(requested_account_id, sender_account_id)
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

    // defered key requests can now be processed
    if let Some(requesters) = state
        .key_request_manager
        .remove_requesters(sender_account_id)
        .await
    {
        for requester in requesters {
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

            enqueue_message(state, msg_id, key_response, requester).await?;
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

pub async fn handle_user_message<T: DenimStateType>(
    state: &mut DenimState<T>,
    mut message: UserMessage,
    sender_account_id: AccountId,
) -> Result<(), DenimRouterError> {
    let id = state
        .message_id_provider
        .get_message_id(sender_account_id)
        .await;
    let receiver_id =
        AccountId::try_from(message.account_id).map_err(|_| DenimRouterError::InvalidAccountId)?;
    if state
        .block_list
        .is_user_blocked(&receiver_id, &sender_account_id)
        .await
    {
        return Ok(());
    }

    // change message account id to sender
    message.account_id = sender_account_id.into();

    match message.message_type() {
        MessageType::SignalMessage => {
            remove_pending_key(state, sender_account_id, receiver_id).await?;
        }
        // first convert to ciphertext when we actually need it
        MessageType::PreKeySignalMessage => match message.ciphertext() {
            Ok(CiphertextMessage::PreKeySignalMessage(pre)) => {
                store_pending_key(state, &pre, sender_account_id, receiver_id).await?;
                message.rng_counter = state
                    .keys
                    .pre_keys
                    .get_key_seed_for(
                        message
                            .account_id
                            .clone()
                            .try_into()
                            .map_err(|_| DenimRouterError::InvalidAccountId)?,
                        DEFAULT_DEVICE_ID.into(),
                    )
                    .await?
                    .offset()
                    .try_into()
                    .ok()
            }
            Ok(_) => Err(DenimRouterError::MalformedUserMessage)?,
            Err(e) => {
                error!("Failed to decode CiphertextMessage '{e}' from '{sender_account_id}'")
            }
        },
        _ => (),
    };

    Ok(state
        .buffer_manager
        .enqueue_message(
            receiver_id,
            DeniableMessage {
                message_id: id,
                message_kind: Some(MessageKind::DeniableMessage(message)),
            },
        )
        .await?)
}

#[cfg(test)]
mod test {
    use denim_sam_common::{
        denim_message::{MessageType, UserMessage},
        rng::seed::{KeyIdSeed, KeySeed},
    };
    use libsignal_protocol::{
        CiphertextMessage, IdentityKeyPair, PreKeySignalMessage, SignalMessage,
    };
    use rand::rngs::OsRng;
    use sam_common::{address::DEFAULT_DEVICE_ID, AccountId};
    use sam_server::managers::traits::key_manager::SignedPreKeyManager;
    use sam_test_utils::server_utils::signed_ec_pre_key;

    use crate::{
        denim_routes::denim_router,
        logic::keys::update_seed,
        managers::{default::ClientRequest, DenimEcPreKeyManager},
        state::{DenimState, InMemoryDenimStateType},
    };

    #[tokio::test]
    async fn deletes_keys_when_reply_on_pre_key_message() {
        let mut state =
            DenimState::<InMemoryDenimStateType>::in_memory_test("127.0.0.1:8080".to_string());
        let id_pair = IdentityKeyPair::generate(&mut OsRng);
        let alice = AccountId::generate();
        let bob = AccountId::generate();
        let id = DEFAULT_DEVICE_ID.into();

        let seed = KeySeed::random(&mut OsRng);
        let id_seed = KeyIdSeed::random(&mut OsRng);

        let signed_id = 23u32;
        let signed = signed_ec_pre_key(signed_id, &id_pair, OsRng);

        state
            .keys
            .pre_keys
            .store_key_seed_for(bob, DEFAULT_DEVICE_ID.into(), seed.clone().into())
            .await
            .expect("Can store seed for bob");

        state
            .keys
            .pre_keys
            .store_key_id_seed_for(bob, DEFAULT_DEVICE_ID.into(), id_seed.clone().into())
            .await
            .expect("Can store id seed for bob");

        state
            .keys
            .pre_keys
            .store_key_seed_for(alice, DEFAULT_DEVICE_ID.into(), seed.clone().into())
            .await
            .expect("Can store seed for bob");

        state
            .keys
            .pre_keys
            .store_key_id_seed_for(alice, DEFAULT_DEVICE_ID.into(), id_seed.clone().into())
            .await
            .expect("Can store id seed for bob");

        state
            .keys
            .signed_pre_keys
            .set_signed_pre_key(bob, id, id_pair.identity_key(), signed)
            .await
            .expect("can set signed pre key");
        update_seed(&mut state, bob, id, seed, id_seed)
            .await
            .expect("can update seed");

        let ec_key = state
            .keys
            .pre_keys
            .get_ec_pre_key(bob, id)
            .await
            .expect("can get ec pre key");

        let sig_msg = SignalMessage::new(
            3u8,
            &[1; 32],
            *id_pair.public_key(),
            1u32,
            0u32,
            &[1, 2, 3],
            id_pair.identity_key(),
            id_pair.identity_key(),
        )
        .expect("can create signal message");
        let pre_msg = PreKeySignalMessage::new(
            3u8,
            1u32,
            Some(ec_key.key_id.into()),
            signed_id.into(),
            None,
            *id_pair.public_key(),
            *id_pair.identity_key(),
            sig_msg.clone(),
        )
        .expect("can create prekey message");

        let pre_cipher = CiphertextMessage::PreKeySignalMessage(pre_msg);
        let cipher = CiphertextMessage::SignalMessage(sig_msg);

        denim_router(
            &mut state,
            ClientRequest::UserMessage(
                1u32,
                UserMessage::builder()
                    .account_id(bob.into())
                    .content(pre_cipher.serialize().into())
                    .message_type(MessageType::PreKeySignalMessage.into())
                    .build(),
            ),
            alice,
        )
        .await
        .expect("Can route prekey message");

        assert!(
            state
                .keys
                .pre_keys
                .has_pending_key(alice, bob, DEFAULT_DEVICE_ID.into())
                .await
        );

        denim_router(
            &mut state,
            ClientRequest::UserMessage(
                1u32,
                UserMessage::builder()
                    .account_id(alice.into())
                    .content(cipher.serialize().into())
                    .message_type(MessageType::SignalMessage.into())
                    .build(),
            ),
            bob,
        )
        .await
        .expect("can route signal message");
        assert!(
            !state
                .keys
                .pre_keys
                .has_pending_key(alice, bob, DEFAULT_DEVICE_ID.into())
                .await
        );
    }
}
