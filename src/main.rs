use axum::{routing::get, Router};
use log::info;
use proxy::PendingSocketConnections;

mod control;
mod proxy;
mod socket;

#[tokio::main]
async fn main() {
    // Initialize tracing and log
    tracing_subscriber::fmt::init();
    env_logger::init();

    // Create shared states
    let pending_sockets = PendingSocketConnections::default();

    // Build our application with a route
    let app = Router::new()
        .route("/control", get(control::ws_handler))
        .route("/connect", get(control::ws_handler))
        .with_state(pending_sockets.clone());

    // Run our app with hyper on another task
    let cf_listen_address = std::env::var("CF_LISTEN_ADDRESS").unwrap_or("0.0.0.0:2095".to_owned());
    info!("Cloudflare listen is {cf_listen_address}");
    let listener = tokio::net::TcpListener::bind(cf_listen_address).await.expect("cannot bind the TCP socket");
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    // In main thread, wait for TCP sockets
    let local_listen_address = std::env::var("LOCAL_LISTEN_ADDRESS").unwrap_or("127.0.0.1:3000".to_owned());
    info!("Local listen is {local_listen_address}");
    socket::handle_socket(&local_listen_address, pending_sockets).await;
}
