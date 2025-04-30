use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use utils::client::client_with_proxy;
use utils::server::in_memory_configs;
use utils::server::TestServerConfig as _;
use utils::server::TestServerConfigs;
use uuid::Uuid;

mod utils;

const TIMEOUT_SECS: u64 = 120;

#[rstest]
#[case(in_memory_configs(get_next_port(), get_next_port(), None))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
pub async fn alice_can_upload_keys(
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
        InMemorySendingBuffer::new(0.5).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    assert!(alice.delete_account().await.is_ok())
}
