use clap::{Arg, Command};
use server::{start_proxy, DenimConfig};
use state::DenimState;

mod error;
mod proxy;
mod routes;
mod server;
mod state;
mod utils;

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = Command::new("sam_server")
        .arg(
            Arg::new("sam_ip")
                .long("sam-ip")
                .required(false)
                .help("IP to run sam server on")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("proxy_ip")
                .long("proxy-ip")
                .required(false)
                .help("IP to run proxy on")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("sam_port")
                .long("sam-port")
                .required(false)
                .help("Port to run sam on")
                .default_value("8080"),
        )
        .arg(
            Arg::new("proxy_port")
                .long("proxy-port")
                .required(false)
                .help("Port to run sam on")
                .default_value("8081"),
        )
        .get_matches();

    let ip = matches.get_one::<String>("proxy_ip").unwrap();
    let port = matches.get_one::<String>("proxy_port").unwrap();
    let sam_ip = matches.get_one::<String>("sam_ip").unwrap();
    let sam_port = matches.get_one::<String>("sam_port").unwrap();

    let addr = format!("{}:{}", ip, port)
        .parse()
        .expect("Unable to parse socket address");

    let config = DenimConfig {
        state: DenimState::new(format!("ws://{}:{}", sam_ip, sam_port), 10),
        addr: addr,
    };

    start_proxy(config).await.unwrap()
}
