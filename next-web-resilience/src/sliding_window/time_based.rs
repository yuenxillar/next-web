//! Time based sliding window.
//!
//! Keeps the outcome of the calls recorded during the last *T* of time. The
//! window is divided into fixed size buckets and each bucket only holds the
//! outcomes of the cycle it currently belongs to, so advancing time evicts old
//! samples without walking or shifting the buffer.
//!
//! Time is read through a [`Clock`](crate::clock::Clock), which lets a test
//! advance the window deterministically instead of sleeping.

use std::time::Duration;

use super::{CallOutcome, WindowSnapshot};
use crate::clock::SharedClock;

/// A single time slice of the window.
#[derive(Debug, Clone, Copy)]
struct Bucket {
    /// Cycle the counters below belong to; [`u64::MAX`] marks an unused bucket.
    cycle: u64,
    total: u64,
    failed: u64,
    slow: u64,
}

impl Bucket {
    const EMPTY: Self = Self {
        cycle: u64::MAX,
        total: 0,
        failed: 0,
        slow: 0,
    };

    /// Re-points the bucket at `cycle`, discarding the previous counters.
    fn rewind(&mut self, cycle: u64) {
        *self = Self {
            cycle,
            ..Self::EMPTY
        };
    }
}

#[derive(Debug)]
pub(crate) struct TimeBasedSlidingWindow {
    buckets: Vec<Bucket>,
    /// Length of one bucket, in nanoseconds. Never zero.
    bucket_nanos: u128,
    clock: SharedClock,
}

impl TimeBasedSlidingWindow {
    /// Creates a window spanning `window` and split into `bucket_count` slices.
    pub(crate) fn new(window: Duration, bucket_count: u32, clock: SharedClock) -> Self {
        let bucket_count = bucket_count.max(1);
        let bucket_nanos = (window / bucket_count).as_nanos().max(1);

        Self {
            buckets: vec![Bucket::EMPTY; bucket_count as usize],
            bucket_nanos,
            clock,
        }
    }

    /// Returns the cycle `elapsed` falls into.
    fn cycle_of(&self, elapsed: Duration) -> u64 {
        u64::try_from(elapsed.as_nanos() / self.bucket_nanos).unwrap_or(u64::MAX)
    }

    pub(crate) fn record(&mut self, outcome: CallOutcome) {
        let cycle = self.cycle_of(self.clock.monotonic_time());
        let index = (cycle % self.buckets.len() as u64) as usize;

        let bucket = &mut self.buckets[index];
        if bucket.cycle != cycle {
            bucket.rewind(cycle);
        }

        bucket.total += 1;
        if outcome.failed {
            bucket.failed += 1;
        }
        if outcome.slow {
            bucket.slow += 1;
        }
    }

    pub(crate) fn snapshot(&self) -> WindowSnapshot {
        let current = self.cycle_of(self.clock.monotonic_time());
        let span = self.buckets.len() as u64;

        let mut snapshot = WindowSnapshot::default();
        for bucket in &self.buckets {
            // Skip buckets that were never written and buckets that have aged
            // out of the window.
            let in_window = current
                .checked_sub(bucket.cycle)
                .is_some_and(|age| age < span);
            if !in_window {
                continue;
            }

            snapshot.total_calls += bucket.total;
            snapshot.failed_calls += bucket.failed;
            snapshot.slow_calls += bucket.slow;
        }

        snapshot
    }

    pub(crate) fn reset(&mut self) {
        self.buckets.fill(Bucket::EMPTY);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::TimeBasedSlidingWindow;
    use crate::{
        clock::{Clock, ManualClock},
        sliding_window::CallOutcome,
    };

    /// Builds a window spanning `seconds` seconds split into `seconds` buckets,
    /// so one bucket covers exactly one second.
    fn window(seconds: u64, clock: &ManualClock) -> TimeBasedSlidingWindow {
        TimeBasedSlidingWindow::new(
            Duration::from_secs(seconds),
            seconds as u32,
            Arc::new(clock.clone()) as Arc<dyn Clock>,
        )
    }

    #[test]
    fn empty_window_reports_no_calls() {
        let clock = ManualClock::new();
        clock.advance(Duration::from_secs(10));

        assert_eq!(window(5, &clock).snapshot().total_calls, 0);
    }

    #[test]
    fn calls_in_the_current_bucket_are_counted() {
        let clock = ManualClock::new();
        let mut window = window(5, &clock);

        window.record(CallOutcome::new(true, false));
        window.record(CallOutcome::SUCCESS);

        let snapshot = window.snapshot();
        assert_eq!(snapshot.total_calls, 2);
        assert_eq!(snapshot.failed_calls, 1);
    }

    #[test]
    fn outcomes_older_than_the_window_age_out() {
        let clock = ManualClock::new();
        let mut window = window(2, &clock);

        window.record(CallOutcome::new(true, false));
        assert_eq!(window.snapshot().total_calls, 1);

        // Move past the whole window; the failure must no longer contribute.
        clock.advance(Duration::from_secs(3));

        assert_eq!(window.snapshot().total_calls, 0);
        assert_eq!(window.snapshot().failed_calls, 0);
    }

    #[test]
    fn a_bucket_is_reused_once_its_cycle_comes_around_again() {
        let clock = ManualClock::new();
        let mut window = window(2, &clock);

        window.record(CallOutcome::new(true, false));
        clock.advance(Duration::from_secs(2));

        // This write lands in the same buffer slot as the first call, so the
        // stale counters must have been discarded rather than accumulated.
        window.record(CallOutcome::SUCCESS);

        let snapshot = window.snapshot();
        assert_eq!(snapshot.total_calls, 1);
        assert_eq!(snapshot.failed_calls, 0);
    }

    #[test]
    fn partial_ageing_keeps_only_the_recent_bucket() {
        let clock = ManualClock::new();
        let mut window = window(3, &clock);

        window.record(CallOutcome::new(true, false));
        clock.advance(Duration::from_secs(1));
        window.record(CallOutcome::SUCCESS);

        // Both buckets are still inside a three second window.
        assert_eq!(window.snapshot().total_calls, 2);

        clock.advance(Duration::from_secs(2));
        // Only the successful call, now two seconds old, survives.
        let snapshot = window.snapshot();
        assert_eq!(snapshot.total_calls, 1);
        assert_eq!(snapshot.failed_calls, 0);
    }

    #[test]
    fn zero_duration_window_does_not_divide_by_zero() {
        let clock = ManualClock::new();
        let mut window = TimeBasedSlidingWindow::new(
            Duration::ZERO,
            4,
            Arc::new(clock) as Arc<dyn Clock>,
        );

        window.record(CallOutcome::new(true, false));

        assert_eq!(window.snapshot().total_calls, 1);
    }

    #[test]
    fn reset_clears_every_bucket() {
        let clock = ManualClock::new();
        let mut window = window(4, &clock);
        window.record(CallOutcome::new(true, true));

        window.reset();

        assert_eq!(window.snapshot().total_calls, 0);
        assert_eq!(window.snapshot().slow_calls, 0);
    }
}
