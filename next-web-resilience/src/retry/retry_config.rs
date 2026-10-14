//! Configuration of a retry.
//!
//! The defaults mirror a Resilience4j `RetryConfig`: three attempts, half a
//! second between them, and every error retried.

use std::{error::Error, fmt, sync::Arc, time::Duration};

use super::interval_function::{
    BoxedIntervalFunction, DEFAULT_WAIT_DURATION, FixedIntervalFunction, IntervalFunction,
};
use crate::clock::{SharedClock, system_clock};

/// Decides whether the failure of an attempt should be retried.
///
/// The predicate receives the error type erased, so it can downcast to the
/// concrete error it cares about:
///
/// ```
/// use std::sync::Arc;
/// use next_web_resilience::retry::RetryConfig;
///
/// #[derive(Debug)]
/// struct Rejected;
/// impl std::fmt::Display for Rejected {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         f.write_str("rejected")
///     }
/// }
/// impl std::error::Error for Rejected {}
///
/// let config = RetryConfig::builder()
///     .retry_on_exception(Arc::new(|error: &(dyn std::error::Error + 'static)| {
///         // Retrying a business rejection would only repeat it.
///         error.downcast_ref::<Rejected>().is_none()
///     }))
///     .build();
/// # let _ = config;
/// ```
pub type RetryExceptionPredicate = Arc<dyn Fn(&(dyn Error + 'static)) -> bool + Send + Sync>;

/// Number of attempts a retry makes when nothing else is configured, counting
/// the first one.
const DEFAULT_MAX_ATTEMPTS: u32 = 3;

/// The settings of a [`Retry`](crate::retry::Retry).
///
/// Build one with [`RetryConfig::builder`]. Invalid settings are rejected by
/// [`RetryConfigBuilder::build`], which panics — this is configuration time, so
/// a mistake is a programming error rather than a runtime condition to recover
/// from.
#[derive(Clone)]
pub struct RetryConfig {
    max_attempts: u32,
    wait_duration: Duration,
    interval_function: BoxedIntervalFunction,
    retry_exception_predicate: RetryExceptionPredicate,
    ignore_exception_predicate: Option<RetryExceptionPredicate>,
    fail_after_max_attempts: bool,
    clock: SharedClock,
}

impl RetryConfig {
    /// Starts building a configuration from the documented defaults.
    pub fn builder() -> RetryConfigBuilder {
        RetryConfigBuilder::default()
    }

    /// Total number of attempts, counting the first one.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// The wait the default fixed interval function uses.
    ///
    /// This is the base wait; when an explicit
    /// [`interval_function`](Self::interval_function) is configured, that
    /// function decides the actual waits instead.
    pub fn wait_duration(&self) -> Duration {
        self.wait_duration
    }

    /// The function computing the wait before each further attempt.
    pub fn interval_function(&self) -> &BoxedIntervalFunction {
        &self.interval_function
    }

    /// Predicate deciding whether the failure of an attempt is retried.
    pub fn retry_exception_predicate(&self) -> &RetryExceptionPredicate {
        &self.retry_exception_predicate
    }

    /// Predicate for failures that are never retried, if one is configured.
    ///
    /// It is consulted before
    /// [`retry_exception_predicate`](Self::retry_exception_predicate), so it
    /// can carve exceptions out of an otherwise permissive policy.
    pub fn ignore_exception_predicate(&self) -> Option<&RetryExceptionPredicate> {
        self.ignore_exception_predicate.as_ref()
    }

    /// Whether a spent attempt budget is reported instead of the last failure.
    pub fn fail_after_max_attempts(&self) -> bool {
        self.fail_after_max_attempts
    }

    /// Time source the retry reads.
    pub fn clock(&self) -> &SharedClock {
        &self.clock
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            wait_duration: DEFAULT_WAIT_DURATION,
            interval_function: Arc::new(FixedIntervalFunction::new(DEFAULT_WAIT_DURATION)),
            retry_exception_predicate: Arc::new(|_| true),
            ignore_exception_predicate: None,
            fail_after_max_attempts: false,
            clock: system_clock(),
        }
    }
}

impl fmt::Debug for RetryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryConfig")
            .field("max_attempts", &self.max_attempts)
            .field("wait_duration", &self.wait_duration)
            .field("interval_function", &self.interval_function)
            .field("fail_after_max_attempts", &self.fail_after_max_attempts)
            .field(
                "has_ignore_exception_predicate",
                &self.ignore_exception_predicate.is_some(),
            )
            .finish_non_exhaustive()
    }
}

/// Builder for [`RetryConfig`].
#[derive(Default)]
pub struct RetryConfigBuilder {
    config: RetryConfig,
    interval_function_overridden: bool,
}

