use denim_sam_client::{client::SqliteDenimClientType, DenimClient, DenimClientError};
use sam_client::{
    net::HttpClientConfig,
    storage::{sqlite::SqliteSamStoreConfig, SqliteSignalStoreConfig},
};
use utils::server::TestSamServer;

mod utils;

pub async fn register_alice(
    address: String,
) -> Result<DenimClient<SqliteDenimClientType>, DenimClientError> {
    DenimClient::from_registration()
        .username("Alice")
        .device_name("Alice's Device")
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.clone()))
        .call()
        .await
}

#[tokio::test]
pub async fn one_client_can_register() {
    let sam_address = "127.0.0.1:8090".to_owned();
    let mut server = TestSamServer::start(&sam_address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = register_alice(sam_address).await;

    assert!(client.is_ok());
}

#[tokio::test]
pub async fn can_delete_account() {
    let address = "127.0.0.1:8091".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = register_alice(address).await.expect("Can register account");

    assert!(client.delete_account().await.is_ok());
}

#[tokio::test]
pub async fn can_delete_a_device() {
    let address = "127.0.0.1:8092".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = register_alice(address).await.expect("Can register account");

    let result = client.delete_account().await;
    assert!(
        result.is_ok(),
        "Error deleting account: {:?}",
        result.unwrap_err().1
    )
}

#[tokio::test]
pub async fn alice_can_find_bobs_account_id() {
    let address = "127.0.0.1:8093".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let alice = register_alice(address.clone())
        .await
        .expect("Can create account");

    let bob: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .username("Bob")
        .device_name("Bob's Device")
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.clone()))
        .call()
        .await
        .unwrap();

    let result = alice.get_account_id_for("Bob").await;

    assert!(result.is_ok());
    assert_eq!(bob.account_id().await.unwrap(), result.unwrap())
}
