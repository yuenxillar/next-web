//! SockJS service implementations.
//!
//! The service layer manages sessions and delegates to transport handlers.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Request, ws::WebSocketUpgrade},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use next_web_core::error::BoxError;

use crate::{
    server::handshake_interceptor::HandshakeInterceptor,
    sockjs::{session::SockJsSession, transport::SockJsServiceConfig},
};

/// Default SockJS service implementation.
pub mod default_sockjs_service;
/// Transport-handling SockJS service implementation.
pub mod transport_handling_sockjs_service;

pub use default_sockjs_service::DefaultSockJsService;
pub use transport_handling_sockjs_service::TransportHandlingSockJsService;

/// Core trait for SockJS service implementations.
///
/// A SockJS service manages sessions and coordinates transport handlers
/// to provide WebSocket fallback functionality.
#[next_web_core::async_trait]
pub trait SockJsService: Send + Sync {
    /// Handles an incoming SockJS request.
    ///
    /// The service is responsible for:
    /// - Extracting session ID from the request path
    /// - Creating or retrieving the session
    /// - Delegating to the appropriate transport handler
    async fn handle_request(&self, req: Request) -> Result<Response, BoxError>;

    /// Gives a service a chance to validate a native SockJS WebSocket
    /// handshake before Axum upgrades the connection.
    ///
    /// The default implementation accepts the request. Implementations that
    /// support interceptors or origin restrictions should override this.
    async fn authorize_websocket_request(
        &self,
        _request: &mut Request,
    ) -> Result<Option<Response>, BoxError> {
        Ok(None)
    }

    /// Adds CORS headers to a response for an already-authorized request.
    ///
    /// This is used for the WebSocket upgrade response. HTTP transport
    /// responses are decorated by [`SockJsService::handle_request`].
    fn add_cors_headers(&self, _request: &Request, _response: &mut Response) {}

    /// Handles the native WebSocket transport after Axum has validated the
    /// upgrade request. Implementations that do not support WebSocket can
    /// retain the default not-implemented response.
    async fn handle_websocket_upgrade(
        &self,
        _ws: WebSocketUpgrade,
        _session_id: String,
        _headers: HeaderMap,
        _remote_addr: SocketAddr,
    ) -> Result<Response, BoxError> {
        Ok(axum::http::StatusCode::NOT_IMPLEMENTED.into_response())
    }

    /// Retrieves an existing session by ID.
    fn get_session(&self, session_id: &str) -> Option<Arc<SockJsSession>>;

    /// Creates a new session with the given ID.
    async fn create_session(
        &self,
        session_id: String,
        remote_addr: SocketAddr,
        headers: axum::http::HeaderMap,
    ) -> Arc<SockJsSession>;

    /// Gets the service configuration.
    fn config(&self) -> Arc<SockJsServiceConfig>;

    /// Gets registered handshake interceptors.
    fn interceptors(&self) -> &[Arc<dyn HandshakeInterceptor>];
}
