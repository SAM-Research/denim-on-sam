use denim_sam_proxy::config::{ProxyMtlsConfig, TlsConfig};
use rustls::ClientConfig;
use sam_net::tls::{create_tls_client_config, MutualTlsConfig};
use sam_server::config::TlsConfig as SamTlsConfig;

#[allow(unused)]
pub fn tls_configs(mtls: bool) -> Option<(SamTlsConfig, TlsConfig)> {
    Some((sam_config(mtls), proxy_config(mtls)))
}

#[allow(unused)]
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

#[allow(unused)]
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

#[allow(unused)]
pub fn client_config(mtls: bool) -> Option<ClientConfig> {
    let mutual = if mtls {
        Some(MutualTlsConfig::new(
            "./cert/client.key".to_string(),
            "./cert/client.crt".to_string(),
        ))
    } else {
        None
    };

    let _ = rustls::crypto::ring::default_provider().install_default();
    Some(create_tls_client_config("./cert/rootCA.crt", mutual).expect("Can create client tls"))
}
