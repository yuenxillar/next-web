//! Read-only counters describing the retries of one block name.

use next_web_core::Named;

/// Read-only counters describing how a retry block behaved.
///
/// A *retry block* is one logical operation that a retry template may attempt
/// several times. Implementations are shared across threads, so every getter
/// takes `&self` and returns the counter value observed at the time of the
/// call; concurrent increments may therefore become visible between two
/// getter calls.
///
/// The name reported through [`Named::name`] identifies the block for
/// reporting purposes. Names are not required to be unique, so a metric sink
/// should combine the name with whatever else distinguishes its retry blocks.
pub trait RetryStatistics: Named + Send + Sync {
    /// Returns the number of retry blocks that completed successfully.
    fn get_complete_count(&self) -> u64;

    /// Returns the number of times a retry block was entered, regardless of
    /// how many attempts the block required.
    fn get_started_count(&self) -> u64;

    /// Returns the number of errors observed, whether or not they led to a
    /// retry attempt.
    fn get_error_count(&self) -> u64;

    /// Returns the number of retry blocks that still failed after their last
    /// attempt.
    fn get_abort_count(&self) -> u64;

    /// Returns the number of times a recovery callback produced a result for
    /// an otherwise exhausted retry block.
    fn get_recovery_count(&self) -> u64;
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use next_web_core::Named;

    use super::RetryStatistics;

    /// Minimal implementation used to prove the trait is object safe and
    /// usable behind a shared reference.
    struct CountingStatistics {
        started: AtomicU64,
    }

    impl Named for CountingStatistics {
        fn name(&self) -> &str {
            "counting"
        }
    }

    impl RetryStatistics for CountingStatistics {
        fn get_complete_count(&self) -> u64 {
            0
        }

        fn get_started_count(&self) -> u64 {
            self.started.load(Ordering::Relaxed)
        }

        fn get_error_count(&self) -> u64 {
            0
        }

        fn get_abort_count(&self) -> u64 {
            0
        }

        fn get_recovery_count(&self) -> u64 {
            0
        }
    }

    #[test]
    fn counters_are_readable_through_a_trait_object() {
        let statistics = CountingStatistics {
            started: AtomicU64::new(7),
        };
        let statistics: &dyn RetryStatistics = &statistics;

        assert_eq!(statistics.get_started_count(), 7);
        assert_eq!(statistics.get_complete_count(), 0);
        assert_eq!(statistics.name(), "counting");
    }
}