impl RetryConfigBuilder {
    /// Sets the total number of attempts, counting the first one.
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.config.max_attempts = max_attempts;
        self
    }

    /// Sets the wait between attempts, using a fixed interval.
    ///
    /// This is a shorthand for
    /// [`interval_function`](Self::interval_function) with a
    /// [`FixedIntervalFunction`]; configuring an interval function explicitly
    /// overrides it.
    pub fn wait_duration(mut self, wait_duration: Duration) -> Self {
        self.config.wait_duration = wait_duration;
        self
    }

    /// Sets the function computing the wait before each further attempt.
    pub fn interval_function(mut self, interval_function: impl IntervalFunction + 'static) -> Self {
        self.config.interval_function = Arc::new(interval_function);
        self.interval_function_overridden = true;
        self
    }

    /// Sets the predicate deciding whether the failure of an attempt is
    /// retried.
    pub fn retry_on_exception(mut self, retry_exception_predicate: RetryExceptionPredicate) -> Self {
        self.config.retry_exception_predicate = retry_exception_predicate;
        self
    }

    /// Sets a predicate for failures that are never retried.
    ///
    /// It is consulted before the retry predicate, so a business rejection can
    /// be excluded from an otherwise permissive policy.
    pub fn ignore_exception(
        mut self,
        ignore_exception_predicate: RetryExceptionPredicate,
    ) -> Self {
        self.config.ignore_exception_predicate = Some(ignore_exception_predicate);
        self
    }

    /// Sets whether a spent attempt budget is reported instead of the last
    /// failure.
    ///
    /// When enabled, a call that runs out of attempts fails with
    /// [`RetryError::MaxRetriesExceeded`](crate::retry::RetryError::MaxRetriesExceeded)
    /// instead of the failure of the last attempt. A result based retry that
    /// runs out of attempts then fails rather than returning the last value.
    pub fn fail_after_max_attempts(mut self, fail_after_max_attempts: bool) -> Self {
        self.config.fail_after_max_attempts = fail_after_max_attempts;
        self
    }

    /// Sets the time source the retry reads.
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
    pub fn build(mut self) -> RetryConfig {
        assert!(
            self.config.max_attempts >= 1,
            "max attempts must be at least 1, got {}",
            self.config.max_attempts
        );

        if !self.interval_function_overridden {
            self.config.interval_function =
                Arc::new(FixedIntervalFunction::new(self.config.wait_duration));
        }

        self.config
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{DEFAULT_MAX_ATTEMPTS, RetryConfig};
    use crate::retry::interval_function::{ExponentialIntervalFunction, FixedIntervalFunction};

    #[test]
    fn defaults_match_resilience4j() {
        let config = RetryConfig::default();

        assert_eq!(config.max_attempts(), DEFAULT_MAX_ATTEMPTS);
        assert_eq!(config.wait_duration(), Duration::from_millis(500));
        assert!(!config.fail_after_max_attempts());
        assert!(config.ignore_exception_predicate().is_none());
        assert_eq!(
            config.interval_function().apply(1),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn the_default_predicate_retries_every_error() {
        #[derive(Debug)]
        struct Boom;
        impl std::fmt::Display for Boom {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("boom")
            }
        }
        impl std::error::Error for Boom {}

        let config = RetryConfig::default();

        assert!((config.retry_exception_predicate())(&Boom));
    }

    #[test]
    fn the_wait_duration_feeds_the_default_interval_function() {
        let config = RetryConfig::builder()
            .wait_duration(Duration::from_millis(20))
            .build();

        assert_eq!(config.wait_duration(), Duration::from_millis(20));
        assert_eq!(config.interval_function().apply(1), Duration::from_millis(20));
        assert_eq!(config.interval_function().apply(4), Duration::from_millis(20));
    }

    #[test]
    fn an_explicit_interval_function_wins_over_the_wait_duration() {
        let config = RetryConfig::builder()
            .wait_duration(Duration::from_millis(20))
            .interval_function(ExponentialIntervalFunction::new(
                Duration::from_millis(10),
                2.0,
            ))
            .build();

        assert_eq!(config.interval_function().apply(1), Duration::from_millis(10));
        assert_eq!(config.interval_function().apply(3), Duration::from_millis(40));
    }

    #[test]
    fn a_fixed_interval_function_can_be_supplied_directly() {
        let config = RetryConfig::builder()
            .interval_function(FixedIntervalFunction::new(Duration::from_millis(7)))
            .build();

        assert_eq!(config.interval_function().apply(9), Duration::from_millis(7));
    }

    #[test]
    fn an_ignore_predicate_is_recorded() {
        let config = RetryConfig::builder()
            .ignore_exception(Arc::new(|_| true))
            .build();

        assert!(config.ignore_exception_predicate().is_some());
    }

    #[test]
    fn fail_after_max_attempts_is_recorded() {
        let config = RetryConfig::builder()
            .fail_after_max_attempts(true)
            .build();

        assert!(config.fail_after_max_attempts());
    }

    #[test]
    #[should_panic(expected = "max attempts")]
    fn zero_attempts_is_rejected() {
        let _ = RetryConfig::builder().max_attempts(0).build();
    }
}
