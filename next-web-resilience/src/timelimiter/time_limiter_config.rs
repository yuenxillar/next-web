//! Configuration of a time limiter.
//!
//! The defaults mirror the ones a Resilience4j `TimeLimiterConfig` uses, so a
//! limiter created from [`TimeLimiterConfig::default`] behaves the way an
//! experienced Resilience4j user expects: a guarded call may run for one second,
//! and the future is cancelled when that limit is reached.

use std::time::Duration;

/// Default limit on the duration of a guarded call.
const DEFAULT_TIMEOUT_DURATION: Duration = Duration::from_secs(1);

/// The settings of a [`TimeLimiter`](crate::timelimiter::TimeLimiter).
///
/// Build one with [`TimeLimiterConfig::builder`]. Invalid settings are rejected
/// by [`TimeLimiterConfigBuilder::build`], which panics — this is configuration
/// time, before any call is guarded, so a mistake is a programming error rather
/// than a runtime condition to recover from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeLimiterConfig {
    /// How long a guarded call may run before it is stopped.
    timeout_duration: Duration,
    /// Whether the guarded future is expected to be cancelled once the limit is
    /// reached.
    cancel_running_future: bool,
}

impl TimeLimiterConfig {
    /// Starts building a configuration from the documented defaults.
    pub fn builder() -> TimeLimiterConfigBuilder {
        TimeLimiterConfigBuilder::default()
    }

    /// How long a guarded call may run before it is stopped.
    pub fn timeout_duration(&self) -> Duration {
        self.timeout_duration
    }

    /// Whether the guarded future is cancelled once the limit is reached.
    ///
    /// [`TimeLimiter::call`](crate::timelimiter::TimeLimiter::call) implements
    /// that behaviour by dropping the future, while
    /// [`TimeLimiter::call_non_cancelling`](crate::timelimiter::TimeLimiter::call_non_cancelling)
    /// deliberately does not.
    pub fn cancel_running_future(&self) -> bool {
        self.cancel_running_future
    }
}

impl Default for TimeLimiterConfig {
    fn default() -> Self {
        Self {
            timeout_duration: DEFAULT_TIMEOUT_DURATION,
            cancel_running_future: true,
        }
    }
}

/// Builder for [`TimeLimiterConfig`].
#[derive(Debug, Default)]
pub struct TimeLimiterConfigBuilder {
    /// Configuration assembled so far.
    config: TimeLimiterConfig,
}

impl TimeLimiterConfigBuilder {
    /// Sets how long a guarded call may run before it is stopped.
    pub fn timeout_duration(mut self, timeout_duration: Duration) -> Self {
        self.config.timeout_duration = timeout_duration;
        self
    }

    /// Sets whether the guarded future is cancelled once the limit is reached.
    ///
    /// Cancelling is the sane default: a call that can no longer deliver a
    /// useful answer should stop consuming a connection, a thread or memory.
    /// Disable it only when abandoning a future would leave shared state in an
    /// inconsistent condition.
    pub fn cancel_running_future(mut self, cancel_running_future: bool) -> Self {
        self.config.cancel_running_future = cancel_running_future;
        self
    }

    /// Validates the settings and returns them.
    ///
    /// # Panics
    ///
    /// Panics when a setting is outside its documented range. This is
    /// configuration time, before any call is guarded.
    pub fn build(self) -> TimeLimiterConfig {
        self.validate();
        self.config
    }

    /// Rejects settings that would make the limiter useless.
    fn validate(&self) {
        assert!(
            !self.config.timeout_duration.is_zero(),
            "timeout duration must be greater than zero"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::TimeLimiterConfig;

    #[test]
    fn defaults_match_resilience4j() {
        let config = TimeLimiterConfig::default();

        assert_eq!(config.timeout_duration(), Duration::from_secs(1));
        assert!(config.cancel_running_future());
    }

    #[test]
    fn the_builder_overrides_every_setting() {
        let config = TimeLimiterConfig::builder()
            .timeout_duration(Duration::from_millis(250))
            .cancel_running_future(false)
            .build();

        assert_eq!(config.timeout_duration(), Duration::from_millis(250));
        assert!(!config.cancel_running_future());
    }

    #[test]
    #[should_panic(expected = "timeout duration")]
    fn a_zero_timeout_is_rejected() {
        let _ = TimeLimiterConfig::builder()
            .timeout_duration(Duration::ZERO)
            .build();
    }
}
