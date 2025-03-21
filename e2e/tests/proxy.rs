use std::time::Duration;

use tokio::time::{sleep, timeout};
use utils::{
    client::client_with_proxy,
    server::{TestDenimProxy, TestSamServer},
};

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[tokio::test]
async fn can_connect() {
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8080";
        let proxy_addr = "127.0.0.1:8081";
        let mut server = TestSamServer::start(sam_addr, None).await;
        let mut proxy = TestDenimProxy::start(sam_addr, proxy_addr).await;
        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let client = client_with_proxy(proxy_addr, sam_addr, "alice", "alice device").await;
        sleep(Duration::from_millis(300)).await;
        assert!(client.is_connected().await)
    })
    .await
    .expect("Test took to long to complete")
}

#[tokio::test]
async fn can_send_message() {
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8082";
        let proxy_addr = "127.0.0.1:8083";
        let mut server = TestSamServer::start(sam_addr, None).await;
        let mut proxy = TestDenimProxy::start(sam_addr, proxy_addr).await;
        server
            .started_rx()
            .await
            .expect("Should be able to start server");
        proxy
            .started_rx()
            .await
            .expect("Should be able to start server");

        let mut alice = client_with_proxy(proxy_addr, sam_addr, "alice", "alice device").await;
        let mut bob = client_with_proxy(proxy_addr, sam_addr, "bob", "bob device").await;

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
