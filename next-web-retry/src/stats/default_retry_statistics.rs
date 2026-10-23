//! The atomic counters of one retry block name.

use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use next_web_core::Named;

use crate::stats::{
    mutable_retry_statistics::MutableRetryStatistics, retry_statistics::RetryStatistics,
};

/// Statistics backed by five independent atomic counters.
///
/// Reads are lock-free and never block a writer. Counters use relaxed
/// ordering: they are independent of each other and a reader that observes one
/// counter imposes no ordering requirement on the others, which is exactly the
/// guarantee a diagnostic counter needs.
///
/// The name is fixed at construction. [`Named::name`] returns a borrow of the
/// value, so a name stored behind interior mutability could not be handed out
/// as `&str`.
pub struct DefaultRetryStatistics {
    name: String,
    started_count: AtomicU64,
    complete_count: AtomicU64,
    recovery_count: AtomicU64,
    error_count: AtomicU64,
    abort_count: AtomicU64,
}

impl DefaultRetryStatistics {
    /// Creates zeroed statistics for the retry block identified by `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            started_count: AtomicU64::new(0),
            complete_count: AtomicU64::new(0),
            recovery_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            abort_count: AtomicU64::new(0),
        }
    }
}

impl Default for DefaultRetryStatistics {
    /// Creates zeroed statistics with an empty name.
    ///
    /// Prefer [`DefaultRetryStatistics::new`] so reports can identify the retry
    /// block.
    fn default() -> Self {
        Self::new("")
    }
}

impl Named for DefaultRetryStatistics {
    fn name(&self) -> &str {
        &self.name
    }
}

impl RetryStatistics for DefaultRetryStatistics {
    fn get_complete_count(&self) -> u64 {
        self.complete_count.load(Ordering::Relaxed)
    }

    fn get_started_count(&self) -> u64 {
        self.started_count.load(Ordering::Relaxed)
    }

    fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    fn get_abort_count(&self) -> u64 {
        self.abort_count.load(Ordering::Relaxed)
    }

    fn get_recovery_count(&self) -> u64 {
        self.recovery_count.load(Ordering::Relaxed)
    }
}

impl MutableRetryStatistics for DefaultRetryStatistics {
    fn increment_started_count(&self) {
        self.started_count.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_complete_count(&self) {
        self.complete_count.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_recovery_count(&self) {
        self.recovery_count.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_error_count(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_abort_count(&self) {
        self.abort_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl fmt::Display for DefaultRetryStatistics {
    /// Formats every counter so a log line carries a complete snapshot.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = &self.name;
        let started_count = self.get_started_count();
        let complete_count = self.get_complete_count();
        let recovery_count = self.get_recovery_count();
        let error_count = self.get_error_count();
        let abort_count = self.get_abort_count();

        write!(
            f,
            "DefaultRetryStatistics [name={name}, startedCount={started_count}, \
             completeCount={complete_count}, recoveryCount={recovery_count}, \
             errorCount={error_count}, abortCount={abort_count}]"
        )
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::Named;

    use super::DefaultRetryStatistics;
    use crate::stats::{
        mutable_retry_statistics::MutableRetryStatistics, retry_statistics::RetryStatistics,
    };

    #[test]
    fn new_statistics_start_at_zero() {
        let statistics = DefaultRetryStatistics::new("test");

        assert_eq!(statistics.name(), "test");
        assert_eq!(statistics.get_started_count(), 0);
        assert_eq!(statistics.get_complete_count(), 0);
        assert_eq!(statistics.get_error_count(), 0);
        assert_eq!(statistics.get_abort_count(), 0);
        assert_eq!(statistics.get_recovery_count(), 0);
    }

    #[test]
    fn every_counter_is_incremented_independently() {
        let statistics = DefaultRetryStatistics::new("test");

        statistics.increment_started_count();
        statistics.increment_started_count();
        statistics.increment_complete_count();
        statistics.increment_error_count();
        statistics.increment_error_count();
        statistics.increment_error_count();
        statistics.increment_abort_count();
        statistics.increment_recovery_count();

        assert_eq!(statistics.get_started_count(), 2);
        assert_eq!(statistics.get_complete_count(), 1);
        assert_eq!(statistics.get_error_count(), 3);
        assert_eq!(statistics.get_abort_count(), 1);
        assert_eq!(statistics.get_recovery_count(), 1);
    }

    #[test]
    fn display_reports_the_name_and_every_counter() {
        let statistics = DefaultRetryStatistics::new("test");
        statistics.increment_started_count();
        statistics.increment_abort_count();

        let rendered = statistics.to_string();

        assert_eq!(
            rendered,
            "DefaultRetryStatistics [name=test, startedCount=1, completeCount=0, \
             recoveryCount=0, errorCount=0, abortCount=1]"
        );
    }
}
