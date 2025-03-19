use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
};

use log::info;

use crate::{
    error::ServerError,
    proxy::{connect_to_sam_server, init_proxy_service},
    state::DenimState,
};

pub async fn websocket_endpoint(
    State(state): State<DenimState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ServerError> {
    let (client, queue) = connect_to_sam_server(headers, &state).await?;

    Ok(ws.on_upgrade(move |socket| async move {
        info!("A User Connected");
        init_proxy_service(socket, client, queue).await
    }))
}
