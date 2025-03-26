use std::io::BufReader;

use clap::{Arg, Command};
use config::TlsConfig;
use error::CliError;

use log::{debug, error, info};
use server::{start_proxy, DenimConfig};

use state::DenimState;

pub mod config;
mod error;
mod proxy;
mod routes;
mod server;
mod state;
mod utils;

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

    let config = if let Some((server, client)) = tls_config {
        DenimConfig {
            state: DenimState::new(format!("{}:{}", sam_ip, sam_port), 10, Some(client)),
            addr,
            tls_config: Some(server),
        }
    } else {
        DenimConfig {
            state: DenimState::new(format!("{}:{}", sam_ip, sam_port), 10, None),
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
