use denim_sam_client::client::DenimClientType;
use denim_sam_client::DenimClient;
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use utils::server::TestServerConfigs;
use uuid::Uuid;
mod utils;
use crate::utils::server::{connection_str, postgres_configs};
use utils::client::client_with_proxy;
use utils::server::TestServerConfig as _;

const TIMEOUT_SECS: u64 = 20;

#[rstest]
#[ignore = "requires a postgres test database"]
#[case(postgres_configs(get_next_port(), get_next_port(), None, connection_str()))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn alice_send_to_charlie(
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
        "alice device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "bob device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut charlie = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "charlie device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id().await.expect("Can get alice account_id");
    let bob_id = bob.account_id().await.expect("Can get bob account_id");
    let charlie_id = charlie
        .account_id()
        .await
        .expect("Can get charlie account_id");

    let _alice_deniable_messages = alice.deniable_subscribe();
    let _alice_regular = alice.regular_subscribe();
    let _bob_deniable_messages = bob.deniable_subscribe();
    let _bob_regular = bob.regular_subscribe();
    let mut charlie_deniable_messages = charlie.deniable_subscribe();
    let _charlie_regular = charlie.regular_subscribe();

    // Alice sends a message to publish her key seeds.
    alice
        .send_message(bob_id, "Hello, mr Bob.")
        .await
        .expect("Alice can send message to Bob");

    bob.process_messages_blocking()
        .await
        .expect("Bob can process messages");

    bob.send_message(alice_id, "Hello, ms Alice.")
        .await
        .expect("Bob can send message to Alice");

    // Charlie sends a message to publish his key seeds.
    charlie
        .send_message(bob_id, "Hello my very good friend")
        .await
        .expect("Charlie can greet bob");

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can process messages");

    let secret_message = "Shh, this is very secret.";

    // Alice sends deniable message to charlie
    alice
        .enqueue_message(charlie_id, secret_message)
        .await
        .expect("Can enqueue deniable message");

    let recipe = r#"Here is the secret pasta recipe:
    Cook the spaghetti in salted boiling water until al dente. Drain and set aside.
    In the same pot, melt the butter over medium heat.
    Add minced garlic and sauté for 1–2 minutes until fragrant (don't brown it).
    Add the cooked pasta back to the pot and toss to coat.
    Season with salt and pepper. Serve warm with parsley and Parmesan if desired."#;

    // Alice sends a large message to bob to ensure that the deniable message is piggy backed.
    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Can send pasta recipe");

    sleep(Duration::from_secs(1)).await;

    bob.send_message(
        alice_id,
        format!("HA HA. What a funny little recipe: {recipe}"),
    )
    .await
    .expect("Can make fun of Alice's recipe");

    alice
        .process_messages_blocking()
        .await
        .expect("Can receive message from bob making fun of recipe");

    sleep(Duration::from_secs(1)).await;

    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Alice can send the recipe again");

    bob.send_message(
        charlie_id,
        format!("Alice just sent a weird recipe: {recipe}"),
    )
    .await
    .expect("Can send message from bob to charlie");

    charlie
        .process_messages_blocking()
        .await
        .expect("Charlie can process messages");

    let envelope = charlie_deniable_messages
        .recv()
        .await
        .expect("can receive message");

    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, secret_message)
}

