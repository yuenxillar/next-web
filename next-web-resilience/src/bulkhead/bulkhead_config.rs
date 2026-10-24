//! Configuration of a bulkhead.
//!
//! The defaults mirror the ones a Resilience4j `SemaphoreBulkhead` uses, so a
//! bulkhead created from [`BulkheadConfig::default`] behaves the way an
//! experienced Resilience4j user expects: 25 calls may run at the same time,
//! and a caller that finds them all taken is rejected instead of waiting.

use std::time::Duration;

/// Default number of calls allowed to run at the same time.
const DEFAULT_MAX_CONCURRENT_CALLS: u32 = 25;
/// Default time a caller waits for a free slot. Zero means it does not wait at
/// all.
const DEFAULT_MAX_WAIT_DURATION: Duration = Duration::ZERO;

/// The settings of a [`Bulkhead`](crate::bulkhead::Bulkhead).
///
/// Build one with [`BulkheadConfig::builder`]. Invalid settings are rejected by
/// [`BulkheadConfigBuilder::build`], which panics — this is configuration time,
/// so a mistake is a programming error rather than a runtime condition to
/// recover from.
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    max_concurrent_calls: u32,
    max_wait_duration: Duration,
}

impl BulkheadConfig {
    /// Starts building a configuration from the documented defaults.
    pub fn builder() -> BulkheadConfigBuilder {
        BulkheadConfigBuilder::default()
    }

    /// Number of calls allowed to run at the same time.
    pub fn max_concurrent_calls(&self) -> u32 {
        self.max_concurrent_calls
    }

    /// How long a caller waits for a free slot before it is rejected.
    ///
    /// Zero, the default, rejects a caller immediately when every slot is
    /// taken.
    pub fn max_wait_duration(&self) -> Duration {
        self.max_wait_duration
    }
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            max_concurrent_calls: DEFAULT_MAX_CONCURRENT_CALLS,
            max_wait_duration: DEFAULT_MAX_WAIT_DURATION,
        }
    }
}

/// Builder for [`BulkheadConfig`].
#[derive(Debug, Default)]
pub struct BulkheadConfigBuilder {
    config: BulkheadConfig,
}

impl BulkheadConfigBuilder {
    /// Sets the number of calls allowed to run at the same time.
    pub fn max_concurrent_calls(mut self, max_concurrent_calls: u32) -> Self {
        self.config.max_concurrent_calls = max_concurrent_calls;
        self
    }

    /// Sets how long a caller waits for a free slot before it is rejected.
    ///
    /// Zero, the default, rejects a caller immediately when every slot is
    /// taken.
    pub fn max_wait_duration(mut self, max_wait_duration: Duration) -> Self {
        self.config.max_wait_duration = max_wait_duration;
        self
    }

    /// Validates the settings and returns them.
    ///
    /// # Panics
    ///
    /// Panics when `max_concurrent_calls` is zero, because such a bulkhead
    /// could never admit a call. This is configuration time, before any call is
    /// guarded.
    pub fn build(self) -> BulkheadConfig {
        assert!(
            self.config.max_concurrent_calls >= 1,
            "max concurrent calls must be at least 1, got {}",
            self.config.max_concurrent_calls
        );

        self.config
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::BulkheadConfig;

    #[test]
    fn defaults_match_resilience4j() {
        let config = BulkheadConfig::default();

        assert_eq!(config.max_concurrent_calls(), 25);
        assert_eq!(config.max_wait_duration(), Duration::ZERO);
    }

    #[test]
    fn the_builder_sets_every_setting() {
        let config = BulkheadConfig::builder()
            .max_concurrent_calls(8)
            .max_wait_duration(Duration::from_millis(250))
            .build();

        assert_eq!(config.max_concurrent_calls(), 8);
        assert_eq!(config.max_wait_duration(), Duration::from_millis(250));
    }

    #[test]
    fn configuration_can_be_cloned_and_debugged() {
        let config = BulkheadConfig::builder().max_concurrent_calls(3).build();

        let clone = config.clone();
        assert_eq!(clone.max_concurrent_calls(), 3);
        assert!(format!("{config:?}").contains("max_concurrent_calls"));
    }

    #[test]
    #[should_panic(expected = "max concurrent calls")]
    fn a_bulkhead_without_a_slot_is_rejected() {
        let _ = BulkheadConfig::builder().max_concurrent_calls(0).build();
    }
}
