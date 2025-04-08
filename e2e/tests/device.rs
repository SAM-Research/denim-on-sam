use std::time::Duration;

use denim_sam_client::{
    client::SqliteDenimClientType, protocol::DenimProtocolClientConfig, DenimClient,
};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use sam_client::{
    net::HttpClientConfig,
    storage::{sqlite::SqliteSamStoreConfig, SqliteSignalStoreConfig},
};
use tokio::time::timeout;
use utils::{
    client::{client_with_proxy, ws_config},
    server::{TestDenimProxy, TestSamServer},
};

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[tokio::test]
async fn can_link_device() {
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8070".to_owned();
        let proxy_addr = "127.0.0.1:8071".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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

        assert!(DenimClient::<SqliteDenimClientType>::from_provisioning()
            .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
            .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
            .sam_store_config(SqliteSamStoreConfig::in_memory().await)
            .api_client_config(HttpClientConfig::new(sam_addr.to_owned()))
            .protocol_config(DenimProtocolClientConfig::new(
                ws_config(&proxy_addr, None),
                InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
                InMemoryReceivingBuffer::default(),
            ))
            .device_name("Alice's Other Device")
            .id_key_pair(id_key_pair)
            .token(token)
            .call()
            .await
            .is_ok());
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
async fn can_unlink_device() {
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8072".to_owned();
        let proxy_addr = "127.0.0.1:8073".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
            .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
            .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
            .sam_store_config(SqliteSamStoreConfig::in_memory().await)
            .protocol_config(DenimProtocolClientConfig::new(
                ws_config(&proxy_addr, None),
                InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8074".to_owned();
        let proxy_addr = "127.0.0.1:8075".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
            .regular_store_config(SqliteSignalStoreConfig::in_memory().await)
            .denim_store_config(SqliteSignalStoreConfig::in_memory().await)
            .sam_store_config(SqliteSamStoreConfig::in_memory().await)
            .protocol_config(DenimProtocolClientConfig::new(
                ws_config(&proxy_addr, None),
                InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8076".to_owned();
        let proxy_addr = "127.0.0.1:8077".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;

        assert!(alice.delete_account().await.is_ok())
    })
    .await
    .expect("Test took to long to complete");
}
