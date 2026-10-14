//! Transport layer implementations for SockJS.
//!
//! Provides different transport mechanisms for client-server communication:
//! - WebSocket (native, preferred)
//! - XHR Streaming (HTTP long-lived connection)
//! - XHR Polling (HTTP request/response cycles)
//! - EventSource (Server-Sent Events)

use std::sync::Arc;

use axum::{
    extract::{Request, ws::WebSocketUpgrade},
    response::{IntoResponse, Response},
};
use next_web_core::error::BoxError;

use crate::sockjs::session::SockJsSession;

pub mod websocket_transport_handler;
pub mod xhr_polling_transport_handler;
pub mod xhr_streaming_transport_handler;

pub use websocket_transport_handler::WebSocketTransportHandler;
pub use xhr_polling_transport_handler::XhrPollingTransportHandler;
pub use xhr_streaming_transport_handler::XhrStreamingTransportHandler;

/// Transport type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    /// Native WebSocket transport (preferred).
    WebSocket,
    /// HTTP streaming transport.
    XhrStreaming,
    /// HTTP polling transport.
    XhrPolling,
    /// Server-Sent Events transport.
    EventSource,
}

impl TransportType {
    /// Returns the name of the transport as used in SockJS URLs.
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportType::WebSocket => "websocket",
            TransportType::XhrStreaming => "xhr_streaming",
            TransportType::XhrPolling => "xhr",
            TransportType::EventSource => "eventsource",
        }
    }
}

/// Configuration for a SockJS service.
#[derive(Debug, Clone)]
pub struct SockJsServiceConfig {
    /// Enable/disable WebSocket transport
    pub websocket_enabled: bool,
    /// Whether session cookie (JSESSIONID) is needed
    pub session_cookie_needed: bool,
    /// Heartbeat interval in milliseconds
    pub heartbeat_time: u64, // milliseconds
    /// Disconnect delay in milliseconds
    pub disconnect_delay: u64, // milliseconds
    /// Stream bytes limit for HTTP streaming
    pub stream_bytes_limit: usize,
    /// HTTP message cache size for polling
    pub http_message_cache_size: usize,
    /// SockJS client library URL
    pub client_library_url: String,
    /// Suppress automatic CORS headers
    pub suppress_cors: bool,
    /// Allowed origins for CORS
    pub allowed_origins: Vec<String>,
    /// Allowed origin patterns for CORS
    pub allowed_origin_patterns: Vec<String>,
}

impl Default for SockJsServiceConfig {
    fn default() -> Self {
        Self {
            websocket_enabled: true,
            session_cookie_needed: true,
            heartbeat_time: 25_000,         // 25 seconds
            disconnect_delay: 5_000,        // 5 seconds
            stream_bytes_limit: 128 * 1024, // 128KB
            http_message_cache_size: 100,
            client_library_url: "https://cdn.jsdelivr.net/sockjs/1.0.0/sockjs.min.js".to_string(),
            suppress_cors: false,
            allowed_origins: vec![],
            allowed_origin_patterns: vec![],
        }
    }
}

/// Trait for handling different transport mechanisms in SockJS.
///
/// Each transport type (WebSocket, XHR streaming, XHR polling) implements
/// this trait to provide its specific request handling logic.
#[next_web_core::async_trait]
pub trait TransportHandler: Send + Sync {
    /// Returns the transport type this handler supports.
    fn transport_type(&self) -> TransportType;

    /// Handles an incoming request for this transport.
    ///
    /// # Arguments
    ///
    /// * `req` - The incoming HTTP request
    /// * `session` - The SockJS session associated with this connection
    /// * `config` - Service configuration
    ///
    /// # Returns
    ///
    /// The HTTP response to send back to the client.
    async fn handle_request(
        &self,
        req: Request,
        session: Arc<SockJsSession>,
        config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError>;

    /// Handles a WebSocket upgrade for transports that support Axum's
    /// `WebSocketUpgrade` extractor.
    async fn handle_websocket_upgrade(
        &self,
        _ws: WebSocketUpgrade,
        _session: Arc<SockJsSession>,
        _config: Arc<SockJsServiceConfig>,
    ) -> Result<Response, BoxError> {
        Ok(axum::http::StatusCode::NOT_IMPLEMENTED.into_response())
    }
}
