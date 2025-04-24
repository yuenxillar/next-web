use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket},
};
use futures::{stream::StreamExt, SinkExt};
use tracing::{error, info};

use super::{session::WebSocketSession, ws_context::WebSocketContext};

/// Actual websocket statemachine (one will be spawned per connection)
pub async fn handle_socket(
    mut socket: WebSocket,
    ctx: Arc<WebSocketContext>,
    remote_address: SocketAddr,
    path: String,
) {
    info!("开始处理WebSocket连接: {remote_address}, 路径: {path}");

    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket
        .send(Message::Ping(Bytes::from_static(&[1, 2, 3])))
        .await
        .is_err()
    {
        error!("无法发送ping包到客户端 {remote_address}，连接可能已断开");
        return;
    }

    // match the corresponding processor through path matching
    let handler = match ctx.get_handler(&path) {
        Some(handler) => handler,
        None => {
            error!("找不到路径 {path} 对应的处理器，连接将被关闭");
            return;
        }
    };

    let (msg_sender, msg_receiver) = flume::unbounded();
    let session = WebSocketSession::new(msg_sender, remote_address);

    // on_open handle
    if let Err(e) = handler.on_open(&session).await {
        error!("Event on_open processing failed: {e}, Client: {remote_address}, Path: {path}");
        return;
    }

    info!("WS Connection established: {remote_address}, Path: {path}");

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut stream_sender, mut stream_receiver) = socket.split();

    // send message to client
    tokio::spawn(async move {
        while let Ok(msg) = msg_receiver.recv_async().await {
            if let Err(e) = stream_sender.send(msg).await {
                error!("Sending message to client failed: {e}, Client: {remote_address}");
                break;
            }
        }
    });

    // receive client messages
    while let Some(result) = stream_receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handler.on_message(&session, msg).await {
                    error!("Failed to process client message: {e}, Client: {remote_address}");
                    if let Err(e) =  handler.on_error(&session, e).await {
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
