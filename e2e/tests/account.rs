use denim_sam_client::{client::SqliteDenimClientType, DenimClient, DenimClientError};
use rstest::rstest;
use rustls::ClientConfig;
use sam_client::{
    net::HttpClientConfig,
    storage::{sqlite::SqliteSamStoreConfig, SqliteSignalStoreConfig},
};
use utils::tls::{client_config, sam_config};
use utils::{client::http_config, server::TestSamServer};

mod utils;

pub async fn register_alice(
    address: String,
    https: Option<ClientConfig>,
) -> Result<DenimClient<SqliteDenimClientType>, DenimClientError> {
    DenimClient::from_registration()
        .username("Alice")
        .device_name("Alice's Device")
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .api_client_config(http_config(&address, https))
        .call()
        .await
}

#[rstest]
#[case(false, None, None, "8090")]
#[case(true, Some(true), Some(true), "8091")]
#[tokio::test]
pub async fn one_client_can_register(
    #[case] install_tls: bool,
    #[case] sam_tls: Option<bool>,
    #[case] client_https: Option<bool>,
    #[case] port: &str,
) {
    if install_tls {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let sam_address = format!("127.0.0.1:{}", port);
    let mut server = TestSamServer::start(
        &sam_address,
        sam_tls.map(|x| sam_config(x).try_into().expect("can create sam tls")),
    )
    .await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = register_alice(sam_address, client_https.map(client_config)).await;

    assert!(client.is_ok());
}

#[tokio::test]
pub async fn can_delete_account() {
    let address = "127.0.0.1:8092".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = register_alice(address, None)
        .await
        .expect("Can register account");

    assert!(client.delete_account().await.is_ok());
}

#[tokio::test]
pub async fn can_delete_a_device() {
    let address = "127.0.0.1:8093".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = register_alice(address, None)
        .await
        .expect("Can register account");

    let result = client.delete_account().await;
    assert!(
        result.is_ok(),
        "Error deleting account: {:?}",
        result.unwrap_err().1
    )
}

#[tokio::test]
pub async fn alice_can_find_bobs_account_id() {
    let address = "127.0.0.1:8094".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let alice = register_alice(address.clone(), None)
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

#[tokio::test]
pub async fn two_clients_cannot_have_the_same_username() {
    let address = "127.0.0.1:8095".to_owned();
    let mut server = TestSamServer::start(&address, None).await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    let _alice: DenimClient<SqliteDenimClientType> = DenimClient::from_registration()
        .username("Alice")
        .device_name("Alice's Device")
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.clone()))
        .call()
        .await
        .expect("Can make Alice");

    let alice_2: Result<DenimClient<SqliteDenimClientType>, _> = DenimClient::from_registration()
        .username("Alice")
        .device_name("Alice's Device")
        .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
        .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
        .sam_store_config(SqliteSamStoreConfig::in_memory().await)
        .api_client_config(HttpClientConfig::new(address.clone()))
        .call()
        .await;

    assert!(alice_2.is_err());
}
