use sam_client::{
    net::{
        protocol::{ProtocolClient, WebSocketProtocolClientConfig},
        HttpClient, HttpClientConfig,
    },
    storage::{SqliteStoreConfig, SqliteStoreType},
    Client,
};

// TODO: when denim stuff is implemented we need to change this to denim client
pub async fn client_with_proxy(
    proxy_addr: &str,
    sam_addr: &str,
    username: &str,
    device_name: &str,
) -> Client<SqliteStoreType, HttpClient, ProtocolClient> {
    Client::from_registration()
        .username(username)
        .device_name(device_name)
        .store_config(SqliteStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(sam_addr.to_string()))
        .protocol_config(WebSocketProtocolClientConfig::new(proxy_addr.to_string()))
        .upload_prekey_count(5)
        .call()
        .await
        .expect("Can register Client")
}
