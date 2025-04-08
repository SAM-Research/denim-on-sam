use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use std::time::Duration;
use tokio::time::timeout;
use utils::{
    client::client_with_proxy,
    server::{TestDenimProxy, TestSamServer},
};

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[tokio::test]
pub async fn alice_can_upload_keys() {
    timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let sam_addr = "127.0.0.1:8060".to_owned();
        let proxy_addr = "127.0.0.1:8061".to_owned();
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
