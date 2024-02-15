use axum::{routing::get, Router};
use log::info;
use proxy::PendingSocketConnections;

mod control;
mod proxy;
mod socket;

pub async fn start_local_server(cf_listen_address: &str, local_listen_address: &str) {
    // Create shared states.
    // We can simply leak these values to do not pay for reference counting because we need them for the rest of the program.
    let pending_sockets: &'static PendingSocketConnections =
        Box::leak(Box::new(PendingSocketConnections::default()));

    // Build our application with a route
    let app = Router::new()
        .route("/control", get(control::ws_handler))
        .route("/connect", get(proxy::ws_handler))
        .with_state(pending_sockets);

    // Run our app with hyper on another task
    info!("Cloudflare listen is {cf_listen_address}");
    let listener = tokio::net::TcpListener::bind(cf_listen_address)
        .await
        .expect("cannot bind the Axum socket");
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    // In main thread, wait for TCP sockets
    info!("Local listen is {local_listen_address}");
    socket::handle_socket(&local_listen_address, pending_sockets).await;
}
