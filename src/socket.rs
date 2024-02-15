use log::{debug, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
};
use uuid::Uuid;

use crate::{
    control,
    proxy::{self, ConnectionPipe},
};

/// How many packets can be queued in the socket queue
const SOCKET_QUEUE_LENGTH: usize = 32;
/// How big is our read buffer size
const READ_BUFFER_SIZE: usize = 32 * 1024;

/// This function will handle the socket listening and controlling the controller
/// to open new connections and such
pub async fn handle_socket(listen: &str, pending_packets: &proxy::PendingSocketConnections) {
    // Create the socket and listen
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .expect("cannot bind the TCP socket");
    loop {
        let (socket, socket_address) = listener.accept().await.expect("cannot accept connections");
        debug!("Accepted connection: {socket_address}");
        // For each socket, create a new UUID
        let socket_id = Uuid::new_v4();
        // Create the pipes and add the request in the pending sockets
        let (socket_sender, socket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
        let (websocket_sender, websocket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
        let connection_pipe = ConnectionPipe {
            websocket_data: websocket_sender,
            socket_data: socket_receiver,
        };
        pending_packets.lock().insert(socket_id, connection_pipe);
        // Send the request to the server before Cloudflare
        let control_channel = control::CONTROLLER_COMMANDER
            .lock()
            .as_ref()
            .map(|c| c.clone());
        match control_channel {
            Some(channel) => channel
                .send(control::ControllerCommand::NewConnection(socket_id))
                .await
                .unwrap(),
            None => {
                // the server's channel is not established yet
                warn!("Control websocket not established yet...");
                pending_packets.lock().remove(&socket_id);
                continue;
            }
        };
        // Wait for acceptance
        tokio::task::spawn(handle_opened_socket(
            socket,
            socket_sender,
            websocket_receiver,
        ));
    }
}

async fn handle_opened_socket(
    socket: TcpStream,
    socket_sender: Sender<Vec<u8>>,
    mut websocket_receiver: Receiver<Vec<u8>>,
) {
    // We dont need to wait for the websocket, just send the data in the pipes and hope for the best.
    let (mut socket_r, mut socket_w) = socket.into_split();
    // First spawn a task that only reads the data from the socket
    let mut socket_reader_task = tokio::task::spawn(async move {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        while let Ok(n) = socket_r.read(&mut read_buffer).await {
            socket_sender
                .send(read_buffer[..n].to_owned())
                .await
                .unwrap();
        }
    });
    // Now in a loop, wait for either a received packet from websocket or reader finishing
    loop {
        tokio::select! {
            // If the socket_reader_task is done, we can simply bail
            _ = (&mut socket_reader_task) => return,
            // But also check for commands
            data = websocket_receiver.recv() => {
                match data {
                    Some(data) => { // if there is data, write it into the pipe
                        socket_w.write(&data).await.unwrap();
                    }
                    None => { // websocket closed
                        socket_reader_task.abort();
                        return; // socket_w will be dropped and connection will be closed
                    }
                }
            }
        }
    }
}
