use axum::http::HeaderMap;
use denim_sam_common::buffers::DenimMessage;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{error, info};
use prost::bytes::Bytes;
use sam_common::{AccountId, DeviceId};
use sam_net::websocket::{WebSocket, WebSocketClient, WebSocketReceiver};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    config::websocket_config,
    denim_routes::denim_router,
    error::ServerError,
    state::{DenimState, StateType},
    utils::TungsteniteMessage,
    utils::{into_axum_message, AxumMessage, AxumWebSocket},
};

type ProxyMessage = AxumMessage;

/// Try and establish connection to sam server using clients credentials
pub async fn connect_to_sam_server<T: StateType>(
    headers: HeaderMap,
    state: &DenimState<T>,
) -> Result<(WebSocketClient, Receiver<ProxyMessage>), ServerError> {
    let basic = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(ServerError::SAMUnAuth)?
        .to_string();

    let mut client: WebSocketClient = websocket_config(basic, state)?.into();

    let (tx, rx) = channel(state.channel_buffer_size());
    client
        .connect(ProxyWebSocketReceiver { enqueue: tx })
        .await
        .map_err(|_| ServerError::SAMUnAuth)?;
    Ok((client, rx))
}

pub async fn init_proxy_service<T: StateType>(
    state: DenimState<T>,
    socket: AxumWebSocket,
    server_client: WebSocketClient,
    server_receiver: Receiver<ProxyMessage>,
    account_id: AccountId,
    _device_id: DeviceId,
) {
    let (sender, receiver) = socket.split();

    tokio::spawn(sam_server_handler(
        state.clone(),
        server_receiver,
        sender,
        account_id,
    ));
    tokio::spawn(denim_client_receiver(
        state,
        server_client,
        receiver,
        account_id,
    ));
}

/// Handles messages from SAM Server and send them to client
/// This is here we should put piggy back denim messages to the client
async fn sam_server_handler<T: StateType>(
    mut state: DenimState<T>,
    mut server_receiver: Receiver<ProxyMessage>,
    mut client_sender: SplitSink<AxumWebSocket, AxumMessage>,
    account_id: AccountId,
) {
    // SAM Server sends proxy a message
    while let Some(msg) = server_receiver.recv().await {
        let msg = match msg {
            AxumMessage::Binary(msg) => msg,
            AxumMessage::Close(_) => break,
            _ => continue,
        };
        let len = match msg.len().try_into() {
            Ok(len) => len,
            Err(_) => {
                error!("SAM Message too big for Denim!");
                info!("Disconnecting...");
                break;
            }
        };
        let payload = match state
            .buffer_manager
            .get_deniable_payload(account_id, len)
            .await
        {
            Ok(payload) => payload,
            Err(e) => {
                error!("get_deniable_payload failed '{e}'");
                info!("Disconnecting...");
                break;
            }
        };

        let msg = DenimMessage::builder()
            .regular_payload(msg.to_vec())
            .deniable_payload(payload)
            .q(state.buffer_manager.get_q().await)
            .build();

        let encoded_msg = match msg.encode() {
            Ok(encoded_msg) => encoded_msg,
            Err(e) => {
                error!("Convertion of Payload Failed '{e}'");
                info!("Disconnecting...");
                break;
            }
        };

        if client_sender
            .send(AxumMessage::Binary(encoded_msg.into()))
            .await
            .is_err()
        {
            break; // disconnected
        }
    }
}

/// Handles messages from Denim Client and forward them to SAM Server
/// This is here we should extract SAM Message and send it
/// We should also build chunks to Denim Messages here
async fn denim_client_receiver<T: StateType>(
    mut state: DenimState<T>,
    mut server_client: WebSocketClient,
    mut client_receiver: SplitStream<AxumWebSocket>,
    account_id: AccountId,
) {
    // Client sends proxy a message
    while let Some(Ok(msg)) = client_receiver.next().await {
        let msg = match msg {
            AxumMessage::Binary(msg) => msg,
            AxumMessage::Close(_) => break,
            _ => continue,
        };
        let msg = match DenimMessage::decode(msg.to_vec()) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to decode denim message '{e}'");
                break;
            }
        };

        if let Err(e) = server_client
            .send(TungsteniteMessage::Binary(Bytes::from(msg.regular_payload)))
            .await
        {
            error!("WebSocketError: {e}");
            info!("Disconnecting...");
            break;
        }

        let chunks = msg.deniable_payload.denim_chunks().to_owned();

        //TODO: this should not happen if a user is blocked.
        match state
            .buffer_manager
            .enqueue_chunks(account_id, chunks)
            .await
        {
            Ok(results) => {
                for res in results {
                    let response = match res {
                        Ok(Some(request)) => denim_router(state.clone(), request, account_id).await,
                        Ok(None) => continue,
                        Err(e) => {
                            error!("failed to process deniable message: '{e}'");
                            continue;
                        }
                    };

                    let enqueue_res = match response {
                        Ok(msg) => state.buffer_manager.enqueue_message(account_id, msg).await,
                        Err(e) => {
                            error!("Denim routing failed '{e}'");
                            continue;
                        }
                    };
                    if let Err(e) = enqueue_res {
                        error!("enqueue_message failed '{e}'");
                        continue;
                    }
                }
                continue;
            }
            Err(e) => {
                error!("enqueue_chunks failed '{e}' for account '{account_id}'");
                continue;
            }
        };
    }
}

struct ProxyWebSocketReceiver {
    enqueue: Sender<ProxyMessage>,
}

#[async_trait::async_trait]
impl WebSocketReceiver for ProxyWebSocketReceiver {
    async fn handler(&mut self, mut receiver: SplitStream<WebSocket>) {
        while let Some(Ok(msg)) = receiver.next().await {
            let res = match into_axum_message(msg) {
                Some(msg) => self.enqueue.send(msg).await,
                None => {
                    error!("Failed to convert tungstenite message into axum message");
                    continue;
                }
            };
            if res.is_err() {
                error!("Failed to send")
            }
        }
    }
}
