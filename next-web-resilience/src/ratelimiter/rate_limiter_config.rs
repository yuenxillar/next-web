//! Configuration of a rate limiter.
//!
//! The defaults mirror the ones a Resilience4j `RateLimiterConfig` uses, so a
//! limiter created from [`RateLimiterConfig::default`] behaves the way an
//! experienced Resilience4j user expects: 50 permissions per 500 millisecond
//! period, and a caller willing to wait 5 seconds for one.

use std::time::Duration;

use crate::clock::{SharedClock, system_clock};

/// Default number of permissions granted per refresh period.
const DEFAULT_LIMIT_FOR_PERIOD: u32 = 50;
/// Default length of one refresh period.
const DEFAULT_LIMIT_REFRESH_PERIOD: Duration = Duration::from_millis(500);
/// Default time a caller is willing to wait for a permission.
const DEFAULT_TIMEOUT_DURATION: Duration = Duration::from_secs(5);

/// The settings of a [`RateLimiter`](crate::ratelimiter::RateLimiter).
///
/// Build one with [`RateLimiterConfig::builder`]. Invalid settings are rejected
/// by [`RateLimiterConfigBuilder::build`], which panics — this is configuration
/// time, so a mistake is a programming error rather than a runtime condition to
/// recover from.
#[derive(Clone)]
pub struct RateLimiterConfig {
    limit_for_period: u32,
    limit_refresh_period: Duration,
    timeout_duration: Duration,
    clock: SharedClock,
}

impl RateLimiterConfig {
    /// Starts building a configuration from the documented defaults.
    pub fn builder() -> RateLimiterConfigBuilder {
        RateLimiterConfigBuilder::default()
    }

    /// Number of permissions granted at the start of every refresh period.
    ///
    /// Permissions that are not used before the period ends are lost; they are
    /// never carried into the next period.
    pub fn limit_for_period(&self) -> u32 {
        self.limit_for_period
    }

    /// Length of one refresh period.
    ///
    /// The period starts at the clock's epoch and repeats every
    /// `limit_refresh_period`, so cycles are aligned to that grid rather than
    /// to the first call.
    pub fn limit_refresh_period(&self) -> Duration {
        self.limit_refresh_period
    }

    /// How long a caller is willing to wait for a permission before giving up.
    ///
    /// [`RateLimiter::call`](crate::ratelimiter::RateLimiter::call) uses this
    /// budget; [`RateLimiter::acquire_permission`](crate::ratelimiter::RateLimiter::acquire_permission)
    /// ignores it because it never waits.
    pub fn timeout_duration(&self) -> Duration {
        self.timeout_duration
    }

    /// Time source the limiter reads to decide which cycle it is in.
    pub fn clock(&self) -> &SharedClock {
        &self.clock
    }
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            limit_for_period: DEFAULT_LIMIT_FOR_PERIOD,
            limit_refresh_period: DEFAULT_LIMIT_REFRESH_PERIOD,
            timeout_duration: DEFAULT_TIMEOUT_DURATION,
            clock: system_clock(),
        }
    }
}

impl std::fmt::Debug for RateLimiterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiterConfig")
            .field("limit_for_period", &self.limit_for_period)
            .field("limit_refresh_period", &self.limit_refresh_period)
            .field("timeout_duration", &self.timeout_duration)
            .finish_non_exhaustive()
    }
}

/// Builder for [`RateLimiterConfig`].
#[derive(Default)]
pub struct RateLimiterConfigBuilder {
    config: RateLimiterConfig,
}

impl RateLimiterConfigBuilder {
    /// Sets the number of permissions granted per refresh period.
    pub fn limit_for_period(mut self, limit_for_period: u32) -> Self {
        self.config.limit_for_period = limit_for_period;
        self
    }

    /// Sets the length of one refresh period.
    pub fn limit_refresh_period(mut self, limit_refresh_period: Duration) -> Self {
        self.config.limit_refresh_period = limit_refresh_period;
        self
    }

    /// Sets how long a caller is willing to wait for a permission.
    pub fn timeout_duration(mut self, timeout_duration: Duration) -> Self {
        self.config.timeout_duration = timeout_duration;
        self
    }

    /// Sets the time source the limiter reads.
    ///
    /// Install a [`ManualClock`](crate::clock::ManualClock) to drive the
    /// refresh cycles deterministically in a test.
    pub fn clock(mut self, clock: SharedClock) -> Self {
        self.config.clock = clock;
        self
    }

    /// Validates the settings and returns them.
    ///
    /// # Panics
    ///
    /// Panics when a setting is outside its documented range. This is
    /// configuration time, before any call is guarded.
    pub fn build(self) -> RateLimiterConfig {
        self.validate();
        self.config
    }

    fn validate(&self) {
        let config = &self.config;

        assert!(
            config.limit_for_period >= 1,
            "limit for period must be at least 1, got {}",
            config.limit_for_period
        );
        assert!(
            !config.limit_refresh_period.is_zero(),
            "limit refresh period must be greater than zero"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{DEFAULT_LIMIT_FOR_PERIOD, DEFAULT_LIMIT_REFRESH_PERIOD, RateLimiterConfig};
    use crate::clock::{Clock, ManualClock};

    #[test]
    fn defaults_match_resilience4j() {
        let config = RateLimiterConfig::default();

        assert_eq!(config.limit_for_period(), 50);
        assert_eq!(config.limit_for_period(), DEFAULT_LIMIT_FOR_PERIOD);
        assert_eq!(config.limit_refresh_period(), Duration::from_millis(500));
        assert_eq!(
            config.limit_refresh_period(),
            DEFAULT_LIMIT_REFRESH_PERIOD
        );
        assert_eq!(config.timeout_duration(), Duration::from_secs(5));
    }

    #[test]
    fn the_builder_starts_from_the_defaults() {
        let config = RateLimiterConfig::builder().build();

        assert_eq!(config.limit_for_period(), 50);
        assert_eq!(config.limit_refresh_period(), Duration::from_millis(500));
        assert_eq!(config.timeout_duration(), Duration::from_secs(5));
    }

    #[test]
    fn every_setting_is_overridable() {
        let clock = Arc::new(ManualClock::new());
        let config = RateLimiterConfig::builder()
            .limit_for_period(3)
            .limit_refresh_period(Duration::from_millis(250))
            .timeout_duration(Duration::from_millis(100))
            .clock(clock.clone())
            .build();

        assert_eq!(config.limit_for_period(), 3);
        assert_eq!(config.limit_refresh_period(), Duration::from_millis(250));
        assert_eq!(config.timeout_duration(), Duration::from_millis(100));
        assert_eq!(clock.monotonic_time(), Duration::ZERO);
    }

    #[test]
    fn the_debug_impl_reports_the_settings() {
        let rendered = format!("{:?}", RateLimiterConfig::default());

        assert!(rendered.contains("RateLimiterConfig"), "{rendered}");
        assert!(rendered.contains("limit_for_period: 50"), "{rendered}");
    }

    #[test]
    #[should_panic(expected = "limit for period")]
    fn a_zero_limit_is_rejected() {
        let _ = RateLimiterConfig::builder().limit_for_period(0).build();
    }

    #[test]
    #[should_panic(expected = "limit refresh period")]
    fn a_zero_refresh_period_is_rejected() {
        let _ = RateLimiterConfig::builder()
            .limit_refresh_period(Duration::ZERO)
            .build();
    }
}
