//! HTTP session support.
//!
//! The module defines the [`HttpSession`] contract, the standard in-memory
//! implementation ([`StandardSession`]), and the [`SessionManager`] layer that
//! owns sessions across requests ([`DefaultSessionManager`]).

mod default_session_manager;
mod http_session;
mod session_manager;
mod standard_session;

pub use default_session_manager::DefaultSessionManager;
pub use http_session::{HttpSession, HttpSessionAccessor};
pub use session_manager::{SessionHandle, SessionManager};
pub use standard_session::{StandardSession, StandardSessionAccessor};
