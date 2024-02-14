use log::{debug, warn};
use tokio::{net::TcpStream, sync::mpsc};
use uuid::Uuid;

use crate::{control, proxy::{self, ConnectionPipe}};

/// How many packets can be queued in the socket queue
const SOCKET_QUEUE_LENGTH: usize = 32;

/// This function will handle the socket listening and controlling the controller
/// to open new connections and such
pub async fn handle_socket(listen: &str, pending_packets: proxy::PendingSocketConnections) {
    // Create the socket and listen
    let listener = tokio::net::TcpListener::bind(listen).await.expect("cannot bind the TCP socket");
    loop {
        let (socket, socket_address) = listener.accept().await.expect("cannot accept connections");
        debug!("Accepted connection: {socket_address}");
        // For each socket, create a new UUID
        let socket_id = Uuid::new_v4();
        // Create the pipes and add the request in the pending sockets
        let (socket_sender, socket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
        let (websocket_sender, websocket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
        let connection_pipe = ConnectionPipe{
            websocket_data: websocket_sender,
            socket_data: socket_receiver,
        };
        pending_packets.lock().insert(socket_id, connection_pipe);
        // Send the request to the server before Cloudflare
        let control_channel = control::CONTROLLER_COMMANDER.lock().as_ref().map(|c| c.clone());
        match control_channel {
            Some(channel) => channel.send(control::ControllerCommand::NewConnection(socket_id)).await.unwrap(),
            None => { // the server's channel is not established yet
                warn!("Control websocket not established yet...");
                pending_packets.lock().remove(&socket_id);
                continue;
            },
        };
        // Wait for acceptance
        tokio::task::spawn(handle_opened_socket(socket, socket_id));
    }
}

async fn handle_opened_socket(socket: TcpStream, id: Uuid) {
    // TODO:
}