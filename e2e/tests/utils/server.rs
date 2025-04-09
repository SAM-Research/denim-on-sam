use denim_sam_proxy::{
    config::TlsConfig,
    server::{start_proxy, DenimConfig},
    state::DenimState,
};
use sam_server::{
    managers::{
        in_memory::{
            account::InMemoryAccountManager,
            device::InMemoryDeviceManager,
            keys::{
                InMemoryEcPreKeyManager, InMemoryLastResortPqPreKeyManager,
                InMemoryPqPreKeyManager, InMemorySignedPreKeyManager,
            },
            message::InMemoryMessageManager,
            InMemStateType,
        },
        KeyManager,
    },
    start_server, ServerConfig, ServerState,
};

use tokio::{
    sync::oneshot::{self, Receiver},
    task::JoinHandle,
};

pub struct TestSamServer {
    thread: JoinHandle<Result<(), std::io::Error>>,
    started_rx: Receiver<()>,
}

impl Drop for TestSamServer {
    fn drop(&mut self) {
        self.thread.abort();
    }
}

impl TestSamServer {
    pub async fn start(address: &str, tls_config: Option<rustls::ServerConfig>) -> Self {
        let config = ServerConfig {
            state: in_memory_server_state(),
            addr: address.parse().expect("Unable to parse socket address"),
            tls_config,
        };
        let (tx, started_rx) = oneshot::channel::<()>();
        let thread = tokio::spawn(async move {
            let server = start_server(config);
            tx.send(())
                .expect("should be able to inform other thread that server is started");
            server.await
        });
        Self { thread, started_rx }
    }

    pub fn started_rx(&mut self) -> &mut Receiver<()> {
        &mut self.started_rx
    }
}

pub fn in_memory_server_state() -> ServerState<InMemStateType> {
    ServerState::new(
        InMemoryAccountManager::default(),
        InMemoryDeviceManager::new("test".to_string(), 600),
        InMemoryMessageManager::default(),
        KeyManager::new(
            InMemoryEcPreKeyManager::default(),
            InMemoryPqPreKeyManager::default(),
            InMemorySignedPreKeyManager::default(),
            InMemoryLastResortPqPreKeyManager::default(),
        ),
    )
}

pub struct TestDenimProxy {
    thread: JoinHandle<Result<(), std::io::Error>>,
    started_rx: Receiver<()>,
}

impl Drop for TestDenimProxy {
    fn drop(&mut self) {
        self.thread.abort();
    }
}

#[allow(unused)]
impl TestDenimProxy {
    pub async fn start(sam_addr: &str, proxy_addr: &str, config: Option<TlsConfig>) -> Self {
        let config = if let Some(tls) = config {
            let (server, client) = tls.create().expect("Can create tls config");
            DenimConfig {
                state: DenimState::new(sam_addr.to_string(), 10, Some(client)),
                addr: proxy_addr.parse().expect("Unable to parse socket address"),
                tls_config: Some(server),
            }
        } else {
            DenimConfig {
                state: DenimState::new(sam_addr.to_string(), 10, None),
                addr: proxy_addr.parse().expect("Unable to parse socket address"),
                tls_config: None,
            }
        };
        let (tx, started_rx) = oneshot::channel::<()>();
        let thread = tokio::spawn(async move {
            let server = start_proxy(config);
            tx.send(())
                .expect("should be able to inform other thread that server is started");
            server.await
        });
        Self { thread, started_rx }
    }

    pub fn started_rx(&mut self) -> &mut Receiver<()> {
        &mut self.started_rx
    }
}
