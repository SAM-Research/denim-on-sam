use std::time::Duration;

use rstest::rstest;
use utils::tls::{client_config, proxy_config, sam_config};

use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use tokio::time::{sleep, timeout};
use utils::{
    client::client_with_proxy,
    server::{TestDenimProxy, TestSamServer},
};

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[rstest]
#[case(false, None, None, None, None, "8080", "8081")]
#[case(true, Some(true), Some(true), Some(true), Some(false), "8082", "8083")]
#[tokio::test]
async fn can_connect(
    #[case] install_tls: bool,
    #[case] sam_tls: Option<bool>,
    #[case] proxy_tls: Option<bool>,
    #[case] client_https: Option<bool>,
    #[case] client_wss: Option<bool>,
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

        let client = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "alice",
            "alice device",
            client_https.map(client_config),
            client_wss.map(client_config),
            InMemorySendingBuffer::new(0.5, 10).expect("Can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;
        sleep(Duration::from_millis(300)).await;
        assert!(client.is_connected().await)
    })
    .await
    .expect("Test took to long to complete")
}

#[rstest]
#[case(false, None, None, None, None, "8084", "8085")]
#[case(true, Some(true), Some(true), Some(true), Some(false), "8086", "8087")]
#[tokio::test]
#[ignore = "DenimClient needs implementation of process_messages"]
async fn can_send_message(
    #[case] install_tls: bool,
    #[case] sam_tls: Option<bool>,
    #[case] proxy_tls: Option<bool>,
    #[case] client_https: Option<bool>,
    #[case] client_wss: Option<bool>,
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

        let mut alice = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Alice",
            "Alice's device",
            client_https.map(client_config),
            client_wss.map(client_config),
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;
        let mut bob = client_with_proxy(
            &proxy_addr,
            &sam_addr,
            "Bob",
            "Bob's device",
            client_https.map(client_config),
            client_wss.map(client_config),
            InMemorySendingBuffer::new(0.5, 10).expect("can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;

        let alice_id = alice.account_id().await.expect("Can get alice id");
        let bob_id = bob.account_id().await.expect("Can get bob id");

        let expected_msg = "Hello bob through denim proxy!";
        alice
            .send_message(bob_id, expected_msg)
            .await
            .expect("Can send message");

        let mut bob_recv = bob.subscribe();
        bob.process_messages_blocking()
            .await
            .expect("Can process messages");

        let msg = bob_recv.recv().await.expect("Can receive message");
        let bob_msg = String::from_utf8_lossy(msg.content_bytes());

        assert!(bob_msg == expected_msg);
        assert!(msg.source_account_id() == alice_id);
    })
    .await
    .expect("Test took to long to complete")
}
