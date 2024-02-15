use std::{process::exit, str::FromStr};

use futures::StreamExt;
use log::{debug, error, info, warn};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

mod proxy;

pub async fn start_remote_controller(cloudflare_server_address: String, forward_address: String) {
    // First thing we should do is starting a websocket client as the controller of the
    // local computer.
    let (mut controller_websocket, _) =
        connect_async(format!("{}/control", cloudflare_server_address))
            .await
            .expect("cannot parse the cloudflare_server_address");
    debug!("Controller connected");
    // Get the ack message
    match controller_websocket.next().await {
        Some(Ok(Message::Text(msg))) => {
            if msg != "ack" {
                error!("First packet is not ack: {msg}");
                exit(1);
            }
        }
        other => {
            error!("First packet is not ack: {:?}", other);
            exit(1);
        }
    }
    // Leak the addresses because memory leaks are cool.
    // We need these variables throughout the whole program and threads. So we can simply leak it
    let cloudflare_server_address: &'static str = Box::leak(Box::new(cloudflare_server_address));
    let forward_address: &'static str = Box::leak(Box::new(forward_address));
    // The connected websocket is only used to read the commands
    info!("Controller connection established");
    loop {
        // Read the command from websocket
        match controller_websocket.next().await {
            Some(Ok(command)) => {
                if let Message::Text(command) = command {
                    // The only message type supported right now is simply the connection request
                    // that sends the UUID of the connection in the socket!
                    let requested_uuid = Uuid::from_str(&command);
                    if let Err(err) = requested_uuid {
                        warn!("Invalid packet received from local server: {:?}", err);
                        continue;
                    }
                    let requested_uuid = requested_uuid.unwrap();
                    // Create a task that handles the connection
                    tokio::task::spawn(proxy::handle_new_connection_request(
                        requested_uuid,
                        cloudflare_server_address,
                        forward_address,
                    ));
                }
            }
            other => {
                error!("Invalid message: {:?}", other);
                exit(1);
            }
        }
    }
}
