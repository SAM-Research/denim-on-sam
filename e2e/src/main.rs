use denim_sam_client::{client::DenimClientType, DenimClient};
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_e2e::utils::{
    client::client_with_proxy,
    server::{connection_str, postgres_configs, TestServerConfig},
    tls::{client_config, tls_configs},
};
use log::info;
use sam_client::encryption::DecryptedEnvelope;
use sam_common::AccountId;
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // wireshark filter: tcp.port == 8443  || tcp.port == 9443
    env_logger::builder()
        .parse_filters("denim_sam_e2e=info,denim_sam_client=info")
        //.parse_filters("info")
        .init();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = postgres_configs(8443, 9443, tls_configs(true), connection_str()).await;
    let mut server = config.sam.start().await;
    let mut proxy = config.denim.start().await;

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
        client_config(true),
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "bob device",
        client_config(true),
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut charlie = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "charlie device",
        client_config(true),
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();

    let mut a_denim_rx = alice.deniable_subscribe();
    let mut a_sam_rx = alice.regular_subscribe();
    let mut b_denim_rx = bob.deniable_subscribe();
    let mut b_sam_rx = bob.regular_subscribe();
    let mut c_denim_rx = charlie.deniable_subscribe();
    let mut c_sam_rx = charlie.regular_subscribe();

    let a_msg = [8u8; 400];
    let b_msg = [16u8; 450];
    let c_msg = [32u8; 500];
    let denim_msg = [64u8; 200];

    // ##### Expirment #####
    info!("Alice {alice_id}");
    info!("Alice msg {}", a_msg.len());
    info!("---------");
    info!("Bob {bob_id}");
    info!("Bob msg {}", b_msg.len());
    info!("---------");
    info!("Charlie {charlie_id}");
    info!("Charlie msg {}", c_msg.len());
    info!("---------");
    info!("Denim msg {}", denim_msg.len());
    info!("---------");

    // alice enqueues denim key request + message to charlie
    alice
        .enqueue_message(charlie_id, denim_msg)
        .await
        .expect("can send denim");
    // alice upload keys + key request to charlie to proxy
    send_recv(&mut alice, &mut bob, &mut b_sam_rx, a_msg).await;

    // ???
    send_recv(&mut bob, &mut alice, &mut a_sam_rx, b_msg).await;

    // charlie upload keys to proxy
    send_recv(&mut charlie, &mut bob, &mut b_sam_rx, c_msg).await;

    // alice receives charlie keys through bob message
    send_recv(&mut bob, &mut alice, &mut a_sam_rx, b_msg).await;

    // alice's denim message to charlie gets uploaded to server
    send_recv(&mut alice, &mut bob, &mut b_sam_rx, a_msg).await;

    // bob sends message to charlie and alice message gets piggy backed
    send_recv(&mut bob, &mut charlie, &mut c_sam_rx, b_msg).await;

    // charlie reads denim message
    let env = c_denim_rx.recv().await.expect("recv");
    log_recv(charlie_id, env, true);

    // charlie enqueues denim message to alice
    charlie
        .enqueue_message(alice_id, denim_msg)
        .await
        .expect("can send denim");

    // charlie sends bob a message to upload denim message for alice
    send_recv(&mut charlie, &mut bob, &mut b_sam_rx, c_msg).await;
    // bob piggy backs charlie message to alice
    send_recv(&mut bob, &mut alice, &mut a_sam_rx, b_msg).await;

    // alice reads message
    let env = a_denim_rx.recv().await.expect("recv");
    log_recv(alice_id, env, true);
}

async fn send_recv(
    a: &mut DenimClient<impl DenimClientType>,
    b: &mut DenimClient<impl DenimClientType>,
    b_rx: &mut Receiver<DecryptedEnvelope>,
    a_msg: impl Into<Vec<u8>> + Clone,
) {
    let bid = b.account_id();
    a.send_message(bid, a_msg).await.expect("can send message");

    b.process_messages_blocking().await.expect("can process");

    let env = b_rx.recv().await.expect("can recv");
    log_recv(bid, env, false);
}

fn log_recv(me: AccountId, env: DecryptedEnvelope, denim: bool) {
    let sender = env.source_account_id();
    let len = env.content_bytes().len();
    let now = env.timestamp();
    let x = if denim { "DENIM " } else { "" };
    info!("[{now}] {me} <-({len})- {sender} {x}");
}
