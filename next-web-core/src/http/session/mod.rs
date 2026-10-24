//! HTTP session support.
//!
//! The module defines the [`HttpSession`] contract, the standard in-memory
//! implementation ([`StandardSession`]), and the [`SessionManager`] layer that
//! owns sessions across requests ([`DefaultSessionManager`]).
//!
//! Sessions are looked up from the cookie named [`SESSION_COOKIE_NAME`]. The
//! [`global_session_manager`] is the process-wide store used by a request when
//! no manager was installed on it, so sessions survive across requests by
//! default.

use std::sync::{Arc, OnceLock};

mod default_session_manager;
mod http_session;
mod session_manager;
mod standard_session;

pub use default_session_manager::DefaultSessionManager;
pub use http_session::{HttpSession, HttpSessionAccessor};
pub use session_manager::{
    generate_session_id, SessionHandle, SessionLifecycleListener, SessionManager,
};
pub use standard_session::{StandardSession, StandardSessionAccessor};

/// The name of the cookie that carries the session identifier.
pub const SESSION_COOKIE_NAME: &str = "NEXT_SESSION";

/// The process-wide default session manager.
///
/// It is created lazily on first use and backs every request that does not have
/// a manager installed on it, so sessions persist across requests without any
/// extra wiring.
static GLOBAL_SESSION_MANAGER: OnceLock<Arc<dyn SessionManager>> = OnceLock::new();

/// Returns the process-wide default session manager, creating it on first use.
#[must_use]
pub fn global_session_manager() -> Arc<dyn SessionManager> {
    GLOBAL_SESSION_MANAGER
        .get_or_init(|| {
            let manager = DefaultSessionManager::shared();
            manager as Arc<dyn SessionManager>
        })
        .clone()
}

/// Installs `manager` as the process-wide default session manager.
///
/// This can only succeed once per process. It returns the already-installed
/// manager when one is present.
///
/// # Errors
///
/// Returns the existing manager when the global manager was already installed.
pub fn install_global_session_manager(
    manager: Arc<dyn SessionManager>,
) -> Result<(), Arc<dyn SessionManager>> {
    GLOBAL_SESSION_MANAGER.set(manager)
}
