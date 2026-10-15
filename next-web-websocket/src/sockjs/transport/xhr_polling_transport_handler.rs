//! XHR polling transport handler for SockJS.
//!
//! This transport uses HTTP request/response cycles for bidirectional communication.
//! The client repeatedly polls the server for messages.

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
use futures::stream;
use next_web_core::error::BoxError;
use tracing::debug;

use crate::sockjs::{
    session::SockJsSession,
    transport::{SockJsServiceConfig, TransportHandler, TransportType},
};

/// Largest `xhr_send` body accepted, in bytes.
///
/// The SockJS client sends its messages as one small JSON array, so this is
/// generous. It exists so that a request cannot make the server buffer an
/// arbitrary amount of data before the body is even parsed.
const MAX_SEND_BODY: usize = 64 * 1024;

/// XHR polling transport handler.
///
/// Implements HTTP long polling where `/xhr` receives queued messages using
/// POST requests and `/xhr_send` delivers client messages using POST requests.
#[derive(Debug, Clone, Default)]
pub struct XhrPollingTransportHandler;

impl XhrPollingTransportHandler {
    /// Creates a new XHR polling transport handler.
    pub fn new() -> Self {
        Self
    }

    /// Handles the receive request - returns queued messages to the client.
    async fn handle_receive(
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        session.touch().await;

        // If not opened yet, send open frame first
        if !session.is_opened().await {
            session.send_open().await?;
        }
        session.ensure_handler().await?;

        // Return the response before waiting for the next frame. This keeps
        // long-poll body reads outside the framework's request timeout while
        // still making the XHR complete only when data or a heartbeat arrives.
        let body = Body::from_stream(stream::once(async move {
            // Claim the session's single receiver slot. A later request takes it
            // over, and that is what ends this response.
            let generation = session.begin_receive().await;

            let heartbeat_deadline = Instant::now() + Duration::from_millis(config.heartbeat_time);
            let mut last_touch = Instant::now();
            let mut taken_over = false;
            let messages = loop {
                let Some(messages) = session
                    .poll_for_receiver(generation, config.http_message_cache_size.max(1))
                    .await
                else {
                    taken_over = true;
                    break Vec::new();
                };

                if !messages.is_empty() || session.is_closed().await {
                    break messages;
                }
                if Instant::now() >= heartbeat_deadline {
                    break messages;
                }

                if last_touch.elapsed() >= Duration::from_secs(1) {
                    session.touch().await;
                    last_touch = Instant::now();
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            };

            let body = if taken_over {
                // A newer receiver has this session, and the client that opened
                // this request has already moved on to it. End quietly rather than
                // sending frames to a client that is no longer listening.
                debug!("XHR polling receiver was replaced by a newer request");
                String::new()
            } else if messages.is_empty() {
                if session.is_closed().await {
                    // A close frame rather than an empty body. The queued copy is
                    // delivered once; a poll arriving after that — or a session a
                    // transport closed without going through `close` — would
                    // otherwise leave the client to infer the end from an EOF.
                    session
                        .close_frame()
                        .await
                        .map_or_else(String::new, |frame| format!("{frame}\n"))
                } else {
                    "h\n".to_string()
                }
            } else {
                messages.join("\n") + "\n"
            };

            debug!(
                "XHR polling receive response: {} bytes, {} messages",
                body.len(),
                messages.len()
            );
            Ok::<_, BoxError>(body)
        }));

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
            .body(body)?;

        Ok(response)
    }

    /// Handles `/xhr_send` POST requests containing messages from the client.
    async fn handle_send(
        req: Request,
        session: Arc<SockJsSession>,
        _config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        session.touch().await;
        if !session.is_opened().await {
            session.send_open().await?;
        }
        session.ensure_handler().await?;

        // Read request body, bounded. Without a limit a client could make the
        // server buffer an arbitrarily large POST before it was ever parsed;
        // SockJS sends its messages as one small JSON array.
        let body_bytes = match axum::body::to_bytes(req.into_body(), MAX_SEND_BODY).await {
            Ok(body) => body,
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                    .body(Body::empty())?);
            }
        };
        let body_str = String::from_utf8(body_bytes.to_vec())
            .map_err(|e| format!("Invalid UTF-8 in request body: {}", e))?;

        debug!("XHR polling POST received: {}", body_str);

        // Parse messages from client
        // Client sends messages as JSON array: ["msg1", "msg2"]
        if !body_str.is_empty() {
            let messages: Vec<String> = serde_json::from_str(&body_str)
                .map_err(|e| format!("Failed to parse messages: {}", e))?;

            debug!("Received {} messages from client", messages.len());
            for message in messages {
                session.dispatch_client_text(message).await?;
            }
        }

        // Return empty response
        let response = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(
                header::CACHE_CONTROL,
                "no-store, no-cache, no-transform, must-revalidate, max-age=0",
            )
            .body(Body::empty())?;

        Ok(response)
    }
}

#[next_web_core::async_trait]
impl TransportHandler for XhrPollingTransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::XhrPolling
    }

    async fn handle_request(
        &self,
        req: Request,
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        // Set transport type
        session.set_transport(TransportType::XhrPolling).await;

        let is_send_endpoint = req.uri().path().ends_with("/xhr_send");
        match (req.method().as_str(), is_send_endpoint) {
            ("GET", false) => Self::handle_receive(session, config).await,
            ("POST", false) => Self::handle_receive(session, config).await,
            ("POST", true) => Self::handle_send(req, session, config).await,
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::from("Method not allowed"))?),
        }
    }
}
