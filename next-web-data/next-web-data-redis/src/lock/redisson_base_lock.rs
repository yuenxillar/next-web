//! Equivalent of Redisson's `RedissonBaseLock`.
//!
//! Defines the owner field shape (`client-id:owner-id`), reentrancy contract,
//! and explicit asynchronous unlock boundary shared by RLock variants.

/// Builds the Redis hash field used by Redisson 4.7.0.
pub fn lock_name(client_id: &str, owner_id: &str) -> String {
    format!("{client_id}:{owner_id}")
}
