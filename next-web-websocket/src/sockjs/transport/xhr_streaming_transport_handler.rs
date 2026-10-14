//! XHR streaming transport handler for SockJS.
//!
//! This transport maintains a long-lived HTTP connection and streams
//! messages to the client as they become available.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    response::Response,
};
use futures::stream::{self, StreamExt};
use next_web_core::error::BoxError;
use tokio::time;
use tracing::debug;

use crate::sockjs::{
    session::SockJsSession,
    transport::{SockJsServiceConfig, TransportHandler, TransportType},
};

/// XHR streaming transport handler.
///
/// Maintains a long-lived HTTP connection and streams frames to the client.
/// The connection is recycled when it exceeds the byte limit configured
/// in `stream_bytes_limit`.
#[derive(Debug, Clone, Default)]
pub struct XhrStreamingTransportHandler;

impl XhrStreamingTransportHandler {
    /// Browsers commonly buffer small XHR responses. SockJS streaming
    /// transports send a 2 KiB prelude before the first protocol frame.
    const PRELUDE_BYTES: usize = 2048;

    /// Creates a new XHR streaming transport handler.
    pub fn new() -> Self {
        Self
    }

    /// Handles streaming request - keeps connection open and streams messages.
    async fn handle_stream(
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        session.touch().await;
        session.set_transport(TransportType::XhrStreaming).await;
        session.reset_bytes_sent().await;

        // If not opened yet, send open frame first
        if !session.is_opened().await {
            session.send_open().await?;
        }
        session.ensure_handler().await?;

        // Create streaming response
        let mut prelude = String::with_capacity(Self::PRELUDE_BYTES + 1);
        prelude.extend(std::iter::repeat_n('h', Self::PRELUDE_BYTES));
        prelude.push('\n');
        session.add_bytes_sent(prelude.len()).await;

        let stream = stream::once(async move { Ok::<_, BoxError>(prelude) })
            .chain(stream::unfold(
                (session.clone(), config.clone(), Instant::now()),
                move |(session, config, last_heartbeat)| async move {
                    // Drain queued close/message frames before checking the closed
                    // flag, so graceful close frames are observable by clients.
                    let messages = session
                        .poll_messages(config.http_message_cache_size.max(1))
                        .await;
                    if !messages.is_empty() {
                        session.touch().await;
                        let data = messages.join("\n") + "\n";
                        let len = data.len();
                        session.add_bytes_sent(len).await;
                        debug!("Streaming {} bytes ({} messages)", len, messages.len());
                        return Some((Ok(data), (session, config, last_heartbeat)));
                    }

                    if session.is_closed().await {
                        return None;
                    }

                    let bytes_sent = session.bytes_sent().await;
                    if bytes_sent >= config.stream_bytes_limit {
                        debug!(
                            "Stream byte limit reached ({}), closing connection",
                            bytes_sent
                        );
                        return None;
                    }

                    time::sleep(Duration::from_millis(100)).await;
                    session.touch().await;
                    if last_heartbeat.elapsed() >= Duration::from_millis(config.heartbeat_time) {
                        let data = "h\n".to_string();
                        session.add_bytes_sent(data.len()).await;
                        return Some((Ok(data), (session, config, Instant::now())));
                    }

                    Some((Ok(String::new()), (session, config, last_heartbeat)))
                },
            ))
            .filter(|result| {
                let is_empty = matches!(result, Ok(s) if s.is_empty());
                async move { !is_empty }
            });

        // Build response with streaming body
        let body = Body::from_stream(stream);

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                "application/javascript; charset=UTF-8",
            )
            .header(
                header::CACHE_CONTROL,
                "no-store, no-cache, no-transform, must-revalidate, max-age=0",
            )
            .header(header::CONNECTION, "keep-alive")
            .body(body)?;

        Ok(response)
    }
}

#[next_web_core::async_trait]
impl TransportHandler for XhrStreamingTransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::XhrStreaming
    }

    async fn handle_request(
        &self,
        req: Request,
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        // Both GET and POST are used by SockJS XHR receivers depending on
        // transport implementation; application messages use `/xhr_send`.
        match req.method().as_str() {
            "GET" => Self::handle_stream(session, config).await,
            "POST" => Self::handle_stream(session, config).await,
            _ => {
                let response = Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Body::from("Method not allowed"))?;
                Ok(response)
            }
        }
    }
}
