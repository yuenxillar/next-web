//! The quorum lock flavour.

pub mod guard;
pub mod lock;

pub use guard::{BlockingRedLockGuard, RedLockGuard};
pub use lock::{BlockingRedLock, RedLock, quorum_for};
