//! Equivalent of Redisson's `RedissonExpirable`.
//!
//! This layer defines the expiration contract shared by locks: explicit
//! lease TTL, watchdog TTL, and remaining TTL semantics.

use std::time::Duration;

/// Default Redisson 4.7.0 lock watchdog timeout.
pub const DEFAULT_LOCK_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);

/// Redisson renews watchdog locks at one third of the watchdog timeout.
pub const fn watchdog_interval(timeout: Duration) -> Duration {
    Duration::from_millis(timeout.as_millis() as u64 / 3)
}
