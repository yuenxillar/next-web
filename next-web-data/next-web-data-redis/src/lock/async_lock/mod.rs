//! The asynchronous lock flavour.

pub mod guard;
pub mod lock;
pub mod watchdog;

pub use guard::RedisLockGuard;
pub use lock::RedisLock;
