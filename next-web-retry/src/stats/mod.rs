//! Counters describing how retry blocks behaved.
//!
//! A *retry block* is one logical operation that a retry template may attempt
//! several times. This module provides the read-only view
//! ([`RetryStatistics`]), the mutators that a recorder uses
//! ([`MutableRetryStatistics`]), two implementations
//! ([`DefaultRetryStatistics`] with plain counters and
//! [`ExponentialAverageRetryStatistics`] with rolling averages), a factory that
//! creates the implementation ([`RetryStatisticsFactory`]), a name-keyed store
//! ([`StatisticsRepository`]), and a listener that feeds the store from retry
//! events ([`StatisticsListener`]).
//!
//! Statistics are keyed by the name of the retry policy that produced them. See
//! [`StatisticsRepository`] for what that means when two policies share a name.

/// Plain atomic counters, plus the [`RetryStatistics`] and
/// [`MutableRetryStatistics`] implementations over them.
pub mod default_retry_statistics;
/// Factory that creates new instances of the default statistics type.
pub mod default_retry_statistics_factory;
/// The name-keyed, thread-safe [`StatisticsRepository`] implementation.
pub mod default_statistics_repository;
/// Statistics that add a rolling, time-decayed view to the plain counters.
pub mod exponential_average_retry_statistics;
/// The mutation view over retry statistics.
pub mod mutable_retry_statistics;
/// The read-only view over retry statistics.
pub mod retry_statistics;
/// Creation of the statistics object used for one retry block.
pub mod retry_statistics_factory;
/// Listener that records retry-block outcomes into a [`StatisticsRepository`].
pub mod statistics_listener;
/// The store that keeps one statistics instance per retry block.
pub mod statistics_repository;

pub use default_retry_statistics::DefaultRetryStatistics;
pub use default_retry_statistics_factory::DefaultRetryStatisticsFactory;
pub use default_statistics_repository::DefaultStatisticsRepository;
pub use exponential_average_retry_statistics::{DEFAULT_WINDOW, ExponentialAverageRetryStatistics};
pub use mutable_retry_statistics::MutableRetryStatistics;
pub use retry_statistics::RetryStatistics;
pub use retry_statistics_factory::RetryStatisticsFactory;
pub use statistics_listener::StatisticsListener;
pub use statistics_repository::StatisticsRepository;
