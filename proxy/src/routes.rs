use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
};

use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};
use log::info;

use sam_server::auth::get_credentials;

use crate::{
    error::ServerError,
    managers::traits::MessageIdProvider,
    proxy::{connect_to_sam_server, init_proxy_service},
    state::DenimState,
};

pub async fn websocket_endpoint<
    T: ReceivingBufferConfig,
    U: SendingBufferConfig,
    V: MessageIdProvider,
>(
    State(state): State<DenimState<T, U, V>>,
    headers: HeaderMap,
    TypedHeader(Authorization(basic)): TypedHeader<Authorization<Basic>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ServerError> {
    let (account_id, device_id) =
        get_credentials(basic.username().to_string()).map_err(|_| ServerError::SAMUnAuth)?;
    let (client, queue) = connect_to_sam_server(headers, &state).await?;
    Ok(ws.on_upgrade(move |socket| async move {
        info!("A User Connected");
        init_proxy_service(state, socket, client, queue, account_id, device_id).await
    }))
}
