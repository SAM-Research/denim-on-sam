use clap::{Arg, Command};
use denim_sam_common::buffers::in_mem::{
    InMemoryReceivingBufferConfig, InMemorySendingBufferConfig,
};
use denim_sam_proxy::{
    config::TlsConfig,
    error::CliError,
    managers::{
        in_mem::{InMemoryDenimEcPreKeyManager, InMemoryDenimSignedPreKeyManager},
        BufferManager, DenimKeyManager, InMemoryMessageIdProvider,
    },
    server,
    state::{self, InMemory, InMemoryBufferManagerType},
};
use log::{debug, error, info};
use sam_server::managers::in_memory::{
    account::InMemoryAccountManager, device::InMemoryDeviceManager,
};
use server::{start_proxy, DenimConfig};
use std::io::BufReader;

use state::DenimState;

async fn cli() -> Result<(), CliError> {
    let matches = Command::new("sam_server")
        .arg(
            Arg::new("sam_ip")
                .long("sam-ip")
                .required(false)
                .help("IP to run sam server on")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("proxy_ip")
                .long("proxy-ip")
                .required(false)
                .help("IP to run proxy on")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("sam_port")
                .long("sam-port")
                .required(false)
                .help("Port to run sam on")
                .default_value("8080"),
        )
        .arg(
            Arg::new("proxy_port")
                .long("proxy-port")
                .required(false)
                .help("Port to run sam on")
                .default_value("8081"),
        )
        .arg(
            Arg::new("config")
                .short('t')
                .long("tls-config")
                .required(false)
                .help("JSON TLS Config path"),
        )
        .get_matches();

    let ip = matches
        .get_one::<String>("proxy_ip")
        .ok_or(CliError::ArgumentError("Expected Proxy IP".to_string()))?;
    let port = matches
        .get_one::<String>("proxy_port")
        .ok_or(CliError::ArgumentError("Expected Proxy port".to_string()))?;
    let sam_ip = matches
        .get_one::<String>("sam_ip")
        .ok_or(CliError::ArgumentError("Expected SAM IP".to_string()))?;
    let sam_port = matches
        .get_one::<String>("sam_port")
        .ok_or(CliError::ArgumentError("Expected SAM port".to_string()))?;

    let addr = format!("{}:{}", ip, port)
        .parse()
        .inspect_err(|e| debug!("{e}"))
        .map_err(|_| CliError::AddressParseError)?;

    let tls_config = if let Some(config_path) = matches.get_one::<String>("config") {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let file = std::fs::File::open(config_path)?;
        let reader = BufReader::new(file);
        Some(TlsConfig::load(reader)?.create()?)
    } else {
        None
    };

    let rcfg = InMemoryReceivingBufferConfig;
    // TODO: this should be configurable
    let scfg = InMemorySendingBufferConfig::builder()
        .min_payload_length(10)
        .q(1.0)
        .build();
    let id_provider = InMemoryMessageIdProvider::default();
    let buffer_mgr: BufferManager<InMemoryBufferManagerType> =
        BufferManager::new(rcfg, scfg, id_provider);

    let config = if let Some((server, client)) = tls_config {
        DenimConfig {
            addr,
            tls_config: Some(server),
            state: DenimState::<InMemory>::new(
                format!("{}:{}", sam_ip, sam_port),
                10,
                Some(client),
                buffer_mgr,
                DenimKeyManager::new(
                    InMemoryDenimEcPreKeyManager::default(),
                    InMemoryDenimSignedPreKeyManager::default(),
                ),
                InMemoryAccountManager::default(),
                InMemoryDeviceManager::new("Test".to_owned(), 120),
            ),
        }
    } else {
        DenimConfig {
            state: DenimState::new(
                format!("{}:{}", sam_ip, sam_port),
                10,
                None,
                buffer_mgr,
                DenimKeyManager::new(
                    InMemoryDenimEcPreKeyManager::default(),
                    InMemoryDenimSignedPreKeyManager::default(),
                ),
                InMemoryAccountManager::default(),
                InMemoryDeviceManager::new("Test".to_owned(), 120),
            ),
            addr,
            tls_config: None,
        }
    };

    start_proxy(config)
        .await
        .inspect_err(|e| debug!("{e}"))
        .map_err(|_| CliError::FailedToStartProxy)
}

#[tokio::main]
async fn main() {
    env_logger::init();
    match cli().await {
        Ok(_) => info!("Goodbye!"),
        Err(e) => error!("Fatal Proxy Error: {}", e),
    }
}