#[rstest]
#[ignore = "requires a postgres test database"]
#[case(postgres_configs(get_next_port(), get_next_port(), None, connection_str()))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn alice_cannot_send_to_charlie_if_blocked(
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
        "alice device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "bob device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut charlie = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "charlie device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id().await.expect("Can get alice account_id");
    let bob_id = bob.account_id().await.expect("Can get bob account_id");
    let charlie_id = charlie
        .account_id()
        .await
        .expect("Can get charlie account_id");

    let _alice_deniable_messages = alice.deniable_subscribe();
    let _alice_regular = alice.regular_subscribe();
    let _bob_deniable_messages = bob.deniable_subscribe();
    let _bob_regular = bob.regular_subscribe();
    let mut charlie_deniable_messages = charlie.deniable_subscribe();
    let _charlie_regular = charlie.regular_subscribe();

    alice
        .send_message(bob_id, "Hello, mr Bob.")
        .await
        .expect("Alice can send message to Bob");

    bob.process_messages_blocking()
        .await
        .expect("Bob can process messages");

    bob.send_message(alice_id, "Hello, ms Alice.")
        .await
        .expect("Bob can send message to Alice");

    charlie.block_user(alice_id).await;

    charlie
        .send_message(bob_id, "Hello my very good friend")
        .await
        .expect("Charlie can greet bob");

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can process messages");

    let secret_message = "Shh, this is very secret.";

    alice
        .enqueue_message(charlie_id, secret_message)
        .await
        .expect("Can enqueue deniable message");

    let recipe = r#"Here is the secret pasta recipe:
    Cook the spaghetti in salted boiling water until al dente. Drain and set aside.
    In the same pot, melt the butter over medium heat.
    Add minced garlic and sauté for 1–2 minutes until fragrant (don't brown it).
    Add the cooked pasta back to the pot and toss to coat.
    Season with salt and pepper. Serve warm with parsley and Parmesan if desired."#;

    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Can send pasta recipe");

    sleep(Duration::from_secs(1)).await;

    bob.send_message(
        alice_id,
        format!("HA HA. What a funny little recipe: {recipe}"),
    )
    .await
    .expect("Can make fun of Alice's recipe");

    alice
        .process_messages_blocking()
        .await
        .expect("Can receive message from bob making fun of recipe");

    sleep(Duration::from_secs(1)).await;

    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Alice can send the recipe again");

    bob.send_message(
        charlie_id,
        format!("Alice just sent a weird recipe: {recipe}"),
    )
    .await
    .expect("Can send message from bob to charlie");

    charlie
        .process_messages_blocking()
        .await
        .expect("Charlie can process messages blocking because there is a regular message for him");

    let result = timeout(Duration::from_secs(2), charlie_deniable_messages.recv()).await;

    assert!(result.is_err());
}

#[rstest]
#[ignore = "requires a postgres test database"]
#[case(postgres_configs(get_next_port(), get_next_port(), None, connection_str()))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn key_request_waits_for_seed_update(
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
        "alice device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "bob device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut charlie = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "charlie device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id().await.expect("Can get alice account_id");
    let bob_id = bob.account_id().await.expect("Can get bob account_id");
    let charlie_id = charlie
        .account_id()
        .await
        .expect("Can get charlie account_id");

    let _alice_deniable_messages = alice.deniable_subscribe();
    let _alice_regular = alice.regular_subscribe();
    let _bob_deniable_messages = bob.deniable_subscribe();
    let _bob_regular = bob.regular_subscribe();
    let mut charlie_deniable_messages = charlie.deniable_subscribe();
    let _charlie_regular = charlie.regular_subscribe();

    alice
        .send_message(bob_id, "Hello, mr Bob.")
        .await
        .expect("Alice can send message to Bob");

    bob.process_messages_blocking()
        .await
        .expect("Bob can process messages");

    bob.send_message(alice_id, "Hello, ms Alice.")
        .await
        .expect("Bob can send message to Alice");

    // Charlie does not send a message and so his seeds are not uploaded yet.
    /*
    charlie
        .send_message(bob_id, "Hello my very good friend")
        .await
        .expect("Charlie can greet bob");
    */

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can process messages");

    let secret_message = "Shh, this is very secret.";

    alice
        .enqueue_message(charlie_id, secret_message)
        .await
        .expect("Can enqueue deniable message");

    conversation(&mut alice, &mut bob, &mut charlie).await;

    let result = timeout(Duration::from_secs(2), charlie_deniable_messages.recv()).await;

    assert!(result.is_err());

    // Charlie now sends a message, uploading his key seeds.
    charlie
        .send_message(bob_id, "Hello my very good friend")
        .await
        .expect("Charlie can greet bob");

    conversation(&mut alice, &mut bob, &mut charlie).await;

    let result = timeout(Duration::from_secs(2), charlie_deniable_messages.recv()).await;

    assert!(result.is_ok());
}

