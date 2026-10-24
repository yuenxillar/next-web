//! Equivalent of Redisson's `RedissonMultiLock` composition layer.
//!
//! The quorum implementation uses the same owner-safe child-lock contract as
//! `RedissonLock`; this module documents that composition boundary without
//! exposing an unsafe generic collection API.

/// Majority quorum used by Redisson's RedLock composition.
pub const fn quorum(node_count: usize) -> usize {
    node_count / 2 + 1
}
