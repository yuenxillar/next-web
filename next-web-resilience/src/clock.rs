//! Time source shared by the time aware resilience patterns.
//!
//! The time based sliding window of the circuit breaker and the refresh cycle
//! of the rate limiter both need to read the current time. Reading it through a
//! [`Clock`] instead of calling [`std::time::Instant::now`] directly keeps those
//! components deterministic: a test can install a [`ManualClock`] and advance
//! time instead of sleeping.

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

/// A source of monotonic time.
///
/// Implementations must never move backwards and must be cheap to call, because
/// the sliding window reads the clock on every recorded call.
pub trait Clock: Send + Sync + fmt::Debug {
    /// Returns the time elapsed since this clock's epoch.
    ///
    /// Only differences between two readings are meaningful; the value itself
    /// carries no relation to the wall clock.
    fn monotonic_time(&self) -> Duration;
}

/// A [`Clock`] backed by the operating system's monotonic clock.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    epoch: std::time::Instant,
}

impl SystemClock {
    /// Creates a clock whose epoch is the moment of this call.
    pub fn new() -> Self {
        Self {
            epoch: std::time::Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn monotonic_time(&self) -> Duration {
        self.epoch.elapsed()
    }
}

/// A [`Clock`] that only advances when the test drives it.
///
/// Cloning a `ManualClock` yields another handle onto the same underlying
/// counter, so a clone handed to a component stays in step with the original.
#[derive(Debug, Clone)]
pub struct ManualClock {
    elapsed_nanos: Arc<AtomicU64>,
}

impl ManualClock {
    /// Creates a clock positioned at its epoch.
    pub fn new() -> Self {
        Self {
            elapsed_nanos: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Moves the clock forward by `duration`.
    pub fn advance(&self, duration: Duration) {
        let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        self.elapsed_nanos.fetch_add(nanos, Ordering::SeqCst);
    }

    /// Moves the clock to an absolute offset from its epoch.
    pub fn set(&self, elapsed: Duration) {
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        self.elapsed_nanos.store(nanos, Ordering::SeqCst);
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn monotonic_time(&self) -> Duration {
        Duration::from_nanos(self.elapsed_nanos.load(Ordering::SeqCst))
    }
}

/// A reference counted, type erased [`Clock`].
pub type SharedClock = Arc<dyn Clock>;

/// Returns the shared system clock used when a configuration selects no clock.
pub(crate) fn system_clock() -> SharedClock {
    Arc::new(SystemClock::new())
}

#[cfg(test)]
mod tests {
    use super::{Clock, ManualClock, SystemClock};
    use std::time::Duration;

    #[test]
    fn system_clock_never_moves_backwards() {
        let clock = SystemClock::new();
        let first = clock.monotonic_time();
        let second = clock.monotonic_time();

        assert!(second >= first, "{second:?} should not precede {first:?}");
    }

    #[test]
    fn manual_clock_only_advances_when_driven() {
        let clock = ManualClock::new();
        assert_eq!(clock.monotonic_time(), Duration::ZERO);

        clock.advance(Duration::from_millis(250));
        assert_eq!(clock.monotonic_time(), Duration::from_millis(250));

        clock.set(Duration::from_secs(1));
        assert_eq!(clock.monotonic_time(), Duration::from_secs(1));
    }

    #[test]
    fn manual_clock_clones_share_one_counter() {
        let clock = ManualClock::new();
        let handle = clock.clone();

        handle.advance(Duration::from_secs(3));

        assert_eq!(clock.monotonic_time(), Duration::from_secs(3));
    }
}