async fn conversation(
    alice: &mut DenimClient<impl DenimClientType>,
    bob: &mut DenimClient<impl DenimClientType>,
    charlie: &mut DenimClient<impl DenimClientType>,
) {
    let alice_id = alice.account_id().await.expect("Can get alice account_id");
    let bob_id = bob.account_id().await.expect("Can get bob account_id");
    let charlie_id = charlie
        .account_id()
        .await
        .expect("Can get charlie account_id");

    let recipe = r#"Here is the secret pasta recipe:
    Cook the spaghetti in salted boiling water until al dente. Drain and set aside.
    In the same pot, melt the butter over medium heat.
    Add minced garlic and sauté for 1–2 minutes until fragrant (don't brown it).
    Add the cooked pasta back to the pot and toss to coat.
    Season with salt and pepper. Serve warm with parsley and Parmesan if desired."#;

    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Can send pasta recipe");

    sleep(Duration::from_secs(1)).await;

    bob.send_message(
        alice_id,
        format!("HA HA. What a funny little recipe: {recipe}"),
    )
    .await
    .expect("Can make fun of Alice's recipe");

    alice
        .process_messages_blocking()
        .await
        .expect("Can receive message from bob making fun of recipe");

    sleep(Duration::from_secs(1)).await;

    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Alice can send the recipe again");

    bob.send_message(
        charlie_id,
        format!("Alice just sent a weird recipe: {recipe}"),
    )
    .await
    .expect("Can send message from bob to charlie");

    charlie
        .process_messages_blocking()
        .await
        .expect("Charlie can process messages");
}

#[rstest]
#[ignore = "requires a postgres test database"]
#[case(postgres_configs(get_next_port(), get_next_port(), None, connection_str()))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn update_seed(
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
        "alice device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "bob device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut charlie = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "charlie device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id().await.expect("Can get alice account_id");
    let bob_id = bob.account_id().await.expect("Can get bob account_id");
    let charlie_id = charlie
        .account_id()
        .await
        .expect("Can get charlie account_id");

    let _alice_deniable_messages = alice.deniable_subscribe();
    let _alice_regular = alice.regular_subscribe();
    let _bob_deniable_messages = bob.deniable_subscribe();
    let _bob_regular = bob.regular_subscribe();
    let mut charlie_deniable_messages = charlie.deniable_subscribe();
    let _charlie_regular = charlie.regular_subscribe();

    alice
        .send_message(bob_id, "Hello, mr Bob.")
        .await
        .expect("Alice can send message to Bob");

    bob.process_messages_blocking()
        .await
        .expect("Bob can process messages");

    bob.send_message(alice_id, "Hello, ms Alice.")
        .await
        .expect("Bob can send message to Alice");

    charlie
        .send_message(bob_id, "Hello my very good friend")
        .await
        .expect("Charlie can greet bob");

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can process messages");

    let secret_message = "Shh, this is very secret.";

    alice
        .enqueue_message(charlie_id, secret_message)
        .await
        .expect("Can enqueue deniable message");

    conversation(&mut alice, &mut bob, &mut charlie).await;

    let envelope = charlie_deniable_messages
        .recv()
        .await
        .expect("can receive message");

    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    alice.update_key_seed().await.expect("Can update Seed");

    assert_eq!(received_message, secret_message);

    alice
        .send_message(bob_id, "Hello, mr Bob.")
        .await
        .expect("Alice can send message to Bob");

    bob.process_messages_blocking()
        .await
        .expect("Bob can process messages");

    bob.send_message(alice_id, "Hello, ms Alice.")
        .await
        .expect("Bob can send message to Alice");

    charlie
        .send_message(bob_id, "Hello my very good friend")
        .await
        .expect("Charlie can greet bob");

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can process messages");

    let secret_message = "Shh, this is very secret.";

    alice
        .enqueue_message(charlie_id, secret_message)
        .await
        .expect("Can enqueue deniable message");

    conversation(&mut alice, &mut bob, &mut charlie).await;

    let envelope = charlie_deniable_messages
        .recv()
        .await
        .expect("can receive message");

    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, secret_message)
}
