//! Equivalent of Redisson's `RedissonObject`.
//!
//! Owns logical lock naming and Redis hash-tag conventions. Concrete naming
//! and protocol helpers remain private to `redisson_lock` until extraction is
//! complete, so callers cannot accidentally create incompatible keys.

/// Prefixes a Redisson object name while preserving an existing hash tag.
pub fn prefix_name(prefix: &str, name: &str) -> String {
    if name.contains('{') {
        format!("{prefix}:{name}")
    } else {
        format!("{prefix}:{{{name}}}")
    }
}
