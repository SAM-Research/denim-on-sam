use denim_sam_client::{client::SqliteDenimClientType, DenimClient};
use sam_client::{
    net::HttpClientConfig,
    storage::{sqlite::SqliteSamStoreConfig, SqliteSignalStoreConfig},
};
use utils::server::TestSamServer;

mod utils;

#[tokio::test]
pub async fn alice_can_upload_keys() {
    let address = "127.0.0.1:9390".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let mut alice: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .username("Alice")
        .device_name("Alice's Device")
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.clone()))
        .call()
        .await
        .unwrap();

    let publish_keys = alice
        .publish_prekeys()
        .onetime_prekeys(10)
        .new_signed_prekey(true)
        .new_last_resort(true)
        .call()
        .await;

    assert!(publish_keys.is_ok())
}
