use denim_sam_client::client::DenimClientType;
use denim_sam_client::DenimClient;
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_proxy::state::DenimStateType;
use rstest::rstest;
use sam_client::encryption::DecryptedEnvelope;
use sam_server::StateType;
use sam_test_utils::get_next_port;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::time::{sleep, timeout};
use utils::server::TestServerConfigs;
use uuid::Uuid;
mod utils;
use crate::utils::server::{connection_str, postgres_configs};
use utils::client::client_with_proxy;
use utils::server::TestServerConfig as _;

const TIMEOUT_SECS: u64 = 20;

fn large_message() -> &'static str {
    r#"Here is the secret pasta recipe:
    Cook the spaghetti in salted boiling water until al dente. Drain and set aside.
    In the same pot, melt the butter over medium heat.
    Add minced garlic and sauté for 1–2 minutes until fragrant (don't brown it).
    Add the cooked pasta back to the pot and toss to coat.
    Season with salt and pepper. Serve warm with parsley and Parmesan if desired."#
}

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

    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();

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

    let recipe = large_message();

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

    let mut dorothy = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "dorothy device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();
    let dorothy_id = dorothy.account_id();

    let _alice_deniable_messages = alice.deniable_subscribe();
    let _alice_regular = alice.regular_subscribe();
    let _bob_deniable_messages = bob.deniable_subscribe();
    let _bob_regular = bob.regular_subscribe();
    let mut charlie_deniable_messages = charlie.deniable_subscribe();
    let _charlie_regular = charlie.regular_subscribe();
    let _dorothy_regular = dorothy.regular_subscribe();
    let _dorothy_deniable_messages = dorothy.deniable_subscribe();

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

    dorothy
        .send_message(alice_id, "Hi Alice.")
        .await
        .expect("dorothy can send a message to publish her key seeds");

    let dorothy_secret = "Here is a secret";

    dorothy
        .enqueue_message(charlie_id, dorothy_secret)
        .await
        .expect("dorothy can enqueue a deniable message");

    dorothy
        .send_message(bob_id, "Hi Bob!")
        .await
        .expect("Dorothy can send message to alice");

    bob.send_message(dorothy_id, "Hi Dorothy")
        .await
        .expect("Bob can send message to Dorothy");

    dorothy
        .process_messages_blocking()
        .await
        .expect("Dorothy can get message from Bob");

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can process messages");

    let secret_message = "Shh, this is very secret.";

    alice
        .enqueue_message(charlie_id, secret_message)
        .await
        .expect("Can enqueue deniable message");

    let recipe = large_message();
    alice
        .send_message(bob_id, recipe)
        .await
        .expect("Can send pasta recipe");

    dorothy
        .send_message(bob_id, recipe)
        .await
        .expect("Dorothy can send the recipe");

    sleep(Duration::from_secs(1)).await;

    bob.send_message(
        alice_id,
        format!("HA HA. What a funny little recipe: {recipe}"),
    )
    .await
    .expect("Can make fun of Alice's recipe");

    bob.send_message(dorothy_id, format!("I just got this from Alice: {recipe}"))
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

    let envelope = charlie_deniable_messages
        .recv()
        .await
        .expect("can receive message");

    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, dorothy_secret)
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

    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();

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
    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();

    let recipe = large_message();
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

/*
TODO: UNCOMMENT when seeding have been fixed
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

    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();

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
}*/

