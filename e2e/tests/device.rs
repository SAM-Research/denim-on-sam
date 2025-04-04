use denim_sam_client::{client::SqliteDenimClientType, DenimClient};
use sam_client::{
    net::HttpClientConfig,
    storage::{sqlite::SqliteSamStoreConfig, SqliteSignalStoreConfig},
};
use utils::server::TestSamServer;

mod utils;

#[tokio::test]
async fn can_link_device() {
    let address = "127.0.0.1:8070";
    let mut server = TestSamServer::start(address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let username = "Alice";
    let device_name = "Alice's Device";

    let mut alice: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .username(username)
        .device_name(device_name)
        .call()
        .await
        .expect("Can register account");

    let token = alice
        .create_provision()
        .await
        .expect("Can create a link token");

    let id_key_pair = alice
        .identity_key_pair()
        .await
        .expect("Can get id key pair");

    assert!(DenimClient::<SqliteDenimClientType>::from_provisioning()
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .device_name("Alice's Other Device")
        .id_key_pair(id_key_pair)
        .token(token)
        .call()
        .await
        .is_ok());
}

#[tokio::test]
async fn can_unlink_device() {
    let address = "127.0.0.1:8071";
    let mut server = TestSamServer::start(address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let username = "Alice";
    let device_name = "Alice's Device";

    let mut alice: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .username(username)
        .device_name(device_name)
        .call()
        .await
        .expect("Can register account");

    let token = alice
        .create_provision()
        .await
        .expect("Can create a link token");

    let id_key_pair = alice
        .identity_key_pair()
        .await
        .expect("Can get id key pair");

    let other_client: DenimClient<SqliteDenimClientType> = DenimClient::from_provisioning()
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .device_name("Alice's Other Device")
        .id_key_pair(id_key_pair)
        .token(token)
        .call()
        .await
        .expect("Can link device");

    assert!(alice
        .unlink_device(other_client.device_id().await.expect("Can get device_id"))
        .await
        .is_ok())
}

#[tokio::test]
async fn can_delete_device() {
    let address = "127.0.0.1:8072";
    let mut server = TestSamServer::start(address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let username = "Alice";
    let device_name = "Alice's Device";

    let mut alice: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .username(username)
        .device_name(device_name)
        .call()
        .await
        .expect("Can register account");

    let token = alice
        .create_provision()
        .await
        .expect("Can create a link token");

    let id_key_pair = alice
        .identity_key_pair()
        .await
        .expect("Can get id key pair");

    let other_client: DenimClient<SqliteDenimClientType> = DenimClient::from_provisioning()
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .device_name("Alice's Other Device")
        .id_key_pair(id_key_pair)
        .token(token)
        .call()
        .await
        .expect("Can link device");

    assert!(other_client.delete_device().await.is_ok())
}

#[tokio::test]
async fn can_delete_account() {
    let address = "127.0.0.1:8073";
    let mut server = TestSamServer::start(address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let username = "Alice";
    let device_name = "Alice's Device";

    let alice: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.to_owned()))
        .username(username)
        .device_name(device_name)
        .call()
        .await
        .expect("Can register account");

    assert!(alice.delete_account().await.is_ok())
}
