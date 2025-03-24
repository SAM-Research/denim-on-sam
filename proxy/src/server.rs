use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use log::info;

use crate::{routes::websocket_endpoint, state::DenimState};

pub struct DenimConfig {
    pub state: DenimState,
    pub addr: SocketAddr,
    pub tls_config: Option<Arc<rustls::ServerConfig>>,
}

pub async fn start_proxy(config: DenimConfig) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/api/v1/websocket", get(websocket_endpoint))
        .with_state(config.state);

    info!(
        "Starting Denim Proxy on http{}://{}...",
        if config.tls_config.is_some() { "s" } else { "" },
        config.addr
    );
    if let Some(tls_config) = config.tls_config {
        let axum_tls_config = RustlsConfig::from_config(tls_config);
        axum_server::bind_rustls(config.addr, axum_tls_config)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await?;
    } else {
        axum_server::bind(config.addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await?;
    };

    Ok(())
}
