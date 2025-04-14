use denim_sam_client::message::queue::InMemoryMessageQueueConfig;
use denim_sam_client::DenimClient;
use denim_sam_client::{client::InMemoryDenimClientType, protocol::DenimProtocolClientConfig};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use rstest::rstest;
use sam_client::storage::InMemoryStoreConfig;

use denim_sam_client::store::inmem::InMemoryDeniableStoreConfig;
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

// TODO: Test with tls once fully integrated.
#[rstest]
#[case(false, None, None, None, "8090", "8091")]
#[tokio::test]
pub async fn one_client_can_register(
    #[case] install_tls: bool,
    #[case] sam_tls: Option<bool>,
    #[case] proxy_tls: Option<bool>,
    #[case] client_https: Option<bool>,
    #[case] port: &str,
    #[case] proxy_port: &str,
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

        let sending_buffer = InMemorySendingBuffer::new(0.5, 10).expect("Can make sending buffer");
        let receiving_buffer = InMemoryReceivingBuffer::default();

        let alice: Result<DenimClient<InMemoryDenimClientType>, _> =
            DenimClient::from_registration()
                .username("Alice")
                .device_name("Alice's device")
                .store_config(InMemoryStoreConfig::default())
                .deniable_store_config(InMemoryDeniableStoreConfig::default())
                .api_client_config(http_config(&sam_addr, client_https.map(client_config)))
                .message_queue_config(InMemoryMessageQueueConfig)
                .protocol_config(DenimProtocolClientConfig::new(
                    proxy_addr,
                    None,
                    sending_buffer,
                    receiving_buffer,
                ))
                .call()
                .await;

        assert!(alice.is_ok())
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
pub async fn can_delete_account() {
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8094".to_owned();
        let proxy_addr = "127.0.0.1:8095".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8096".to_owned();
        let proxy_addr = "127.0.0.1:8097".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8098".to_owned();
        let proxy_addr = "127.0.0.1:8099".to_owned();
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

        let bob = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Bob",
            "Bob's device",
            None,
            None,
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8100".to_owned();
        let proxy_addr = "127.0.0.1:8101".to_owned();
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
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
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
                    InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
                    InMemoryReceivingBuffer::default(),
                ))
                .call()
                .await;

        assert!(alice_2.is_err());
    })
    .await
    .expect("Test took to long to complete")
}
