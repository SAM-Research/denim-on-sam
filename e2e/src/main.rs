use std::time::Duration;

use denim_sam_client::{client::DenimClientType, DenimClient};
use denim_sam_common::{buffers::{DeniablePayload, DenimMessage, InMemoryReceivingBuffer, InMemorySendingBuffer}, denim_message::DenimEnvelope};
use denim_sam_e2e::utils::{
    client::client_with_proxy,
    server::{connection_str, postgres_configs, TestServerConfig},
    tls::{client_config, tls_configs},
};
use env_logger::fmt::Formatter;
use log::info;
use sam_client::encryption::DecryptedEnvelope;
use sam_common::{time_now_millis, AccountId};
use std::io::Write;
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;
use prost::Message;

#[tokio::main]
async fn main() {
    // wireshark filter: tcp.port == 8443  || tcp.port == 9443
    let millis = 5000;
    env_logger::builder()
    .format(|buf: &mut Formatter, record: &log::Record| {
        let now = time_now_millis();
        
        writeln!(buf, "{} |{}| {}", now, record.target(), record.args())
    })
        .parse_filters(
            "denim_sam_e2e=info,denim_sam_client=info,denim_sam_proxy::denim_routes=debug,denim_sam_client::client=info,denim_sam_proxy::proxy=debug",
        )
        .init();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = postgres_configs(8443, 9443, None, connection_str()).await;
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
    tokio::time::sleep(Duration::from_millis(millis)).await;
    let mut alice = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "alice device",
        None,
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut bob = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "bob device",
        None,
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let mut charlie = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "charlie device",
        None,
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;
    let mut dorothy = client_with_proxy(
        proxy.address(),
        server.address(),
        &Uuid::new_v4().to_string(),
        "dorothy device",
        None,
        InMemorySendingBuffer::new(0.0).expect("Can make sending buffer"),
        InMemoryReceivingBuffer::default(),
    )
    .await;

    let alice_id = alice.account_id();
    let bob_id = bob.account_id();
    let charlie_id = charlie.account_id();
    let dorothy_id = dorothy.account_id();

    let mut a_denim_rx = alice.deniable_subscribe();
    let mut a_sam_rx = alice.regular_subscribe();

    let mut b_sam_rx = bob.regular_subscribe();

    let mut c_denim_rx = charlie.deniable_subscribe();
    let mut c_sam_rx = charlie.regular_subscribe();

    let mut d_denim_rx = dorothy.deniable_subscribe();
    let mut d_sam_rx = dorothy.regular_subscribe();

    let a_msg = [8u8; 400];
    let b_msg = [16u8; 450];
    let c_msg = [32u8; 500];
    let d_msg = [64u8; 550];
    let denim_msg = [128u8; 200];

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
    info!("Dorothy {dorothy_id}");
    info!("Dorothy msg {}", d_msg.len());
    info!("---------");
    info!("Denim msg {}", denim_msg.len());
    info!("---------");

    // key uploads
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n-------------------- SEED UPDATES ------------------");
    send_recv(&mut alice, &mut bob, &mut b_sam_rx, a_msg).await;
    send_recv(&mut bob, &mut alice, &mut a_sam_rx, b_msg).await;
    send_recv(&mut charlie, &mut dorothy, &mut d_sam_rx, c_msg).await;
    send_recv(&mut dorothy, &mut charlie, &mut c_sam_rx, d_msg).await;

    // key request + inital deniable message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n------------- ALICE KEY REQ + ENQUEUE DENIM --------");
    alice
        .enqueue_message(dorothy_id, denim_msg)
        .await
        .expect("can enqueue");
    send_recv(&mut alice, &mut bob, &mut b_sam_rx, a_msg).await;

    // key response
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n--------------- KEY RESPONSE ----------------------");
    send_recv(&mut bob, &mut alice, &mut a_sam_rx, b_msg).await;

    // piggy back denim message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n--------------- SEND DENIM ------------------------");
    send_recv(&mut alice, &mut bob, &mut b_sam_rx, a_msg).await;

    // dorothy receives denim message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n---------------- RECEIVE DENIM --------------------");
    send_recv(&mut charlie, &mut dorothy, &mut d_sam_rx, c_msg).await;

    // dorothy reads alice denim message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    let env = d_denim_rx.recv().await.expect("can recv");
    log_recv(dorothy_id, env, true);

    // piggy back denim message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n--------------- SEND DENIM ------------------------");
    dorothy
        .enqueue_message(alice_id, denim_msg)
        .await
        .expect("can enqueue");
    send_recv(&mut dorothy, &mut charlie, &mut c_sam_rx, d_msg).await;

    // alice receives dorothy denim message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    info!("\n\n---------------- RECEIVE DENIM --------------------");
    send_recv(&mut bob, &mut alice, &mut a_sam_rx, b_msg).await;

    // alice reads dorothy denim message
    tokio::time::sleep(Duration::from_millis(millis)).await;
    let env = a_denim_rx.recv().await.expect("can recv");
    log_recv(alice_id, env, true);

    info!("\n\n--------------------------alie message to bob without denim-----");
    tokio::time::sleep(Duration::from_millis(millis)).await;
    send_recv(&mut alice, &mut bob, &mut b_sam_rx, a_msg).await;

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
    let x = if denim { "DENIM " } else { "" };
    info!("{me} <-({len})- {sender} {x}");
}
