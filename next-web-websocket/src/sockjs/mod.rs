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
