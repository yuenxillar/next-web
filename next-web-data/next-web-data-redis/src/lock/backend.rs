//! Redis direct/cluster command and Pub/Sub transport layer.
//! Transport selection is implemented by the concrete lock module; this
//! re-export keeps the Redisson layer boundary explicit for integrations.
pub use super::redisson_lock::RedisLockMode;
