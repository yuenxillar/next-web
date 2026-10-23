//! The mutating companion of the statistics interface.

use crate::stats::retry_statistics::RetryStatistics;

/// Statistics whose counters can be advanced while the value is shared.
///
/// Every mutator takes `&self`: implementations are kept behind `Arc` and are
/// updated from several threads, so they own their synchronization (typically
/// relaxed atomic counters). A mutator must not fail and should not block,
/// because it runs on the retry hot path.
///
/// The `increment_*` methods are the primitives that a recorder calls; the
/// shorter `started`, `complete`, `recovery`, `error` and `abort` methods are
/// provided shorthands for the same operations.
pub trait MutableRetryStatistics: RetryStatistics {
    /// Records that a retry block was entered.
    fn increment_started_count(&self);

    /// Records that a retry block completed successfully.
    fn increment_complete_count(&self);

    /// Records that a recovery callback produced a result.
    fn increment_recovery_count(&self);

    /// Records that an error was observed.
    fn increment_error_count(&self);

    /// Records that a retry block gave up without completing.
    fn increment_abort_count(&self);

    /// Shorthand for [`MutableRetryStatistics::increment_started_count`].
    fn started(&self) {
        self.increment_started_count();
    }

    /// Shorthand for [`MutableRetryStatistics::increment_complete_count`].
    fn complete(&self) {
        self.increment_complete_count();
    }

    /// Shorthand for [`MutableRetryStatistics::increment_recovery_count`].
    fn recovery(&self) {
        self.increment_recovery_count();
    }

    /// Shorthand for [`MutableRetryStatistics::increment_error_count`].
    fn error(&self) {
        self.increment_error_count();
    }

    /// Shorthand for [`MutableRetryStatistics::increment_abort_count`].
    fn abort(&self) {
        self.increment_abort_count();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use next_web_core::Named;

    use super::MutableRetryStatistics;
    use crate::stats::retry_statistics::RetryStatistics;

    /// Counts every mutator in a single total so the shorthand methods can be
    /// observed through the read-only trait.
    #[derive(Default)]
    struct TotalStatistics {
        total: AtomicU64,
    }

    impl Named for TotalStatistics {
        fn name(&self) -> &str {
            "total"
        }
    }

    impl RetryStatistics for TotalStatistics {
        fn get_complete_count(&self) -> u64 {
            self.total.load(Ordering::Relaxed)
        }

        fn get_started_count(&self) -> u64 {
            self.total.load(Ordering::Relaxed)
        }

        fn get_error_count(&self) -> u64 {
            self.total.load(Ordering::Relaxed)
        }

        fn get_abort_count(&self) -> u64 {
            self.total.load(Ordering::Relaxed)
        }

        fn get_recovery_count(&self) -> u64 {
            self.total.load(Ordering::Relaxed)
        }
    }

    impl MutableRetryStatistics for TotalStatistics {
        fn increment_started_count(&self) {
            self.total.fetch_add(1, Ordering::Relaxed);
        }

        fn increment_complete_count(&self) {
            self.total.fetch_add(1, Ordering::Relaxed);
        }

        fn increment_recovery_count(&self) {
            self.total.fetch_add(1, Ordering::Relaxed);
        }

        fn increment_error_count(&self) {
            self.total.fetch_add(1, Ordering::Relaxed);
        }

        fn increment_abort_count(&self) {
            self.total.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn shorthand_methods_advance_the_counters() {
        let statistics = TotalStatistics::default();

        statistics.started();
        statistics.complete();
        statistics.recovery();
        statistics.error();
        statistics.abort();

        assert_eq!(statistics.get_started_count(), 5);
    }

    #[test]
    fn mutators_are_callable_through_a_trait_object() {
        let statistics: &dyn MutableRetryStatistics = &TotalStatistics::default();

        statistics.increment_started_count();
        statistics.increment_error_count();

        assert_eq!(statistics.get_error_count(), 2);
    }
}
