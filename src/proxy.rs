use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use uuid::Uuid;

pub type PendingSocketConnections = Arc<Mutex<HashMap<Uuid, ConnectionPipe>>>;

pub struct ConnectionPipe {
    /// Websocket sends into this pipe in order to send data in the socket
    pub websocket_data: mpsc::Sender<Vec<u8>>,
    /// Websocket await this pipe to get the data from the opened socket
    pub socket_data: mpsc::Receiver<Vec<u8>>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket))
}

async fn handle_socket(mut socket: WebSocket) {

}