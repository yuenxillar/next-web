use std::error::Error;

use axum::extract::ws::{CloseFrame, Message};
use next_web_core::async_trait;

use crate::server::support::ws_session::WebSocketSession;

/// Custom result type for WebSocket operations.
///
/// Wraps the standard `Result<T, E>`, using `Box<dyn Error + Send + Sync>` as the error type,
/// suitable for asynchronous processing and cross-thread error propagation.
///
pub type WSResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// WebSocket handler trait.
///
/// Defines various callback methods during the WebSocket lifecycle.
/// All methods are asynchronous, and implementers must also implement `Send + Sync`
/// to support multi-threaded environments.
///
#[async_trait]
pub trait WebSocketHandler
where
    Self: Send + Sync,
{
    /// Called when a client establishes a WebSocket connection.
    ///
    /// Can be used for session initialization, logging, or resource allocation.
    ///
    async fn on_open(&self, session: &WebSocketSession) -> WSResult<()>;

    /// Triggered when a client sends a message over the WebSocket.
    ///
    /// Receives and processes text or binary messages.
    ///
    async fn on_message(&self, session: &WebSocketSession, message: Message) -> WSResult<()>;

    /// Called when an error occurs during the connection or message transmission.
    ///
    /// Can be used for logging errors or performing exception recovery.
    ///
    async fn on_error(
        &self,
        session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> WSResult<()>;

    /// Called after handling an error or when the connection is closed normally.
    ///
    /// Can be used to release resources, clean up state, or log disconnection events.
    ///
    async fn on_close(&self, session: &WebSocketSession, close: Option<CloseFrame>)
    -> WSResult<()>;
}
