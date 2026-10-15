//! SockJS protocol implementation for WebSocket fallback support.
//!
//! SockJS provides cross-browser WebSocket-like functionality with automatic
//! fallback to HTTP-based transports when native WebSocket is unavailable.

pub mod codec;
pub mod frame;
pub mod heartbeat;
pub mod info;
pub mod router;
pub mod service;
pub mod session;
pub mod transport;

pub use frame::SockJsFrame;
pub use service::{DefaultSockJsService, SockJsService, TransportHandlingSockJsService};
pub use session::SockJsSession;
pub use transport::{TransportHandler, TransportType};

/// Attribute whose value identifies who a SockJS session belongs to.
///
/// A handshake interceptor sets it, and from then on only a request whose
/// interceptor produces the same value may use that session. Turn the check on
/// with `SockJsServiceRegistration::set_session_identity_attribute`, either with
/// this name or with one of your own:
///
/// ```ignore
/// attributes.insert(
///     next_web_websocket::SESSION_IDENTITY_ATTRIBUTE.to_owned(),
///     AnyValue::from(principal.user_id()),
/// );
/// ```
///
/// This is the check that still works behind a reverse proxy, where every client
/// shares one address and the IP pin cannot tell them apart.
pub const SESSION_IDENTITY_ATTRIBUTE: &str = "next.sockjs.identity";
