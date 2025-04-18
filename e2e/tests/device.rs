use std::time::Duration;

use denim_sam_client::{
    client::SqliteDenimClientType, message::queue::InMemoryMessageQueueConfig,
    protocol::DenimProtocolClientConfig, store::InMemoryDeniableStoreConfig, DenimClient,
};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use sam_client::{net::HttpClientConfig, storage::SqliteStoreConfig};
use sam_test_utils::get_next_port;
use tokio::time::timeout;
use utils::{
    client::client_with_proxy,
    server::{TestDenimProxy, TestSamServer},
};

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[tokio::test]
async fn can_link_device() {
    let (sam_port, denim_port) = (get_next_port(), get_next_port());
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = format!("127.0.0.1:{sam_port}");
        let proxy_addr = format!("127.0.0.1:{denim_port}");
        let mut server = TestSamServer::start(&sam_addr, None).await;
        let mut proxy = TestDenimProxy::start(&sam_addr, &proxy_addr, None).await;

        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let mut alice = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Alice",
            "Alice's device",
            None,
            None,
            InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
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
            .store_config(SqliteStoreConfig::in_memory().await)
            .deniable_store_config(InMemoryDeniableStoreConfig::default())
            .api_client_config(HttpClientConfig::new(sam_addr.to_owned()))
            .message_queue_config(InMemoryMessageQueueConfig)
            .protocol_config(DenimProtocolClientConfig::new(
                proxy_addr,
                None,
                10,
                InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
                InMemoryReceivingBuffer::default(),
            ))
            .device_name("Alice's Other Device")
            .id_key_pair(id_key_pair)
            .token(token)
            .call()
            .await;

        assert!(new_device.is_ok());
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
async fn can_unlink_device() {
    let (sam_port, denim_port) = (get_next_port(), get_next_port());
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = format!("127.0.0.1:{sam_port}");
        let proxy_addr = format!("127.0.0.1:{denim_port}");
        let mut server = TestSamServer::start(&sam_addr, None).await;
        let mut proxy = TestDenimProxy::start(&sam_addr, &proxy_addr, None).await;

        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let mut alice = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Alice",
            "Alice's device",
            None,
            None,
            InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
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
            .api_client_config(HttpClientConfig::new(sam_addr.to_owned()))
            .store_config(SqliteStoreConfig::in_memory().await)
            .deniable_store_config(InMemoryDeniableStoreConfig::default())
            .message_queue_config(InMemoryMessageQueueConfig)
            .protocol_config(DenimProtocolClientConfig::new(
                proxy_addr,
                None,
                10,
                InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
                InMemoryReceivingBuffer::default(),
            ))
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
    })
    .await
    .expect("Test took to long to complete");
}

#[tokio::test]
async fn can_delete_device() {
    let (sam_port, denim_port) = (get_next_port(), get_next_port());
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = format!("127.0.0.1:{sam_port}");
        let proxy_addr = format!("127.0.0.1:{denim_port}");
        let mut server = TestSamServer::start(&sam_addr, None).await;
        let mut proxy = TestDenimProxy::start(&sam_addr, &proxy_addr, None).await;

        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let mut alice = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Alice",
            "Alice's device",
            None,
            None,
            InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
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
            .api_client_config(HttpClientConfig::new(sam_addr.to_owned()))
            .store_config(SqliteStoreConfig::in_memory().await)
            .deniable_store_config(InMemoryDeniableStoreConfig::default())
            .message_queue_config(InMemoryMessageQueueConfig)
            .protocol_config(DenimProtocolClientConfig::new(
                proxy_addr,
                None,
                10,
                InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
                InMemoryReceivingBuffer::default(),
            ))
            .device_name("Alice's Other Device")
            .id_key_pair(id_key_pair)
            .token(token)
            .call()
            .await
            .expect("Can link device");

        assert!(other_client.delete_device().await.is_ok())
    })
    .await
    .expect("Test took to long to complete");
}

#[tokio::test]
async fn can_delete_account() {
    let (sam_port, denim_port) = (get_next_port(), get_next_port());
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = format!("127.0.0.1:{sam_port}").to_owned();
        let proxy_addr = format!("127.0.0.1:{denim_port}").to_owned();
        let mut server = TestSamServer::start(&sam_addr, None).await;
        let mut proxy = TestDenimProxy::start(&sam_addr, &proxy_addr, None).await;

        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let alice = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Alice",
            "Alice's device",
            None,
            None,
            InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;

        assert!(alice.delete_account().await.is_ok())
    })
    .await
    .expect("Test took to long to complete");
}
