//! Creates the mutable statistics a repository stores.

use std::sync::Arc;

use crate::stats::mutable_retry_statistics::MutableRetryStatistics;

/// Creates the statistics object that records one retry block.
///
/// A repository holds one factory and calls it whenever it sees a name for the
/// first time, so the concrete statistics type (and its configuration) can be
/// chosen once, for the whole repository.
///
/// Implementations are shared between threads. Every call returns a fresh,
/// independent instance: the factory performs no caching, and callers that
/// need one instance per name must keep it themselves.
pub trait RetryStatisticsFactory: Send + Sync {
    /// Creates zeroed statistics for the retry block identified by `name`.
    fn create(&self, name: &str) -> Arc<dyn MutableRetryStatistics>;
}
