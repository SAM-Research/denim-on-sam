use rustls::ClientConfig;
use sam_client::{
    net::{
        protocol::{ProtocolClient, WebSocketProtocolClientConfig},
        HttpClient, HttpClientConfig,
    },
    storage::{InMemoryStoreConfig, InMemoryStoreType},
    Client,
};

fn http_config(addr: &str, https: Option<ClientConfig>) -> HttpClientConfig {
    if let Some(tls) = https {
        HttpClientConfig::new_with_tls(addr.to_string(), tls)
    } else {
        HttpClientConfig::new(addr.to_string())
    }
}

fn ws_config(addr: &str, wss: Option<ClientConfig>) -> WebSocketProtocolClientConfig {
    if let Some(tls) = wss {
        WebSocketProtocolClientConfig::new_with_tls(addr.to_string(), tls)
    } else {
        WebSocketProtocolClientConfig::new(addr.to_string())
    }
}

// TODO: when denim stuff is implemented we need to change this to denim client
pub async fn client_with_proxy(
    proxy_addr: &str,
    sam_addr: &str,
    username: &str,
    device_name: &str,
    https: Option<ClientConfig>,
    wss: Option<ClientConfig>,
) -> Client<InMemoryStoreType, HttpClient, ProtocolClient> {
    Client::from_registration()
        .username(username)
        .device_name(device_name)
        .store_config(InMemoryStoreConfig::default())
        .api_client_config(http_config(sam_addr, https))
        .protocol_config(ws_config(proxy_addr, wss))
        .upload_prekey_count(5)
        .call()
        .await
        .expect("Can register Client")
}
