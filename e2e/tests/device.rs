use denim_sam_client::store::sqlite::SqliteDeniableStoreConfig;
use denim_sam_client::{
    client::SqliteDenimClientType, message::queue::InMemoryMessageQueueConfig,
    protocol::DenimProtocolClientConfig, DenimClient,
};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use sam_client::{net::HttpClientConfig, storage::SqliteStoreConfig};
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use utils::client::client_with_proxy;
use utils::server::in_memory_configs;
use utils::server::TestServerConfig;
use utils::server::TestServerConfigs;
use uuid::Uuid;
mod utils;

const TIMEOUT_SECS: u64 = 20;

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn can_link_device(
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

    let mut alice = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let token = alice
        .create_provision()
        .await
        .expect("Can create a link token");

    let id_key_pair = alice
        .identity_key_pair()
        .await
        .expect("Can get id key pair");

    let new_device = DenimClient::<SqliteDenimClientType>::from_provisioning()
        .store_config(
            SqliteStoreConfig::in_memory(10)
                .await
                .expect("can create inmemory"),
        )
        .deniable_store_config(
            SqliteDeniableStoreConfig::in_memory(10)
                .await
                .expect("can create inmemory"),
        )
        .api_client_config(HttpClientConfig::new(server.address().to_owned()))
        .message_queue_config(InMemoryMessageQueueConfig)
        .protocol_config(DenimProtocolClientConfig::new(
            proxy.address().to_owned(),
            None,
            10,
            InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        ))
        .device_name("Alice's Other Device")
        .id_key_pair(id_key_pair)
        .token(token)
        .call()
        .await;

    assert!(new_device.is_ok());
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn can_unlink_device(
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

    let mut alice = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let token = alice
        .create_provision()
        .await
        .expect("Can create a link token");

    let id_key_pair = alice
        .identity_key_pair()
        .await
        .expect("Can get id key pair");

    let other_client: DenimClient<SqliteDenimClientType> = DenimClient::from_provisioning()
        .api_client_config(HttpClientConfig::new(server.address().to_owned()))
        .store_config(
            SqliteStoreConfig::in_memory(10)
                .await
                .expect("can create inmemory"),
        )
        .deniable_store_config(
            SqliteDeniableStoreConfig::in_memory(10)
                .await
                .expect("can create inmemory"),
        )
        .message_queue_config(InMemoryMessageQueueConfig)
        .protocol_config(DenimProtocolClientConfig::new(
            proxy.address().to_owned(),
            None,
            10,
            InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        ))
        .device_name("Alice's Other Device")
        .id_key_pair(id_key_pair)
        .token(token)
        .call()
        .await
        .expect("Can link device");

    assert!(alice.unlink_device(other_client.device_id()).await.is_ok())
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn can_delete_device(
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

    let mut alice = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let token = alice
        .create_provision()
        .await
        .expect("Can create a link token");

    let id_key_pair = alice
        .identity_key_pair()
        .await
        .expect("Can get id key pair");

    let other_client: DenimClient<SqliteDenimClientType> = DenimClient::from_provisioning()
        .api_client_config(HttpClientConfig::new(server.address().to_owned()))
        .store_config(
            SqliteStoreConfig::in_memory(10)
                .await
                .expect("can create inmemory"),
        )
        .deniable_store_config(
            SqliteDeniableStoreConfig::in_memory(10)
                .await
                .expect("can create inmemory"),
        )
        .message_queue_config(InMemoryMessageQueueConfig)
        .protocol_config(DenimProtocolClientConfig::new(
            proxy.address().to_owned(),
            None,
            10,
            InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        ))
        .device_name("Alice's Other Device")
        .id_key_pair(id_key_pair)
        .token(token)
        .call()
        .await
        .expect("Can link device");

    assert!(other_client.delete_device().await.is_ok())
}

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn can_delete_account(
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
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    assert!(alice.delete_account().await.is_ok())
}
