//! SockJS service implementations.
//!
//! The service layer manages sessions and delegates to transport handlers.

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Request, ws::WebSocketUpgrade},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use next_web_core::{any::AnyValue, error::BoxError};

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

/// The outcome of running a request through the handshake interceptors.
///
/// A SockJS request is authorized before the session it belongs to exists, and
/// the attributes its interceptors collected have to reach that session, so the
/// two travel together. On a refusal the attributes are dropped along with the
/// request: no session is created for it.
#[derive(Debug)]
pub struct Authorization {
    /// The response to answer with when an interceptor refused the request.
    /// `None` means the request may continue.
    pub refusal: Option<Response>,
    /// What the interceptors collected, handed to the session the request
    /// creates.
    pub attributes: HashMap<String, AnyValue>,
}

impl Authorization {
    /// Accepts the request, carrying `attributes` to the session it creates.
    pub fn accepted(attributes: HashMap<String, AnyValue>) -> Self {
        Self {
            refusal: None,
            attributes,
        }
    }

    /// Refuses the request with `response`.
    pub fn refused(response: Response) -> Self {
        Self {
            refusal: Some(response),
            attributes: HashMap::new(),
        }
    }
}

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
    /// support interceptors or origin restrictions should override this. What
    /// the interceptors put in the attribute map travels back in the
    /// [`Authorization`] and is handed to
    /// [`SockJsService::handle_websocket_upgrade`].
    async fn authorize_websocket_request(
        &self,
        _request: &mut Request,
    ) -> Result<Authorization, BoxError> {
        Ok(Authorization::accepted(HashMap::new()))
    }

    /// Adds CORS headers to a response for an already-authorized request.
    ///
    /// This is used for the WebSocket upgrade response. HTTP transport
    /// responses are decorated by [`SockJsService::handle_request`].
    fn add_cors_headers(&self, _request: &Request, _response: &mut Response) {}

    /// Handles the native WebSocket transport after Axum has validated the
    /// upgrade request. Implementations that do not support WebSocket can
    /// retain the default not-implemented response.
    ///
    /// `attributes` are what
    /// [`SockJsService::authorize_websocket_request`] collected, and belong on
    /// the session's application session so the handler can read them.
    async fn handle_websocket_upgrade(
        &self,
        _ws: WebSocketUpgrade,
        _session_id: String,
        _headers: HeaderMap,
        _remote_addr: SocketAddr,
        _attributes: HashMap<String, AnyValue>,
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
