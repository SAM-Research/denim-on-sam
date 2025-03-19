mod error;

use std::net::SocketAddr;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use error::ServerError;

use sam_client::net::protocol::websocket::WebSocketClientConfig;

#[derive(Clone)]
struct DenimState {
    sam_addr: String,
}

impl DenimState {
    pub fn new(sam_addr: String) -> Self {
        Self { sam_addr }
    }
}

async fn websocket_endpoint(
    State(mut state): State<DenimState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ServerError> {
    let x = WebSocketClientConfig::builder();
    Err::<String, ServerError>(ServerError::SAMUnAuth)
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8081"
        .parse()
        .expect("Unable to parse socket address");

    let state = DenimState::new("127.0.0.1:8080".to_string());
    let app = Router::new()
        .route("/api/v2/websocket", get(websocket_endpoint))
        .with_state(state);

    axum_server::bind(addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
