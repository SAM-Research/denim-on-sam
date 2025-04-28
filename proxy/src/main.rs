use clap::{Arg, Command};
use denim_sam_proxy::{
    config::DenimCliConfig,
    error::CliError,
    server::{start_proxy, DenimConfig},
};
use log::{debug, error, info};
use std::io::BufReader;

const DEFAULT_SAM_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_PROXY_ADDR: &str = "127.0.0.1:8081";
const DEFAULT_DENIABLE_RATIO: f32 = 1.0; // q
const DEFAULT_CHANNEL_BUFFER_SIZE: usize = 10;
const DEFAULT_KEY_GENERATE_AMOUNT: usize = 10;

fn welcome(config: &DenimCliConfig) {
    let sam_addr = config
        .sam_address
        .clone()
        .unwrap_or(DEFAULT_SAM_ADDR.to_string());
    let proxy_addr = config
        .denim_proxy_address
        .clone()
        .unwrap_or(DEFAULT_PROXY_ADDR.to_string());
    let den_rat = config.deniable_ratio.unwrap_or(DEFAULT_DENIABLE_RATIO);
    let channel_buffer = config
        .channel_buffer_size
        .unwrap_or(DEFAULT_CHANNEL_BUFFER_SIZE);
    info!("*********Configuration*********");
    info!("SAM Address: {sam_addr}");
    info!("Proxy Address: {proxy_addr}");
    info!("Deniable Ratio (q): {den_rat}");
    info!("Channel Buffer size: {channel_buffer}");
    if let Some(tls) = &config.tls {
        info!("Proxy Clients requires mTLS: {}", tls.proxy_mtls);
        info!("Certificate Authority: {}", tls.ca_cert_path);
        info!("Proxy Certificate: {}", tls.proxy_cert_path);
        info!("Proxy Key: {}", tls.proxy_key_path);
        if let Some(sam_mtls) = &tls.proxy_client {
            info!("SAM Connection: mTLS");
            info!("SAM Client Certificate: {}", sam_mtls.cert_path);
            info!("SAM Client Key: {}", sam_mtls.key_path);
        } else {
            info!("SAM Connection: TLS")
        }
    } else {
        info!("SAM Connection: Insecure")
    }
    info!("*******************************");
}

async fn cli() -> Result<(), CliError> {
    let matches = Command::new("sam_server")
        .arg(
            Arg::new("database_url")
                .short('d')
                .long("database-url")
                .required(true)
                .help("PostgreSQL connection url")
                .conflicts_with("config"),
        )
        .arg(
            Arg::new("sam_address")
                .short('s')
                .long("sam-address")
                .required(false)
                .help("Address to run sam server on")
                .default_value(DEFAULT_SAM_ADDR)
                .conflicts_with("config"),
        )
        .arg(
            Arg::new("proxy_address")
                .short('p')
                .long("proxy-address")
                .required(false)
                .help("Address to run proxy on")
                .default_value(DEFAULT_PROXY_ADDR)
                .conflicts_with("config"),
        )
        .arg(
            Arg::new("deniable_ratio")
                .short('q')
                .long("deniable-ratio")
                .required(false)
                .help("Deniable to regular payload ratio (q)")
                .default_value(DEFAULT_DENIABLE_RATIO.to_string())
                .conflicts_with("config"),
        )
        .arg(
            Arg::new("buffer_size")
                .short('b')
                .long("buffer-size")
                .required(false)
                .help("How many messages can be in a buffer channel before blocking behaviour")
                .default_value(DEFAULT_CHANNEL_BUFFER_SIZE.to_string())
                .conflicts_with("config"),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .required(false)
                .help("JSON Config path"),
        )
        .get_matches();

    let config = if let Some(config_path) = matches.get_one::<String>("config") {
        let file = std::fs::File::open(config_path)?;
        let reader = BufReader::new(file);
        DenimCliConfig::load(reader)?
    } else {
        let url = matches
            .get_one::<String>("database_url")
            .ok_or(CliError::ArgumentError("Expected Database url".to_string()))?;
        let proxy_addr = matches.get_one::<String>("proxy_address");

        let sam_addr = matches.get_one::<String>("sam_address");
        let deniable_ratio = if let Some(x) = matches.get_one::<String>("deniable_ratio") {
            Some(x.parse().map_err(|_| {
                CliError::ArgumentError("Expected float for deniable ratio".to_string())
            })?)
        } else {
            None
        };
        let buffer_size = matches
            .get_one::<String>("buffer_size")
            .ok_or(CliError::ArgumentError("Expected buffer size".to_string()))?
            .parse()
            .map_err(|_| {
                CliError::ArgumentError("Expected usize for deniable ratio. On 32 bit target, this is 4 bytes and on a 64 bit target, this is 8 bytes".to_string())
            })?;

        DenimCliConfig::new(
            url.clone(),
            sam_addr.cloned(),
            proxy_addr.cloned(),
            deniable_ratio,
            None,
            Some(buffer_size),
            None,
            None,
        )
    };

    if let Some(filter) = &config.logging {
        env_logger::builder().parse_filters(filter).init();
    } else {
        env_logger::init();
    }

    welcome(&config);
    let tls_config = if let Some(tls_config) = config.tls {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Some(tls_config.create()?)
    } else {
        None
    };

    let addr = config
        .denim_proxy_address
        .unwrap_or(DEFAULT_PROXY_ADDR.to_string())
        .parse()
        .inspect_err(|e| debug!("{e}"))
        .map_err(|_| CliError::AddressParseError)?;

    let (tls_config, ws_proxy_tls_config) = match tls_config {
        Some((server, client)) => (Some(server), Some(client)),
        None => (None, None),
    };

    let denim_cfg = DenimConfig::postgres()
        .url(config.database_url)
        .addr(addr)
        .sam_address(config.sam_address.unwrap_or(DEFAULT_SAM_ADDR.to_string()))
        .maybe_tls_config(tls_config)
        .maybe_ws_proxy_tls_config(ws_proxy_tls_config)
        .channel_buffer_size(
            config
                .channel_buffer_size
                .unwrap_or(DEFAULT_CHANNEL_BUFFER_SIZE),
        )
        .deniable_ratio(config.deniable_ratio.unwrap_or(DEFAULT_DENIABLE_RATIO))
        .key_generate_amount(
            config
                .key_generate_amount
                .unwrap_or(DEFAULT_KEY_GENERATE_AMOUNT),
        )
        .call()
        .await?;
    info!("Database: OK");

    start_proxy(denim_cfg)
        .await
        .inspect_err(|e| debug!("{e}"))
        .map_err(|_| CliError::FailedToStartProxy)
}

#[tokio::main]
async fn main() {
    let res = cli().await;
    let _ = env_logger::try_init();
    match res {
        Ok(_) => info!("Goodbye!"),
        Err(e) => error!("Fatal Proxy Error: {}", e),
    }
}
