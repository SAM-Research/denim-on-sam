use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use rustls::ClientConfig;
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use utils::client::client_with_proxy;
use utils::server::in_memory_configs;
use utils::server::TestServerConfig as _;
use utils::server::TestServerConfigs;
use utils::tls::client_config;
use utils::tls::tls_configs;
use uuid::Uuid;

use crate::utils::server::connection_str;
use crate::utils::server::postgres_configs;

mod utils;

const TIMEOUT_SECS: u64 = 20;

#[rstest]
#[case(
    in_memory_configs(get_next_port(), get_next_port(), tls_configs(true)),
    client_config(true)
)]
#[ignore = "requires a postgres test database"]
#[case(
    postgres_configs(get_next_port(), get_next_port(), tls_configs(true), connection_str()),
    client_config(true)
)]
#[case(in_memory_configs(get_next_port(), get_next_port(), None), None)]
#[tokio::test]
async fn can_connect(
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
    #[case] client_tls: Option<ClientConfig>,
) {
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

        let client = client_with_proxy(
            proxy.address(),
            server.address(),
            &Uuid::new_v4().to_string(),
            "alice device",
            client_tls,
            InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
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
#[case(
    in_memory_configs(get_next_port(), get_next_port(), tls_configs(true)),
    client_config(true)
)]
#[case(in_memory_configs(get_next_port(), get_next_port(), None), None)]
#[tokio::test]
async fn can_send_message(
    #[future(awt)]
    #[case]
    server_configs: TestServerConfigs<impl StateType, impl DenimStateType>,
    #[case] client_tls: Option<ClientConfig>,
) {
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

        let mut alice = client_with_proxy(
            proxy.address(),
            server.address(),
            &Uuid::new_v4().to_string(),
            "alice device",
            client_tls.clone(),
            InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;
        let mut bob = client_with_proxy(
            proxy.address(),
            server.address(),
            &Uuid::new_v4().to_string(),
            "bob device",
            client_tls,
            InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
            InMemoryReceivingBuffer::default(),
        )
        .await;

        let alice_id = alice.account_id();
        let bob_id = bob.account_id();

        let expected_msg = "Hello bob through denim proxy!";
        alice
            .send_message(bob_id, expected_msg)
            .await
            .expect("Can send message");

        let mut bob_recv = bob.regular_subscribe();
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
