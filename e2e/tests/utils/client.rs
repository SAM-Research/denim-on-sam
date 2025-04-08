use denim_sam_client::client::InMemoryDenimClientType;
use denim_sam_client::protocol::DenimProtocolClientConfig;
use denim_sam_client::DenimClient;
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use rustls::ClientConfig;
use sam_client::{
    net::{protocol::WebSocketProtocolClientConfig, HttpClientConfig},
    storage::inmem::{InMemorySamStoreConfig, InMemorySignalStoreConfig},
};

#[allow(unused)]
pub fn http_config(addr: &str, https: Option<ClientConfig>) -> HttpClientConfig {
    if let Some(tls) = https {
        HttpClientConfig::new_with_tls(addr.to_string(), tls)
    } else {
        HttpClientConfig::new(addr.to_string())
    }
}

#[allow(unused)]
pub fn ws_config(addr: &str, wss: Option<ClientConfig>) -> WebSocketProtocolClientConfig {
    if let Some(tls) = wss {
        WebSocketProtocolClientConfig::new_with_tls(addr.to_string(), tls)
    } else {
        WebSocketProtocolClientConfig::new(addr.to_string())
    }
}


#[allow(unused, clippy::too_many_arguments)]
pub async fn client_with_proxy(
    proxy_addr: &str,
    sam_addr: &str,
    username: &str,
    device_name: &str,
    https: Option<ClientConfig>,
    wss: Option<ClientConfig>,
    sending_buffer: InMemorySendingBuffer,
    receiving_buffer: InMemoryReceivingBuffer,
) -> DenimClient<InMemoryDenimClientType> {
    DenimClient::from_registration()
        .username(username)
        .device_name(device_name)
        .regular_store_config(InMemorySignalStoreConfig::default())
        .denim_store_config(InMemorySignalStoreConfig::default())
        .sam_store_config(InMemorySamStoreConfig::default())
        .api_client_config(http_config(sam_addr, https))
        .protocol_config(DenimProtocolClientConfig::new(
            ws_config(proxy_addr, wss),
            sending_buffer,
            receiving_buffer,
        ))
        .upload_prekey_count(5)
        .call()
        .await
        .expect("Can register Client")
}
