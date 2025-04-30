use denim_sam_client::message::queue::InMemoryMessageQueueConfig;
use denim_sam_client::store::inmem::InMemoryDeniableStoreConfig;
use denim_sam_client::DenimClient;
use denim_sam_client::{client::InMemoryDenimClientType, protocol::DenimProtocolClientConfig};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use rustls::ClientConfig;
use sam_client::storage::InMemoryStoreConfig;
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use tokio::time::timeout;
use utils::server::TestServerConfigs;
use utils::server::{in_memory_configs, TestServerConfig};
use utils::{
    client::{client_with_proxy, http_config},
    tls::client_config,
};
use uuid::Uuid;

use crate::utils::server::{connection_str, postgres_configs};
use crate::utils::tls::tls_configs;

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[rstest]
#[case::in_memory_tls(
    false,
    in_memory_configs(get_next_port(), get_next_port(), tls_configs(true)),
    client_config(true)
)]
#[case::in_memory(true, in_memory_configs(get_next_port(), get_next_port(), None), None)]
#[ignore = "requires a postgres test database"]
#[case::postgres(
    true,
    postgres_configs(get_next_port(), get_next_port(), None, connection_str()),
    None
)]
#[tokio::test]
pub async fn one_client_can_register(
    #[case] install_tls: bool,
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
    #[case] client_tls: Option<ClientConfig>,
) {
    use uuid::Uuid;

    if install_tls {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let mut server = server_configs.sam.start().await;
        let mut proxy = server_configs.denim.start().await;

        server
            .started_rx()
            .await
            .expect("Should be able to start server");

        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let _ = client_with_proxy(
            proxy.address(),
            server.address(),
            &Uuid::new_v4().to_string(),
            "alice device",
            client_tls,
            InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;
    })
    .await
    .expect("Test took to long to complete")
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
pub async fn can_delete_account(
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
) {
    let mut server = server_configs.sam.start().await;
    let mut proxy = server_configs.denim.start().await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    proxy
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = client_with_proxy(
        &proxy.address(),
        &server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    assert!(client.delete_account().await.is_ok());
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
pub async fn can_delete_a_device(
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
) {
    let mut server = server_configs.sam.start().await;
    let mut proxy = server_configs.denim.start().await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    proxy
        .started_rx()
        .await
        .expect("Should be able to start server");

    let client = client_with_proxy(
        &proxy.address(),
        &server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let result = client.delete_account().await;
    assert!(
        result.is_ok(),
        "Error deleting account: {:?}",
        result.unwrap_err().1
    )
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
pub async fn alice_can_find_bobs_account_id(
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
) {
    let mut server = server_configs.sam.start().await;
    let mut proxy = server_configs.denim.start().await;

    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    proxy
        .started_rx()
        .await
        .expect("Should be able to start server");

    let alice = client_with_proxy(
        &proxy.address(),
        &server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let bob_username = Uuid::new_v4().to_string();
    let bob = client_with_proxy(
        &proxy.address(),
        &server.address(),
        &bob_username,
        "Bob's device",
        None,
        InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let result = alice.get_account_id_for(&bob_username).await;

    assert!(result.is_ok());
    assert_eq!(bob.account_id().await.unwrap(), result.unwrap())
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
pub async fn two_clients_cannot_have_the_same_username(
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
) {
    let mut server = server_configs.sam.start().await;

    let mut proxy = server_configs.denim.start().await;
    server
        .started_rx()
        .await
        .expect("Should be able to start server");

    proxy
        .started_rx()
        .await
        .expect("Should be able to start server");

    let username = Uuid::new_v4().to_string();

    let _alice = client_with_proxy(
        &proxy.address(),
        &server.address(),
        &username,
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_2: Result<DenimClient<InMemoryDenimClientType>, _> = DenimClient::from_registration()
        .username(&username)
        .device_name("Alice's device")
        .store_config(InMemoryStoreConfig::default())
        .deniable_store_config(InMemoryDeniableStoreConfig::default())
        .api_client_config(http_config(&server.address(), None))
        .message_queue_config(InMemoryMessageQueueConfig)
        .protocol_config(DenimProtocolClientConfig::new(
            proxy.address().to_string(),
            None,
            10,
            InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        ))
        .call()
        .await;

    assert!(alice_2.is_err());
}
