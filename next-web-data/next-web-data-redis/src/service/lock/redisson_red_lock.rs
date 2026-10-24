//! Equivalent of Redisson's deprecated `RedissonRedLock`.

pub use super::redisson_lock::RedisRedLock;

/// Returns the minimum child-lock count required for RedLock success.
pub const fn min_locks_amount(node_count: usize) -> usize {
    node_count / 2 + 1
}
