//! A point in time view of a retry's counters.
//!
//! The four call counters split the outcome of a guarded call by whether the
//! first attempt was enough, which is what tells an operator whether a
//! dependency is merely flaky or reliably broken:
//!
//! * a high *without retry* failure count means the dependency is failing
//!   immediately and a retry cannot help;
//! * a high *with retry* success count means the retry policy is earning its
//!   keep;
//! * a high *with retry* failure count, together with a high
//!   [`Self::number_of_retries`], means the retries mostly add load.

/// The counters a [`Retry`](crate::retry::Retry) keeps.
///
/// Every counter accumulates since the retry was created and is never reset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetrySnapshot {
    /// Calls that succeeded on the first attempt.
    pub number_of_successful_calls_without_retry: u64,
    /// Calls that succeeded after at least one retry.
    pub number_of_successful_calls_with_retry: u64,
    /// Calls that failed on the first attempt and were not retried.
    pub number_of_failed_calls_without_retry: u64,
    /// Calls that failed after at least one retry.
    pub number_of_failed_calls_with_retry: u64,
    /// Total number of extra attempts made, over all calls.
    pub number_of_retries: u64,
}

impl RetrySnapshot {
    /// Total number of calls that succeeded.
    pub fn number_of_successful_calls(&self) -> u64 {
        self.number_of_successful_calls_without_retry
            .saturating_add(self.number_of_successful_calls_with_retry)
    }

    /// Total number of calls that failed.
    pub fn number_of_failed_calls(&self) -> u64 {
        self.number_of_failed_calls_without_retry
            .saturating_add(self.number_of_failed_calls_with_retry)
    }

    /// Total number of guarded calls.
    pub fn number_of_calls(&self) -> u64 {
        self.number_of_successful_calls()
            .saturating_add(self.number_of_failed_calls())
    }
}

#[cfg(test)]
mod tests {
    use super::RetrySnapshot;

    #[test]
    fn totals_add_both_buckets() {
        let snapshot = RetrySnapshot {
            number_of_successful_calls_without_retry: 3,
            number_of_successful_calls_with_retry: 4,
            number_of_failed_calls_without_retry: 1,
            number_of_failed_calls_with_retry: 2,
            number_of_retries: 9,
        };

        assert_eq!(snapshot.number_of_successful_calls(), 7);
        assert_eq!(snapshot.number_of_failed_calls(), 3);
        assert_eq!(snapshot.number_of_calls(), 10);
    }

    #[test]
    fn an_empty_snapshot_reports_nothing() {
        let snapshot = RetrySnapshot::default();

        assert_eq!(snapshot.number_of_calls(), 0);
        assert_eq!(snapshot.number_of_retries, 0);
    }
}
