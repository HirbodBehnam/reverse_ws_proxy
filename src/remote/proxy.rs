use futures::{SinkExt, StreamExt};
use log::{debug, info, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// How many packets can be queued in the socket queue
const SOCKET_QUEUE_LENGTH: usize = 32;
/// How big is our read buffer size
const READ_BUFFER_SIZE: usize = 32 * 1024;

/// Handles a new connection request.
/// At first, creates a websocket connection
pub(crate) async fn handle_new_connection_request(
    connection_id: Uuid,
    cloudflare_server_address: &str,
    forward_address: &str,
) {
    info!("Accepted connection {connection_id}");
    // At first create the websocket
    let websocket = connect_async(format!("{}/connect", cloudflare_server_address)).await;
    if let Err(err) = websocket {
        warn!(
            "cannot connect to /connect websocket {connection_id}: {:?}",
            err
        );
        return;
    }
    let (mut websocket, _) = websocket.unwrap();
    // Send the uuid in the socket
    if let Err(err) = websocket
        .send(Message::Text(connection_id.to_string()))
        .await
    {
        warn!("cannot send id in websocket {connection_id}: {:?}", err);
        return;
    }
    let (mut websocket_tx, mut websocket_rx) = websocket.split();
    // Now create the TCP socket
    let tcp_socket = TcpStream::connect(forward_address).await;
    if let Err(err) = tcp_socket {
        warn!(
            "cannot connect to TCP socket of connection {connection_id}: {:?}",
            err
        );
        return;
    }
    let (mut tcp_socket_rx, mut tcp_socket_tx) = tcp_socket.unwrap().into_split();
    // Create the pipes in order to proxy the data
    let (socket_sender, mut socket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
    let (websocket_sender, mut websocket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
    // Create four tasks to...
    // 1. Read data from websocket
    let mut websocket_reader = tokio::task::spawn(async move {
        loop {
            match websocket_rx.next().await {
                Some(Ok(msg)) => {
                    if let Message::Binary(data) = msg {
                        websocket_sender.send(data).await.unwrap();
                    }
                    // We dont care about other types of messages
                }
                other => {
                    debug!("Reader websocket {connection_id} closed: {:?}", other);
                    break;
                }
            }
        }
    });
    // 2. Read data from socket
    let mut socket_reader = tokio::task::spawn(async move {
        let mut buffer = [0u8; READ_BUFFER_SIZE];
        loop {
            match tcp_socket_rx.read(&mut buffer).await {
                Ok(n) => {
                    socket_sender.send(buffer[..n].to_owned()).await.unwrap();
                }
                Err(err) => {
                    debug!("Reader socket {connection_id} closed: {:?}", err);
                    break;
                }
            }
        }
    });
    // 3. Write data to websocket
    let mut websocket_writer = tokio::task::spawn(async move {
        while let Some(data) = socket_receiver.recv().await {
            if let Err(err) = websocket_tx.send(Message::Binary(data)).await {
                debug!("Writer websocket {connection_id} returned error: {:?}", err);
                break;
            }
        }
    });
    // 4. Write data to socket
    let mut tcp_socket_writer = tokio::task::spawn(async move {
        while let Some(data) = websocket_receiver.recv().await {
            if let Err(err) = tcp_socket_tx.write(&data).await {
                debug!("Writer socket {connection_id} returned error: {:?}", err);
                break;
            }
        }
    });
    // Wait until one of these tasks return, and then abort all of them
    tokio::select! {
        _ = (&mut websocket_reader) => {},
        _ = (&mut socket_reader) => {},
        _ = (&mut websocket_writer) => {},
        _ = (&mut tcp_socket_writer) => {},
    };
    // Abort everything
    websocket_reader.abort();
    socket_reader.abort();
    websocket_writer.abort();
    tcp_socket_writer.abort();
    info!("Connection {connection_id} finished");
}
