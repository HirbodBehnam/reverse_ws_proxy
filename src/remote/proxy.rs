use futures::SinkExt;
use log::{debug, info, warn};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// How many packets can be queued in the socket queue
const SOCKET_QUEUE_LENGTH: usize = 32;

/// Handles a new connection request.
/// At first, creates a websocket connection 
pub(crate) async fn handle_new_connection_request(connection_id: Uuid, cloudflare_server_address: &str, forward_address: &str) {
    debug!("Accepted connection {connection_id}");
    // At first create the websocket
    let websocket = connect_async(format!("{}/connect", cloudflare_server_address))
        .await;
    if let Err(err) = websocket {
        warn!("cannot connect to /connect websocket {connection_id}: {:?}", err);
        return;
    }
    let (mut websocket, _) = websocket.unwrap();
    // Send the uuid in the socket
    if let Err(err) = websocket.send(Message::Text(connection_id.to_string())).await {
        warn!("cannot send id in websocket {connection_id}: {:?}", err);
        return;
    }
    // Now create the TCP socket
    let tcp_socket = TcpStream::connect(forward_address).await;
    if let Err(err) = tcp_socket {
        warn!("cannot connect to TCP socket of connection {connection_id}: {:?}", err);
        return;
    }
    let mut tcp_socket = tcp_socket.unwrap();
    // Create the pipes in order to proxy the data
    let (socket_sender, socket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
    let (websocket_sender, websocket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
    // TODO:
}