#[rstest]
#[ignore = "requires a postgres test database"]
#[case(postgres_configs(get_next_port(), get_next_port(), None, connection_str()))]
#[timeout(Duration::from_secs(TIMEOUT_SECS))]
#[tokio::test]
async fn ongoing_communication(
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

    let mut dorothy = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "dorothy device",
        None,
        InMemorySendingBuffer::new(0.5).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut alice_deniable_messages = alice.deniable_subscribe();
    let _alice_regular = alice.regular_subscribe();
    let _bob_deniable_messages = bob.deniable_subscribe();
    let _bob_regular = bob.regular_subscribe();
    let mut charlie_deniable_messages = charlie.deniable_subscribe();
    let _charlie_regular = charlie.regular_subscribe();
    let _dorothy_deniable_messages = dorothy.deniable_subscribe();
    let _dorothy_regular = dorothy.regular_subscribe();

    communicate_deniable(&mut alice, &mut bob, &mut charlie, &mut dorothy).await;

    let secret_message = "Shh, this is very secret.";

    // Alice sends deniable message to charlie
    alice
        .enqueue_message(charlie.account_id(), secret_message)
        .await
        .expect("Can enqueue deniable message");

    communicate_deniable(&mut alice, &mut bob, &mut charlie, &mut dorothy).await;

    sleep(Duration::from_secs(1)).await;

    communicate_deniable(&mut alice, &mut bob, &mut charlie, &mut dorothy).await;

    communicate_deniable(&mut alice, &mut bob, &mut charlie, &mut dorothy).await;

    let envelope = charlie_deniable_messages
        .recv()
        .await
        .expect("can receive message");

    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, secret_message);

    charlie
        .enqueue_message(alice.account_id(), secret_message)
        .await
        .expect("Charlie can enqueue_message");

    let envelope;
    loop {
        communicate_deniable(&mut alice, &mut bob, &mut charlie, &mut dorothy).await;
        if let Ok(result) = timeout(Duration::from_millis(50), alice_deniable_messages.recv()).await
        {
            envelope = result.expect("Can get deniable message from Charlie");
            break;
        }
    }
    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, secret_message);

    for _ in 0..10 {
        alice_and_charlie_deniable_conversation(
            &mut alice,
            &mut bob,
            &mut charlie,
            &mut dorothy,
            &mut alice_deniable_messages,
            &mut charlie_deniable_messages,
        )
        .await;
    }
}

async fn alice_and_charlie_deniable_conversation(
    alice: &mut DenimClient<impl DenimClientType>,
    bob: &mut DenimClient<impl DenimClientType>,
    charlie: &mut DenimClient<impl DenimClientType>,
    dorothy: &mut DenimClient<impl DenimClientType>,
    alice_deniable_messages: &mut Receiver<DecryptedEnvelope>,
    charlie_deniable_messages: &mut Receiver<DecryptedEnvelope>,
) {
    let secret_message = "Very secret";

    alice
        .enqueue_message(charlie.account_id(), secret_message)
        .await
        .expect("Charlie can enqueue_message");
    let envelope;
    loop {
        communicate_deniable(alice, bob, charlie, dorothy).await;
        if let Ok(result) =
            timeout(Duration::from_millis(50), charlie_deniable_messages.recv()).await
        {
            envelope = result.expect("Can get deniable message from Charlie");
            break;
        }
    }
    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, secret_message);

    charlie
        .enqueue_message(alice.account_id(), secret_message)
        .await
        .expect("Charlie can enqueue_message");
    let envelope;
    loop {
        communicate_deniable(alice, bob, charlie, dorothy).await;
        if let Ok(result) = timeout(Duration::from_millis(50), alice_deniable_messages.recv()).await
        {
            envelope = result.expect("Can get deniable message from Charlie");
            break;
        }
    }
    let received_message = String::from_utf8_lossy(envelope.content_bytes()).to_string();

    assert_eq!(received_message, secret_message);
}

// Alice talks to Bob, Bob talks to Alice
// Charlie talks to Dorothy, Dorothy talks to Charlie.
async fn communicate_deniable(
    alice: &mut DenimClient<impl DenimClientType>,
    bob: &mut DenimClient<impl DenimClientType>,
    charlie: &mut DenimClient<impl DenimClientType>,
    dorothy: &mut DenimClient<impl DenimClientType>,
) {
    let message = large_message().to_string();
    alice
        .send_message(bob.account_id(), message.clone())
        .await
        .expect("Alice can send message to bob");

    bob.process_messages_blocking()
        .await
        .expect("Bob can receive a message from Alice");

    bob.send_message(alice.account_id(), message.clone())
        .await
        .expect("Bob can send message to Alice");

    alice
        .process_messages_blocking()
        .await
        .expect("Alice can receive message from Bob");

    charlie
        .send_message(dorothy.account_id(), message.clone())
        .await
        .expect("Charlie can send message to Dorothy");

    dorothy
        .process_messages_blocking()
        .await
        .expect("Dorothy can receive message from Charlie");

    dorothy
        .send_message(charlie.account_id(), message)
        .await
        .expect("Dorothy can send message to Charlie");

    charlie
        .process_messages_blocking()
        .await
        .expect("Charlie can receive message from Dorothy");
}
