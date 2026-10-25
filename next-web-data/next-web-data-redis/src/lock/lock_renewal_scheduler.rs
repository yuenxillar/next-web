//! Equivalent of Redisson's `LockRenewalScheduler` and `LockTask`.
//!
//! The concrete guard starts and stops renewal jobs while this module owns the
//! scheduler timing contract.

use std::time::Duration;

/// Redisson lock renewal period (watchdog timeout divided by three).
pub const fn renewal_period(watchdog_timeout: Duration) -> Duration {
    super::redisson_expirable::watchdog_interval(watchdog_timeout)
}
