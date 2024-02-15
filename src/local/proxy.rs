use std::collections::HashMap;
use std::str::FromStr;

use axum::extract::State;
use futures::{SinkExt, StreamExt};
use log::{debug, info, warn};
use parking_lot::Mutex;
use tokio::sync::mpsc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use uuid::Uuid;

pub type PendingSocketConnections = Mutex<HashMap<Uuid, ConnectionPipe>>;

/// ConnectionPipe is used to connect a socket to a websocket.
pub struct ConnectionPipe {
    /// Websocket sends into this pipe in order to send data in the socket
    pub websocket_data: mpsc::Sender<Vec<u8>>,
    /// Websocket await this pipe to get the data from the opened socket
    pub socket_data: mpsc::Receiver<Vec<u8>>,
}

/// Entry point of websockets which are coming to proxy the data between a remote peer and a local peer.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(pending_connections): State<&'static PendingSocketConnections>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, pending_connections))
}

async fn handle_socket(mut socket: WebSocket, pending_connections: &PendingSocketConnections) {
    // The first packet must be the UUID of the connection
    let (mut connection_pipe, socket_id) = match socket.recv().await {
        Some(Ok(Message::Text(uuid))) => match uuid::Uuid::from_str(&uuid.trim()) {
            Ok(uuid) => match pending_connections.lock().remove(&uuid) {
                Some(pipe) => (pipe, uuid),
                None => {
                    warn!("UUID {uuid} does not exists");
                    return;
                }
            },
            Err(err) => {
                warn!("Cannot parse UUID of websocket {uuid}: {err}");
                return;
            }
        },
        _ => return, // socket closed?
    };
    debug!("Websocket of connection {socket_id} joined");
    // Now we simply proxy the data
    let (mut sender, mut receiver) = socket.split();
    // Create another task for watch for the incoming data from the websocket.
    // In that case, we can catch the errors. Note that I could have possibly just put it in the
    // select loop but I think this is quite nicer because the data will be continuously pulled.
    // Plus, I don't now if receiver.next() is cancel safe or not.
    let mut recv_packet = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(payload) => {
                    connection_pipe.websocket_data.send(payload).await.unwrap();
                }
                Message::Close(close_code) => {
                    info!("Websocket {socket_id} closed with {:?}", close_code);
                    return;
                }
                _ => {} // do nothing and poll again
            }
        }
    });
    // In a loop, wait for events
    loop {
        tokio::select! {
            // If the recv_packet is done, we can simply bail
            _ = (&mut recv_packet) => return,
            // But also check for data to send
            data = connection_pipe.socket_data.recv() => {
                match data {
                    Some(data) => sender.send(Message::Binary(data)).await.unwrap(), // TODO: better error handling
                    None => { // connection closed
                        recv_packet.abort();
                        return;
                    },
                }
            }
        }
    }
}
