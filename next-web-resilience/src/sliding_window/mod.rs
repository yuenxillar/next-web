//! Sliding window counters.
//!
//! A sliding window aggregates the outcome of recent calls so that a failure
//! rate can be computed over a moving set of samples instead of over all calls
//! ever made. Two flavours are provided, matching the circuit breaker
//! configuration:
//!
//! * [`SlidingWindowType::CountBased`] keeps the outcome of the last *N* calls.
//! * [`SlidingWindowType::TimeBased`] keeps the outcome of the calls recorded
//!   during the last *T* of time, split into fixed size buckets.

pub(crate) mod count_based;
pub(crate) mod time_based;

use std::time::Duration;

use crate::clock::SharedClock;

pub(crate) use count_based::CountBasedSlidingWindow;
pub(crate) use time_based::TimeBasedSlidingWindow;

/// How the circuit breaker aggregates the outcome of recent calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlidingWindowType {
    /// Aggregate the outcome of the last *N* recorded calls.
    #[default]
    CountBased,
    /// Aggregate the outcome of the calls recorded during the last *T*.
    TimeBased,
}

/// The outcome of a single call as the sliding window records it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CallOutcome {
    /// Whether the call counted as a failure.
    pub failed: bool,
    /// Whether the call counted as slow.
    pub slow: bool,
}

impl CallOutcome {
    /// A successful, fast call.
    pub const SUCCESS: Self = Self {
        failed: false,
        slow: false,
    };

    /// Creates an outcome from its two independent flags.
    pub fn new(failed: bool, slow: bool) -> Self {
        Self { failed, slow }
    }
}

/// Aggregated call outcomes over a sliding window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowSnapshot {
    /// Number of calls currently inside the window.
    pub total_calls: u64,
    /// Number of those calls that counted as failures.
    pub failed_calls: u64,
    /// Number of those calls that counted as slow.
    pub slow_calls: u64,
}

impl WindowSnapshot {
    /// Percentage of failed calls, in the range `0.0..=100.0`.
    ///
    /// An empty window reports `0.0` rather than a division by zero.
    pub fn failure_rate(&self) -> f32 {
        percentage(self.failed_calls, self.total_calls)
    }

    /// Percentage of slow calls, in the range `0.0..=100.0`.
    ///
    /// An empty window reports `0.0` rather than a division by zero.
    pub fn slow_call_rate(&self) -> f32 {
        percentage(self.slow_calls, self.total_calls)
    }
}

fn percentage(part: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (part as f64 * 100.0 / total as f64) as f32
}

/// The sliding window a circuit breaker records its call outcomes into.
#[derive(Debug)]
pub(crate) enum SlidingWindow {
    /// Records the outcome of the last *N* calls.
    Count(CountBasedSlidingWindow),
    /// Records the outcome of the calls seen during the last *T*.
    Time(TimeBasedSlidingWindow),
}

impl SlidingWindow {
    /// Builds a count based window holding `size` calls.
    pub(crate) fn count_based(size: u32) -> Self {
        Self::Count(CountBasedSlidingWindow::new(size))
    }

    /// Builds a time based window spanning `window`, split into buckets.
    pub(crate) fn time_based(window: Duration, bucket_count: u32, clock: SharedClock) -> Self {
        Self::Time(TimeBasedSlidingWindow::new(window, bucket_count, clock))
    }

    /// Records one call outcome.
    pub(crate) fn record(&mut self, outcome: CallOutcome) {
        match self {
            Self::Count(window) => window.record(outcome),
            Self::Time(window) => window.record(outcome),
        }
    }

    /// Returns the counters currently inside the window.
    pub(crate) fn snapshot(&self) -> WindowSnapshot {
        match self {
            Self::Count(window) => window.snapshot(),
            Self::Time(window) => window.snapshot(),
        }
    }

    /// Discards every recorded outcome.
    pub(crate) fn reset(&mut self) {
        match self {
            Self::Count(window) => window.reset(),
            Self::Time(window) => window.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WindowSnapshot;

    #[test]
    fn empty_snapshot_reports_zero_rates() {
        let snapshot = WindowSnapshot::default();

        assert_eq!(snapshot.failure_rate(), 0.0);
        assert_eq!(snapshot.slow_call_rate(), 0.0);
    }

    #[test]
    fn rates_are_percentages_of_the_total() {
        let snapshot = WindowSnapshot {
            total_calls: 8,
            failed_calls: 2,
            slow_calls: 4,
        };

        assert_eq!(snapshot.failure_rate(), 25.0);
        assert_eq!(snapshot.slow_call_rate(), 50.0);
    }
}
