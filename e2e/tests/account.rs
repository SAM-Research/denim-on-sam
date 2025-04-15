use denim_sam_client::message::queue::InMemoryMessageQueueConfig;
use denim_sam_client::DenimClient;
use denim_sam_client::{client::InMemoryDenimClientType, protocol::DenimProtocolClientConfig};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use rstest::rstest;
use sam_client::storage::InMemoryStoreConfig;

use denim_sam_client::store::inmem::InMemoryDeniableStoreConfig;
use sam_test_utils::get_next_port;
use std::time::Duration;
use tokio::time::timeout;
use utils::server::TestDenimProxy;
use utils::{
    client::client_with_proxy,
    tls::{client_config, sam_config},
};

use utils::tls::proxy_config;
use utils::{client::http_config, server::TestSamServer};

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[rstest]
#[case(false, None, None, None, None, get_next_port(), get_next_port())]
#[case(
    true,
    Some(true),
    Some(true),
    Some(true),
    Some(false),
    get_next_port(),
    get_next_port()
)]
#[tokio::test]
pub async fn one_client_can_register(
    #[case] install_tls: bool,
    #[case] sam_tls: Option<bool>,
    #[case] proxy_tls: Option<bool>,
    #[case] client_https: Option<bool>,
    #[case] client_wss: Option<bool>,
    #[case] port: u16,
    #[case] proxy_port: u16,
) {
    if install_tls {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = format!("127.0.0.1:{port}");
        let proxy_addr = format!("127.0.0.1:{proxy_port}");
        let mut server = TestSamServer::start(
            &sam_addr,
            sam_tls.map(|x| sam_config(x).try_into().expect("can create sam tls")),
        )
        .await;
        let mut proxy =
            TestDenimProxy::start(&sam_addr, &proxy_addr, proxy_tls.map(proxy_config)).await;
        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let _ = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "alice",
            "alice device",
            client_https.map(client_config),
            client_wss.map(client_config),
            InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
pub async fn can_delete_account() {
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

        let client = client_with_proxy(
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

        assert!(client.delete_account().await.is_ok());
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
pub async fn can_delete_a_device() {
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

        let client = client_with_proxy(
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

        let result = client.delete_account().await;
        assert!(
            result.is_ok(),
            "Error deleting account: {:?}",
            result.unwrap_err().1
        )
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
pub async fn alice_can_find_bobs_account_id() {
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

        let bob = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Bob",
            "Bob's device",
            None,
            None,
            InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;

        let result = alice.get_account_id_for("Bob").await;

        assert!(result.is_ok());
        assert_eq!(bob.account_id().await.unwrap(), result.unwrap())
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
pub async fn two_clients_cannot_have_the_same_username() {
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

        let _alice = client_with_proxy(
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

        let alice_2: Result<DenimClient<InMemoryDenimClientType>, _> =
            DenimClient::from_registration()
                .username("Alice")
                .device_name("Alice's device")
                .store_config(InMemoryStoreConfig::default())
                .deniable_store_config(InMemoryDeniableStoreConfig::default())
                .api_client_config(http_config(&sam_addr, None))
                .message_queue_config(InMemoryMessageQueueConfig)
                .protocol_config(DenimProtocolClientConfig::new(
                    proxy_addr,
                    None,
                    10,
                    InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
                    InMemoryReceivingBuffer::default(),
                ))
                .call()
                .await;

        assert!(alice_2.is_err());
    })
    .await
    .expect("Test took to long to complete")
}
