//! WebSocket transport handler for SockJS.
//!
//! This handler provides native WebSocket support, which is the preferred
//! transport when available.

use std::sync::Arc;

use axum::{
    extract::{Request, ws::WebSocket},
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use next_web_core::error::BoxError;
use tracing::{debug, error, info};

use crate::{
    server::support::ws_session::WebSocketSession,
    sockjs::{
        heartbeat::HeartbeatHandle,
        session::SockJsSession,
        transport::{SockJsServiceConfig, TransportHandler, TransportType},
    },
};

/// WebSocket transport handler.
///
/// This is the preferred transport, providing full-duplex communication
/// over a native WebSocket connection.
#[derive(Debug, Clone, Default)]
pub struct WebSocketTransportHandler;

impl WebSocketTransportHandler {
    /// Creates a new WebSocket transport handler.
    pub fn new() -> Self {
        Self
    }

    /// Handles the WebSocket upgrade and connection lifecycle.
    async fn handle_websocket(
        socket: WebSocket,
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) {
        let session_id = session.id().to_string();
        let remote_addr = session.remote_addr();

        debug!(
            "SockJS WebSocket connection established: {} (session: {})",
            remote_addr, session_id
        );

        let (mut ws_sender, mut ws_receiver) = socket.split();
        let (msg_tx, msg_rx) = flume::bounded(128);
        let ws_session = WebSocketSession::new(
            msg_tx,
            remote_addr,
            session.headers().clone(),
            format!("/sockjs/{}", session_id),
        );
        session.attach_websocket(ws_session).await;

        if let Err(e) = session.send_open().await {
            error!("Failed to send open frame: {}", e);
            let _ = ws_sender.close().await;
            session.detach_websocket().await;
            return;
        }

        let sender_task = tokio::spawn(async move {
            while let Ok(msg) = msg_rx.recv_async().await {
                let close_transport = matches!(
                    &msg,
                    axum::extract::ws::Message::Text(text) if text.starts_with("c[")
                );
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
                if close_transport {
                    let _ = ws_sender.close().await;
                    break;
                }
            }
        });

        if let Err(error) = session.ensure_handler().await {
            error!("SockJS application on_open failed: {}", error);
            let _ = session
                .close(3000, "Application handler failed".to_string())
                .await;
        }

        let heartbeat = HeartbeatHandle::spawn(session.clone(), config.heartbeat_time.max(1));

        let mut close_frame = None;
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    session.touch().await;
                    match msg {
                        axum::extract::ws::Message::Text(text) => {
                            match serde_json::from_str::<Vec<String>>(&text) {
                                Ok(messages) => {
                                    for message in messages {
                                        if let Err(error) =
                                            session.dispatch_client_text(message).await
                                        {
                                            error!(
                                                "SockJS application on_message failed: {}",
                                                error
                                            );
                                            let _ = session
                                                .close(3000, "Application handler failed".into())
                                                .await;
                                            break;
                                        }
                                    }
                                }
                                Err(error) => {
                                    debug!("Invalid SockJS WebSocket message: {}", error);
                                    let _ =
                                        session.close(3002, "Invalid SockJS message".into()).await;
                                    break;
                                }
                            }
                        }
                        axum::extract::ws::Message::Binary(_) => {
                            debug!("Received binary message (ignoring)");
                        }
                        axum::extract::ws::Message::Close(frame) => {
                            info!("WebSocket closed by client: {:?}", frame);
                            close_frame = frame;
                            break;
                        }
                        axum::extract::ws::Message::Ping(_) => {
                            // Pongs are handled automatically by axum
                        }
                        axum::extract::ws::Message::Pong(_) => {
                            // Received pong
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        heartbeat.stop();
        sender_task.abort();
        session.notify_close(close_frame).await;
        session.detach_websocket().await;
        debug!("SockJS WebSocket connection closed: {}", session_id);
    }
}

#[next_web_core::async_trait]
impl TransportHandler for WebSocketTransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }

    async fn handle_request(
        &self,
        _req: Request,
        _session: Arc<SockJsSession>,
        _config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        // WebSocket transport requires special handling via the router
        // This method is not used directly - see handle_websocket instead
        Err("WebSocket transport must be handled via Axum extractors".into())
    }

    async fn handle_websocket_upgrade(
        &self,
        ws_upgrade: axum::extract::ws::WebSocketUpgrade,
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        Ok(Self::handle_with_upgrade(ws_upgrade, session, config).await)
    }
}

impl WebSocketTransportHandler {
    /// Handle WebSocket connection with extractors.
    ///
    /// This should be called from an Axum handler that extracts WebSocketUpgrade.
    pub async fn handle_with_upgrade(
        ws_upgrade: axum::extract::ws::WebSocketUpgrade,
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Response {
        ws_upgrade
            .on_upgrade(move |socket| Self::handle_websocket(socket, session, config))
            .into_response()
    }
}
