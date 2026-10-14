//! Count based sliding window.
//!
//! Keeps a fixed size ring buffer holding the outcome of the last *N* calls.
//! Until the buffer is full the snapshot only covers the calls actually
//! recorded, so a freshly created window does not report the zero-initialised
//! slots as successful calls.

use super::{CallOutcome, WindowSnapshot};

/// Marks a recorded call as a failure.
const FAILED: u8 = 0b0000_0001;
/// Marks a recorded call as slow.
const SLOW: u8 = 0b0000_0010;

#[derive(Debug)]
pub(crate) struct CountBasedSlidingWindow {
    /// One byte per slot; the low bits carry the flags above.
    slots: Vec<u8>,
    /// Index the next outcome is written to.
    head: usize,
    /// Number of slots written so far, saturating at the buffer length.
    recorded: usize,
}

impl CountBasedSlidingWindow {
    /// Creates a window holding the last `size` calls.
    ///
    /// A size of zero would make the window useless, so it is raised to one.
    pub(crate) fn new(size: u32) -> Self {
        let size = size.max(1) as usize;
        Self {
            slots: vec![0; size],
            head: 0,
            recorded: 0,
        }
    }

    pub(crate) fn record(&mut self, outcome: CallOutcome) {
        let mut flags = 0u8;
        if outcome.failed {
            flags |= FAILED;
        }
        if outcome.slow {
            flags |= SLOW;
        }

        self.slots[self.head] = flags;
        self.head = (self.head + 1) % self.slots.len();
        if self.recorded < self.slots.len() {
            self.recorded += 1;
        }
    }

    pub(crate) fn snapshot(&self) -> WindowSnapshot {
        let mut snapshot = WindowSnapshot::default();
        let wrapped = self.recorded == self.slots.len();

        for offset in 0..self.recorded {
            // Once the buffer has wrapped, the oldest sample sits at `head`.
            // Before that, samples occupy the buffer from the start.
            let index = if wrapped {
                (self.head + offset) % self.slots.len()
            } else {
                offset
            };

            let flags = self.slots[index];
            snapshot.total_calls += 1;
            if flags & FAILED != 0 {
                snapshot.failed_calls += 1;
            }
            if flags & SLOW != 0 {
                snapshot.slow_calls += 1;
            }
        }

        snapshot
    }

    pub(crate) fn reset(&mut self) {
        self.slots.fill(0);
        self.head = 0;
        self.recorded = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::CountBasedSlidingWindow;
    use crate::sliding_window::CallOutcome;

    fn failure() -> CallOutcome {
        CallOutcome::new(true, false)
    }

    fn success() -> CallOutcome {
        CallOutcome::SUCCESS
    }

    #[test]
    fn empty_window_reports_no_calls() {
        let window = CountBasedSlidingWindow::new(4);

        assert_eq!(window.snapshot().total_calls, 0);
    }

    #[test]
    fn partial_window_covers_only_recorded_calls() {
        let mut window = CountBasedSlidingWindow::new(4);

        window.record(success());
        window.record(failure());

        let snapshot = window.snapshot();
        assert_eq!(snapshot.total_calls, 2);
        assert_eq!(snapshot.failed_calls, 1);
        assert_eq!(snapshot.failure_rate(), 50.0);
    }

    #[test]
    fn full_window_evicts_the_oldest_outcome() {
        let mut window = CountBasedSlidingWindow::new(3);

        window.record(failure());
        window.record(success());
        window.record(success());
        assert_eq!(window.snapshot().failed_calls, 1);

        // Evicts the oldest failure, so the window turns fully successful.
        window.record(success());

        let snapshot = window.snapshot();
        assert_eq!(snapshot.total_calls, 3);
        assert_eq!(snapshot.failed_calls, 0);
    }

    #[test]
    fn slow_calls_are_counted_independently_of_failures() {
        let mut window = CountBasedSlidingWindow::new(4);

        window.record(CallOutcome::new(true, true));
        window.record(CallOutcome::new(false, true));

        let snapshot = window.snapshot();
        assert_eq!(snapshot.total_calls, 2);
        assert_eq!(snapshot.failed_calls, 1);
        assert_eq!(snapshot.slow_calls, 2);
        assert_eq!(snapshot.slow_call_rate(), 100.0);
    }

    #[test]
    fn zero_size_is_raised_to_a_single_slot() {
        let mut window = CountBasedSlidingWindow::new(0);

        window.record(failure());
        window.record(success());

        assert_eq!(window.snapshot().total_calls, 1);
    }

    #[test]
    fn reset_clears_every_slot() {
        let mut window = CountBasedSlidingWindow::new(2);
        window.record(failure());
        window.record(failure());

        window.reset();

        assert_eq!(window.snapshot().total_calls, 0);
        assert_eq!(window.snapshot().failed_calls, 0);
    }
}
