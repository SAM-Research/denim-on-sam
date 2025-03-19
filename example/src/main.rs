use clap::{Arg, Command};
use denim_sam_proxy::{
    server::{start_proxy, DenimConfig},
    state::DenimState,
};
use sam_server::{start_server, ServerConfig, ServerState};

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = Command::new("sam_server")
        .arg(
            Arg::new("ip")
                .short('i')
                .long("ip")
                .required(false)
                .help("IP to run example on")
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

    let ip = matches.get_one::<String>("ip").unwrap();
    let port = matches.get_one::<String>("proxy_port").unwrap();
    let sam_port = matches.get_one::<String>("sam_port").unwrap();

    let proxy_addr = format!("{}:{}", ip, port)
        .parse()
        .expect("Unable to parse socket address");
    let server_addr = format!("{}:{}", ip, sam_port)
        .parse()
        .expect("Unable to parse socket address");

    let denim_config = DenimConfig {
        state: DenimState::new(format!("ws://{}:{}", ip, sam_port), 10),
        addr: proxy_addr,
    };

    let config = ServerConfig {
        state: ServerState::in_memory("denimproxy".to_string(), 600, 10),
        addr: server_addr,
        tls_config: None,
    };

    tokio::spawn(start_server(config));
    start_proxy(denim_config).await.unwrap();
}
