use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};
use log::info;

use crate::{managers::traits::MessageIdProvider, routes::websocket_endpoint, state::DenimState};

pub struct DenimConfig<T: ReceivingBufferConfig, U: SendingBufferConfig, V: MessageIdProvider> {
    pub state: DenimState<T, U, V>,
    pub addr: SocketAddr,
    pub tls_config: Option<rustls::ServerConfig>,
}

pub async fn start_proxy<T: ReceivingBufferConfig, U: SendingBufferConfig, V: MessageIdProvider>(
    config: DenimConfig<T, U, V>,
) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/hello", get(|| async { "Hello From DenIM SAM Proxy" }))
        .route("/api/v1/websocket", get(websocket_endpoint))
        .with_state(config.state);

    info!(
        "Starting Denim Proxy on ws{}://{}",
        if config.tls_config.is_some() { "s" } else { "" },
        config.addr
    );
    if let Some(tls_config) = config.tls_config {
        let axum_tls_config = RustlsConfig::from_config(Arc::new(tls_config));
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
