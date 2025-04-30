use denim_sam_proxy::config::{ProxyMtlsConfig, TlsConfig};
use rstest::fixture;
use rustls::ClientConfig;
use sam_net::tls::{create_tls_client_config, MutualTlsConfig};
use sam_server::config::TlsConfig as SamTlsConfig;

#[fixture]
pub fn tls_configs(#[default(false)] mtls: bool) -> Option<(SamTlsConfig, TlsConfig)> {
    Some((sam_config(mtls), proxy_config(mtls)))
}

pub fn sam_config(mtls: bool) -> SamTlsConfig {
    let ca = if mtls {
        Some("./cert/rootCA.crt".to_string())
    } else {
        None
    };
    SamTlsConfig {
        ca_cert_path: ca,
        cert_path: "./cert/server.crt".to_string(),
        key_path: "./cert/server.key".to_string(),
    }
}

pub fn proxy_config(mtls: bool) -> TlsConfig {
    let proxy_client = if mtls {
        Some(ProxyMtlsConfig {
            cert_path: "./cert/client.crt".to_string(),
            key_path: "./cert/client.key".to_string(),
        })
    } else {
        None
    };
    TlsConfig {
        ca_cert_path: "./cert/rootCA.crt".to_string(),
        proxy_cert_path: "./cert/proxy.crt".to_string(),
        proxy_key_path: "./cert/proxy.key".to_string(),
        proxy_client,
        proxy_mtls: false,
    }
}

#[fixture]
pub fn client_config(#[default(false)] mtls: bool) -> Option<ClientConfig> {
    let mutual = if mtls {
        Some(MutualTlsConfig::new(
            "./cert/client.key".to_string(),
            "./cert/client.crt".to_string(),
        ))
    } else {
        None
    };

    Some(create_tls_client_config("./cert/rootCA.crt", mutual).expect("Can create client tls"))
}
