use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use sam_client::storage::ContactStore;
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use utils::client::client_with_proxy;
use utils::server::in_memory_configs;
use utils::server::TestServerConfig as _;
use utils::server::TestServerConfigs;
use uuid::Uuid;

use crate::utils::server::connection_str;
use crate::utils::server::postgres_configs;

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
        InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    assert!(alice.delete_account().await.is_ok())
}

/*
    Users did not store senders in their deniable contact stores when receiving the first prekey message.
    This lead to receivers of the first denim message to ask for the senders keys, which is redundant.
*/
#[rstest]
#[ignore = "requires a postgres test database"]
#[case(postgres_configs(get_next_port(), get_next_port(), None, connection_str()))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
pub async fn denim_client_stores_sender_on_first_message(
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
    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "Alice's device",
        None,
        InMemorySendingBuffer::new(0.0).expect("can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut b_den_rx = bob.deniable_subscribe();
    let bob_id = bob.account_id();
    let alice_id = alice.account_id();
    let msg = [32u8; 500];
    let dmsg = [16u8; 200];
    assert!(!alice
        .deniable_store()
        .contact_store
        .contains_contact(bob.account_id())
        .await
        .expect("can check contacts"));

    // bob uploads keys
    bob.send_message(alice_id, msg)
        .await
        .expect("can send message");

    alice
        .enqueue_message(bob_id, dmsg)
        .await
        .expect("can enqueue");
    alice
        .send_message(bob_id, msg)
        .await
        .expect("can send message");

    bob.send_message(alice_id, msg)
        .await
        .expect("can send message");

    alice
        .process_messages_blocking()
        .await
        .expect("can process");
    assert!(alice
        .deniable_store()
        .contact_store
        .contains_contact(bob.account_id())
        .await
        .expect("can check contacts"));

    assert!(!bob
        .deniable_store()
        .contact_store
        .contains_contact(alice_id)
        .await
        .expect("can check contacts"));
    alice
        .send_message(bob_id, msg)
        .await
        .expect("can send message");
    bob.process_messages_blocking()
        .await
        .expect("can process messages");
    let env = b_den_rx.recv().await.expect("can receive");
    let rmsg = env.content_bytes().clone();
    assert!(rmsg == dmsg);

    assert!(bob
        .deniable_store()
        .contact_store
        .contains_contact(alice_id)
        .await
        .expect("can check contacts"));
}
