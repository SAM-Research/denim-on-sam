use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use bon::bon;
use denim_sam_common::buffers::in_mem::{
    InMemoryReceivingBufferConfig, InMemorySendingBufferConfig,
};
use log::info;
use rustls::{ClientConfig, ServerConfig};
use sam_server::managers::in_memory::account::InMemoryAccountManager;
use sam_server::managers::in_memory::device::InMemoryDeviceManager;
use sam_server::managers::in_memory::keys::InMemorySignedPreKeyManager;

use crate::managers::in_mem::InMemoryDenimEcPreKeyManager;
use crate::managers::{BufferManager, DenimKeyManager, InMemoryMessageIdProvider};
use crate::routes::websocket_endpoint;
use crate::state::{DenimState, InMemoryBufferManagerType, InMemoryStateType, StateType};

pub struct DenimConfig<T: StateType> {
    pub state: DenimState<T>,
    pub addr: SocketAddr,
    pub tls_config: Option<rustls::ServerConfig>,
}

#[bon]
impl DenimConfig<InMemoryStateType> {
    #[builder]
    pub fn in_memory(
        addr: SocketAddr,
        sam_address: String,
        tls_config: Option<ServerConfig>,
        ws_proxy_tls_config: Option<ClientConfig>,
        #[builder(default = 10)] channel_buffer_size: usize,
        #[builder(default = 10)] key_generate_amount: usize,
        #[builder(default = 1.0)] deniable_ratio: f32,
    ) -> Self {
        let rcfg = InMemoryReceivingBufferConfig;
        let scfg = InMemorySendingBufferConfig::builder()
            .q(deniable_ratio)
            .build();
        let id_provider = InMemoryMessageIdProvider::default();
        let buffer_mgr: BufferManager<InMemoryBufferManagerType> =
            BufferManager::new(rcfg, scfg, id_provider);

        Self {
            addr,
            tls_config,
            state: DenimState::<InMemoryStateType>::new(
                sam_address,
                channel_buffer_size,
                ws_proxy_tls_config,
                buffer_mgr,
                DenimKeyManager::new(
                    InMemoryDenimEcPreKeyManager::new(key_generate_amount),
                    InMemorySignedPreKeyManager::default(),
                ),
                InMemoryAccountManager::default(),
                // TODO: When adding postgres manager, connect for device manager should not take these
                // params as they are already set by SAM.
                InMemoryDeviceManager::new("Test".to_owned(), 120),
            ),
        }
    }
}

pub async fn start_proxy<T: StateType>(config: DenimConfig<T>) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/api/v1/websocket", get(websocket_endpoint))
        .with_state(config.state);

    info!(
        "Starting Denim Proxy on http{}://{}...",
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
