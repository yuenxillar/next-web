use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket},
};
use futures::{stream::StreamExt, SinkExt};
use tracing::{debug, error, info};

use super::{session::WebSocketSession, ws_context::WebSocketContext};

/// Actual websocket statemachine (one will be spawned per connection)
pub async fn handle_socket(
    mut socket: WebSocket,
    ctx: Arc<WebSocketContext>,
    remote_address: SocketAddr,
    path: String,
) {
    debug!("Start processing WebSocket connections: {remote_address}, Path: {path}");

    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket
        .send(Message::Ping(Bytes::from_static(&[1])))
        .await
        .is_err()
    {
        error!("Unable to send ping packet to client {remote_address}, The connection may have been disconnected");
        return;
    }

    // match the corresponding processor through path matching
    let handler = match ctx.get_handler(&path) {
        Some(handler) => handler,
        None => {
            error!("Path not found {path} The corresponding processor's connection will be closed");
            let _ = socket.close().await;
            return;
        }
    };

    let (msg_sender, msg_receiver) = flume::unbounded();
    let session = WebSocketSession::new(msg_sender, remote_address);

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut stream_sender, mut stream_receiver) = socket.split();

    // on_open handle
    if let Err(e) = handler.on_open(&session).await {
        error!("Event on_open processing failed: {e}, Client: {remote_address}, Path: {path}");
        return;
    }

    info!("WS Connection established: {remote_address}, Path: {path}");

    // send message to client
    tokio::spawn(async move {
        while let Ok(msg) = msg_receiver.recv_async().await {
            let close = if let Message::Close(_) = &msg {
                true
            } else {
                false
            };
            if let Err(e) = stream_sender.send(msg).await {
                error!("Sending message to client failed: {e}, Client: {remote_address}");
                break;
            } else {
                if close {
                    break;
                }
            }
        }
    });

    // receive client messages
    while let Some(result) = stream_receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handler.on_message(&session, msg).await {
                    error!("Failed to process client message: {e}, Client: {remote_address}");
                    if let Err(e) = handler.on_error(&session, e).await {
                        error!("Failed to handle error event: {e}, Client: {remote_address}");
                    }
                    break;
                }
            }

            Err(e) => {
                error!("Failed to receive client message: {e}, Client: {remote_address}");
                let boxed_err = Box::new(e);
                let _ = handler.on_error(&session, boxed_err).await;
                break;
            }
        }
    }

    // Connection closed
    if let Err(e) = handler.on_close(&session, None).await {
        error!("Failed to handle connection closure event: {e}, Client: {remote_address}");
    }
}
