//! Stores statistics per retry block name.

use std::sync::Arc;

use crate::{retry_policy::RetryPolicy, stats::mutable_retry_statistics::MutableRetryStatistics};

/// Stores one statistics instance per retry block and records events against
/// it.
///
/// # Identity of a retry block
///
/// Statistics are keyed by the name of the retry policy that produced them,
/// which [`RetryPolicy`] exposes through [`next_web_core::Named`]. Keying by
/// policy *instance* would need a stable address for a borrowed trait object,
/// and `&dyn RetryPolicy` has none: the address of a temporary borrow is not an
/// identity, and trait objects cannot be compared. The name is the only stable
/// identity a policy offers, so two distinct policies that report the same name
/// share one statistics instance. Deployments that need separate accounting for
/// policies built from the same type must give them distinct names.
///
/// Implementations are shared between threads and must create the entry for a
/// name on first use, so that callers never have to register a policy first.
pub trait StatisticsRepository: Send + Sync {
    /// Returns the statistics for `policy`, creating them on first use.
    fn get_statistics(&self, policy: &dyn RetryPolicy) -> Arc<dyn MutableRetryStatistics>;

    /// Returns the statistics stored under `name`, creating them on first use.
    fn get_statistics_by_name(&self, name: &str) -> Arc<dyn MutableRetryStatistics>;

    /// Returns the statistics stored under `name`, if this name was ever seen.
    ///
    /// Unlike [`StatisticsRepository::get_statistics_by_name`] this never
    /// creates an entry, so a monitoring endpoint can look up a block without
    /// registering it.
    fn find_one(&self, name: &str) -> Option<Arc<dyn MutableRetryStatistics>>;

    /// Returns a snapshot of every stored statistics instance.
    ///
    /// The returned handles observe live counters; only the set of names is a
    /// snapshot, so instances created after the call are not included.
    fn find_all(&self) -> Vec<Arc<dyn MutableRetryStatistics>>;

    /// Records that the retry block named `name` was entered.
    fn add_started(&self, name: &str) {
        self.get_statistics_by_name(name).increment_started_count();
    }

    /// Records that the retry block named `name` observed an error.
    fn add_error(&self, name: &str) {
        self.get_statistics_by_name(name).increment_error_count();
    }

    /// Records that a recovery callback produced a result for the retry block
    /// named `name`.
    fn add_recovery(&self, name: &str) {
        self.get_statistics_by_name(name).increment_recovery_count();
    }

    /// Records that the retry block named `name` completed successfully.
    fn add_complete(&self, name: &str) {
        self.get_statistics_by_name(name).increment_complete_count();
    }

    /// Records that the retry block named `name` gave up.
    fn add_abort(&self, name: &str) {
        self.get_statistics_by_name(name).increment_abort_count();
    }
}
