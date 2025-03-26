use axum::http::HeaderMap;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{error, info};
use sam_client::net::protocol::websocket::{WebSocket, WebSocketClient, WebSocketReceiver};
use sam_common::{AccountId, DeviceId};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    error::ServerError,
    state::{DenimState, StateType},
    utils::{
        into_axum_message, into_tungstenite_message, websocket_config, AxumMessage, AxumWebSocket,
    },
};

// TODO: placeholder until we implement protobuf
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

    let queue = client
        .connect(ProxyWebSocketReceiver {})
        .await
        .map_err(|_| ServerError::SAMUnAuth)?;
    Ok((client, queue))
}

pub async fn init_proxy_service(
    socket: AxumWebSocket,
    server_client: WebSocketClient,
    server_receiver: Receiver<ProxyMessage>,
    _account_id: AccountId,
    _device_id: DeviceId,
) {
    let (sender, receiver) = socket.split();

    tokio::spawn(sam_server_handler(server_receiver, sender));
    tokio::spawn(denim_client_receiver(server_client, receiver));
}

/// Handles messages from SAM Server and send them to client
/// This is here we should put piggy back denim messages to the client
async fn sam_server_handler(
    mut server_receiver: Receiver<ProxyMessage>,
    mut client_sender: SplitSink<AxumWebSocket, AxumMessage>,
) {
    // SAM Server sends proxy a message
    while let Some(msg) = server_receiver.recv().await {
        if client_sender.send(msg).await.is_err() {
            break; // disconnected
        }
    }
}

/// Handles messages from Denim Client and forward them to SAM Server
/// This is here we should extract SAM Message and send it
/// We should also build chunks to Denim Messages here
async fn denim_client_receiver(
    mut server_client: WebSocketClient,
    mut client_receiver: SplitStream<AxumWebSocket>,
) {
    // Client sends proxy a message
    while let Some(Ok(msg)) = client_receiver.next().await {
        let res = match into_tungstenite_message(msg) {
            Some(msg) => server_client.send(msg).await,
            None => {
                error!("Failed to convert axum message to tungstenite message");
                info!("Disconnecting...");

                break;
            }
        };
        match res {
            Ok(_) => continue,
            Err(e) => {
                error!("WebSocketError: {e}");
                info!("Disconnecting...");
                break;
            }
        }
    }
}

struct ProxyWebSocketReceiver {}

#[async_trait::async_trait]
impl WebSocketReceiver<ProxyMessage> for ProxyWebSocketReceiver {
    async fn handler(
        &mut self,
        mut receiver: SplitStream<WebSocket>,
        enqueue: Sender<ProxyMessage>,
    ) {
        while let Some(Ok(msg)) = receiver.next().await {
            let res = match into_axum_message(msg) {
                Some(msg) => enqueue.send(msg).await,
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
