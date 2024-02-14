use axum::{routing::get, Router};
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
    let listen_address = std::env::var("LISTEN_ADDRESS").unwrap_or("0.0.0.0:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    // In main thread, wait for TCP sockets
    // TODO: socket
}
