use std::{net::SocketAddr, sync::Arc};

use axum::extract::Request;
use axum::middleware::{from_fn, Next};
use axum::response::IntoResponse;
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
use sam_server::managers::postgres::keys::PostgresSignedPreKeyManager;
use sam_server::managers::postgres::{
    PostgresAccountManager, PostgresConnector, PostgresDeviceManager,
};

use sqlx::Error;

use crate::managers::in_mem::{
    InMemoryBlockList, InMemoryDenimEcPreKeyManager, InMemoryKeyRequestManager,
};

use crate::managers::{BufferManager, DenimKeyManager, InMemoryMessageIdProvider};
use crate::routes::websocket_endpoint;
use crate::state::{
    DenimState, InMemoryBufferManagerType, InMemoryStateType, PostgresStateType, StateType,
};

pub struct DenimConfig<T: StateType> {
    pub state: DenimState<T>,
    pub addr: SocketAddr,
    pub tls_config: Option<rustls::ServerConfig>,
}

#[bon]
impl DenimConfig<PostgresStateType> {
    #[builder]
    pub async fn postgres(
        url: String,
        addr: SocketAddr,
        sam_address: String,
        tls_config: Option<ServerConfig>,
        ws_proxy_tls_config: Option<ClientConfig>,
        #[builder(default = 10)] channel_buffer_size: usize,
        #[builder(default = 10)] key_generate_amount: usize,
        #[builder(default = 1.0)] deniable_ratio: f32,
    ) -> Result<Self, Error> {
        let conn = PostgresConnector::connect(&url).await?;
        let rcfg = InMemoryReceivingBufferConfig;
        let scfg = InMemorySendingBufferConfig::default();
        let id_provider = InMemoryMessageIdProvider::default();
        let buffer_mgr: BufferManager<InMemoryBufferManagerType> = BufferManager::new(
            InMemoryBlockList::default(),
            rcfg,
            scfg,
            id_provider,
            deniable_ratio,
        );

        Ok(Self {
            addr,
            tls_config,
            state: DenimState::<PostgresStateType>::builder()
                .sam_addr(sam_address)
                .channel_buffer_size(channel_buffer_size)
                .maybe_ws_proxy_tls_config(ws_proxy_tls_config)
                .buffer_manager(buffer_mgr)
                .keys(DenimKeyManager::new(
                    InMemoryDenimEcPreKeyManager::new(key_generate_amount),
                    PostgresSignedPreKeyManager::new(conn.pool()),
                ))
                .accounts(PostgresAccountManager::new(conn.pool()))
                .devices(PostgresDeviceManager::new(conn.pool()))
                .key_request_manager(InMemoryKeyRequestManager::default())
                .build(),
        })
    }
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
        let scfg = InMemorySendingBufferConfig::default();
        let id_provider = InMemoryMessageIdProvider::default();
        let buffer_mgr: BufferManager<InMemoryBufferManagerType> = BufferManager::new(
            InMemoryBlockList::default(),
            rcfg,
            scfg,
            id_provider,
            deniable_ratio,
        );

        Self {
            addr,
            tls_config,
            state: DenimState::<InMemoryStateType>::builder()
                .sam_addr(sam_address)
                .channel_buffer_size(channel_buffer_size)
                .maybe_ws_proxy_tls_config(ws_proxy_tls_config)
                .buffer_manager(buffer_mgr)
                .keys(DenimKeyManager::new(
                    InMemoryDenimEcPreKeyManager::new(key_generate_amount),
                    InMemorySignedPreKeyManager::default(),
                ))
                .accounts(InMemoryAccountManager::default()) // TODO: When adding postgres manager, connect for device manager should not take these
                .devices(InMemoryDeviceManager::new("Test".to_owned(), 120)) // params as they are already set by SAM.
                .key_request_manager(InMemoryKeyRequestManager::default())
                .build(),
        }
    }
}

async fn log_request(req: Request, next: Next) -> impl IntoResponse {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    info!("{} '{}'", method, path);
    next.run(req).await
}

pub async fn start_proxy<T: StateType>(config: DenimConfig<T>) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/v1/websocket", get(websocket_endpoint))
        .layer(from_fn(log_request))
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
