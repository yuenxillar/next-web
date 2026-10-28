//! The blocking lock flavour.

pub mod guard;
pub mod lock;
pub mod watchdog;

pub use guard::BlockingRedisLockGuard;
pub use lock::BlockingRedisLock;
