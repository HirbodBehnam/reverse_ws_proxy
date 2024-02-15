use futures::SinkExt;
use log::{info, trace, warn};
use parking_lot::Mutex;

use std::ops::DerefMut;

use tokio::sync::mpsc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use uuid::Uuid;

use futures::stream::{SplitSink, StreamExt};

/// The possible commands that we can be sent to the controller.
pub(crate) enum ControllerCommand {
    /// Request for a new connection with a specific UUID
    NewConnection(Uuid),
}

/// How many messages can be queued in the CONTROLLER_COMMANDER
const CONTROLLER_COMMANDER_CHAN_LENGTH: usize = 10;

/// One side of a channel which
pub(crate) static CONTROLLER_COMMANDER: Mutex<Option<mpsc::Sender<ControllerCommand>>> =
    Mutex::new(Option::None);

/// Entry point of websockets which are coming to control type
pub(crate) async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    // We only allow on instance of the controller.
    let mut commander = CONTROLLER_COMMANDER.lock();
    if commander.is_some() {
        drop(commander);
        warn!("Duplicate controller");
        // Well, no. LOL
        return axum::http::Response::builder()
            .status(axum::http::StatusCode::CONFLICT)
            .body(axum::body::Body::empty())
            .unwrap();
    }
    // Create the channel
    let (command_sender, command_receiver) = mpsc::channel(CONTROLLER_COMMANDER_CHAN_LENGTH);
    *commander.deref_mut() = Some(command_sender);
    drop(commander);
    // Finalize the upgrade process by returning upgrade callback.
    info!("Detected a new commander");
    ws.on_upgrade(move |socket| async {
        handle_socket(socket, command_receiver).await;
        CONTROLLER_COMMANDER.lock().take(); // empty the commander
        warn!("Commander died");
    })
}

/// Handle the connection of the controller
async fn handle_socket(
    mut socket: WebSocket,
    mut command_receiver: mpsc::Receiver<ControllerCommand>,
) {
    // At first send an ack
    if let Err(err) = socket.send(Message::Text("ack".to_owned())).await {
        warn!("Commander did not send the ack: {err}");
        return;
    }
    // Create another task for watch for the incoming data from the websocket.
    // In that case, we can catch the errors. Note that I could have possibly just put it in the
    // select loop but I think this is quite nicer because the data will be continuously pulled.
    // Plus, I don't now if receiver.next() is cancel safe or not.
    let (mut sender, mut receiver) = socket.split();
    let mut recv_packet = tokio::spawn(async move {
        // Ignore all messages except the close message
        while let Some(Ok(Message::Close(close_code))) = receiver.next().await {
            warn!("Controller died: {:?}", close_code);
            return;
        }
    });
    // In a loop, wait for events
    loop {
        tokio::select! {
            // If the recv_packet is done, we can simply bail
            _ = (&mut recv_packet) => return,
            // But also check for commands
            command = command_receiver.recv() => {
                match command {
                    Some(command) => handle_control_command(command, &mut sender).await,
                    None => unreachable!("command receiver closed"),
                }
            }
        }
    }
    // At last, wait for their one to finish
}

/// handle_control_command will handle a command
async fn handle_control_command(command: ControllerCommand, sender: &mut SplitSink<WebSocket, Message>) {
    match command {
        ControllerCommand::NewConnection(uuid) => {
            // Just send the uuid in the socket
            // TODO: what i should do with the result
            trace!("Asking for new connection: {uuid}");
            let _ = sender.send(Message::Text(uuid.to_string())).await;
        },
    }
}