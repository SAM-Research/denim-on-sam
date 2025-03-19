use std::net::SocketAddr;

use axum::{routing::get, Router};
use log::info;

use crate::{routes::websocket_endpoint, state::DenimState};

pub struct DenimConfig {
    pub state: DenimState,
    pub addr: SocketAddr,
}

pub async fn start_proxy(config: DenimConfig) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/api/v1/websocket", get(websocket_endpoint))
        .with_state(config.state);

    info!("Starting DenIM Proxy on http://{}...", config.addr);
    Ok(axum_server::bind(config.addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?)
}
