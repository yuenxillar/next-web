//! Creates the statistics a repository stores.

use std::sync::Arc;

use crate::stats::{
    exponential_average_retry_statistics::{DEFAULT_WINDOW, ExponentialAverageRetryStatistics},
    mutable_retry_statistics::MutableRetryStatistics,
    retry_statistics_factory::RetryStatisticsFactory,
};

/// Creates [`ExponentialAverageRetryStatistics`] instances that share one
/// decay window.
///
/// Every created value keeps the plain counters and, in addition, the rolling
/// averages described by [`ExponentialAverageRetryStatistics`]. The window is
/// in milliseconds and defaults to [`DEFAULT_WINDOW`].
pub struct DefaultRetryStatisticsFactory {
    window: u64,
}

impl DefaultRetryStatisticsFactory {
    /// Creates a factory that uses `window` milliseconds as the decay window.
    pub fn new(window: u64) -> Self {
        Self { window }
    }

    /// Returns the decay window in milliseconds.
    pub fn get_window(&self) -> u64 {
        self.window
    }

    /// Replaces the decay window used for statistics created from now on.
    ///
    /// Statistics that were already created keep the window they were built
    /// with; only later calls to [`RetryStatisticsFactory::create`] observe the
    /// new value.
    pub fn set_window(&mut self, window: u64) {
        self.window = window;
    }
}

impl Default for DefaultRetryStatisticsFactory {
    fn default() -> Self {
        Self {
            window: DEFAULT_WINDOW,
        }
    }
}

impl RetryStatisticsFactory for DefaultRetryStatisticsFactory {
    fn create(&self, name: &str) -> Arc<dyn MutableRetryStatistics> {
        Arc::new(ExponentialAverageRetryStatistics::with_window(
            name,
            self.window,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::DefaultRetryStatisticsFactory;
    use crate::stats::{
        exponential_average_retry_statistics::DEFAULT_WINDOW,
        retry_statistics_factory::RetryStatisticsFactory,
    };

    #[test]
    fn created_statistics_start_at_zero() {
        let factory = DefaultRetryStatisticsFactory::default();
        let statistics = factory.create("test");

        assert_eq!(factory.get_window(), DEFAULT_WINDOW);
        assert_eq!(statistics.name(), "test");
        assert_eq!(statistics.get_started_count(), 0);
        assert_eq!(statistics.get_error_count(), 0);
    }

    #[test]
    fn every_call_returns_an_independent_instance() {
        let factory = DefaultRetryStatisticsFactory::new(1_000);

        let first = factory.create("test");
        let second = factory.create("test");

        first.increment_started_count();

        assert_eq!(first.get_started_count(), 1);
        assert_eq!(second.get_started_count(), 0);
    }

    #[test]
    fn setting_the_window_updates_the_factory() {
        let mut factory = DefaultRetryStatisticsFactory::new(1_000);
        factory.set_window(2_000);

        let statistics = factory.create("test");

        assert_eq!(factory.get_window(), 2_000);
        assert_eq!(statistics.name(), "test");
    }
}
