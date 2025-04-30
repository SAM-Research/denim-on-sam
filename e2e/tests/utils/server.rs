use crate::utils::tls::tls_configs;
use async_trait::async_trait;
use denim_sam_proxy::{
    config::TlsConfig,
    server::{start_proxy, DenimConfig},
    state::{DenimStateType, InMemoryDenimStateType, PostgresDenimStateType},
};
use rstest::fixture;
use sam_server::{
    config::TlsConfig as SamTlsConfig,
    managers::{in_memory::InMemStateType, postgres::PostgresStateType},
    start_server, ServerConfig, StateType,
};
use sam_test_utils::e2e::{in_memory_server_state, postgres_server_state};
use sam_test_utils::get_next_port;
use tokio::{
    sync::oneshot::{self, Receiver},
    task::JoinHandle,
};

#[fixture]
pub fn next_sam_port() -> u16 {
    get_next_port()
}

#[fixture]
pub fn next_denim_port() -> u16 {
    get_next_port()
}

pub struct TestServerConfigs<S: StateType, D: DenimStateType> {
    pub sam: SamTestServerConfig<S>,
    pub denim: DenimTestServerConfig<D>,
}

#[fixture]
pub async fn in_memory_configs(
    next_sam_port: u16,
    next_denim_port: u16,
    tls_configs: Option<(SamTlsConfig, TlsConfig)>,
) -> TestServerConfigs<InMemStateType, InMemoryDenimStateType> {
    let sam_addr = format!("127.0.0.1:{next_sam_port}");
    let proxy_addr = format!("127.0.0.1:{next_denim_port}");

    let (sam_tls, proxy_tls) = match tls_configs {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };

    let sam = ServerConfig {
        state: in_memory_server_state().await,
        addr: sam_addr.parse().expect("Unable to parse socket address"),
        tls_config: sam_tls.map(|tls| tls.try_into().expect("can create tls config for SAM")),
    }
    .into();

    let (maybe_tls_config, maybe_ws_proxy_tls_config) = match proxy_tls {
        Some(tls) => {
            let (tls_config, ws_proxy_tls_config) = tls.create().expect("Can create tls config");
            (Some(tls_config), Some(ws_proxy_tls_config))
        }
        None => (None, None),
    };

    let denim = DenimConfig::<InMemoryDenimStateType>::in_memory()
        .addr(proxy_addr.parse().expect("Unable to parse socket address"))
        .sam_address(sam_addr.to_string())
        .maybe_tls_config(maybe_tls_config)
        .maybe_ws_proxy_tls_config(maybe_ws_proxy_tls_config)
        .call()
        .into();

    TestServerConfigs { sam, denim }
}

#[fixture]
pub fn connection_str() -> String {
    "postgres://test:test@127.0.0.1:5432/sam_test_db".to_string()
}

#[fixture]
pub async fn postgres_configs(
    next_sam_port: u16,
    next_denim_port: u16,
    tls_configs: Option<(SamTlsConfig, TlsConfig)>,
    connection_str: String,
) -> TestServerConfigs<PostgresStateType, PostgresDenimStateType> {
    let sam_addr = format!("127.0.0.1:{next_sam_port}");
    let proxy_addr = format!("127.0.0.1:{next_denim_port}");

    let (sam_tls, proxy_tls) = match tls_configs {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };

    let sam = ServerConfig {
        state: postgres_server_state().await,
        addr: sam_addr.parse().expect("Unable to parse socket address"),
        tls_config: sam_tls.map(|tls| tls.try_into().expect("can create tls config for SAM")),
    }
    .into();

    let (maybe_tls_config, maybe_ws_proxy_tls_config) = match proxy_tls {
        Some(tls) => {
            let (tls_config, ws_proxy_tls_config) = tls.create().expect("Can create tls config");
            (Some(tls_config), Some(ws_proxy_tls_config))
        }
        None => (None, None),
    };

    let denim = DenimConfig::postgres()
        .db_url(connection_str)
        .addr(proxy_addr.parse().expect("Unable to parse socket address"))
        .sam_address(sam_addr.to_string())
        .maybe_tls_config(maybe_tls_config)
        .maybe_ws_proxy_tls_config(maybe_ws_proxy_tls_config)
        .call()
        .await
        .expect("can create a postgres denim config")
        .into();

    TestServerConfigs { sam, denim }
}

pub struct TestServer {
    thread: JoinHandle<Result<(), std::io::Error>>,
    started_rx: Receiver<()>,
    address: String,
}

impl TestServer {
    pub fn join_handle(&mut self) -> &mut JoinHandle<Result<(), std::io::Error>> {
        &mut self.thread
    }
    pub fn started_rx(&mut self) -> &mut Receiver<()> {
        &mut self.started_rx
    }
    pub fn address(&self) -> &String {
        &self.address
    }
}

pub struct SamTestServerConfig<T: StateType> {
    server_config: ServerConfig<T>,
}

impl<T: StateType> From<ServerConfig<T>> for SamTestServerConfig<T> {
    fn from(server_config: ServerConfig<T>) -> Self {
        Self { server_config }
    }
}

#[async_trait]
pub trait TestServerConfig {
    async fn start(self) -> TestServer;
}

#[async_trait]
impl<T: StateType> TestServerConfig for SamTestServerConfig<T> {
    async fn start(self) -> TestServer {
        let address = self.server_config.addr.to_string();
        let (tx, started_rx) = oneshot::channel::<()>();
        let thread = tokio::spawn(async move {
            let server = start_server(self.server_config);
            tx.send(())
                .expect("should be able to inform other thread that server is started");
            server.await
        });
        TestServer {
            thread,
            started_rx,
            address,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.join_handle().abort();
    }
}

pub struct DenimTestServerConfig<T: DenimStateType> {
    server_config: DenimConfig<T>,
}

impl<T: DenimStateType> From<DenimConfig<T>> for DenimTestServerConfig<T> {
    fn from(server_config: DenimConfig<T>) -> Self {
        Self { server_config }
    }
}

#[async_trait]
impl<T: DenimStateType> TestServerConfig for DenimTestServerConfig<T> {
    async fn start(self) -> TestServer {
        let address = self.server_config.addr.to_string();
        let (tx, started_rx) = oneshot::channel::<()>();
        let thread = tokio::spawn(async move {
            let server = start_proxy(self.server_config);
            tx.send(())
                .expect("should be able to inform other thread that server is started");
            server.await
        });
        TestServer {
            thread,
            started_rx,
            address,
        }
    }
}
