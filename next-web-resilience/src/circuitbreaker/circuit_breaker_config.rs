//! Configuration of a circuit breaker.
//!
//! The defaults mirror the ones a Resilience4j `CircuitBreakerConfig` uses, so
//! a breaker created from [`CircuitBreakerConfig::default`] behaves the way an
//! experienced Resilience4j user expects: a count based window of 100 calls, a
//! failure rate threshold of 50 percent, and 60 seconds in the open state.

use std::{error::Error, sync::Arc, time::Duration};

use crate::{
    clock::{SharedClock, system_clock},
    sliding_window::SlidingWindowType,
};

/// Decides whether the failure of a call counts against the failure rate.
///
/// The predicate receives the error type erased, so it can downcast to the
/// concrete error it cares about:
///
/// ```
/// use std::sync::Arc;
/// use next_web_resilience::circuitbreaker::CircuitBreakerConfig;
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
/// let config = CircuitBreakerConfig::builder()
///     .record_failure_predicate(Arc::new(|error: &(dyn std::error::Error + 'static)| {
///         // A business rejection is not a failure of the guarded resource.
///         error.downcast_ref::<Rejected>().is_none()
///     }))
///     .build();
/// # let _ = config;
/// ```
pub type ErrorPredicate = Arc<dyn Fn(&(dyn Error + 'static)) -> bool + Send + Sync>;

/// Decides whether the duration of a call makes it a slow call.
pub type SlowCallPredicate = Arc<dyn Fn(Duration) -> bool + Send + Sync>;

/// Default failure rate that opens the breaker, in percent.
const DEFAULT_FAILURE_RATE_THRESHOLD: f32 = 50.0;
/// Default slow call rate that opens the breaker, in percent.
const DEFAULT_SLOW_CALL_RATE_THRESHOLD: f32 = 100.0;
/// Default duration at which a call counts as slow.
const DEFAULT_SLOW_CALL_DURATION_THRESHOLD: Duration = Duration::from_secs(60);
/// Default number of calls held by a count based window.
const DEFAULT_SLIDING_WINDOW_SIZE: u32 = 100;
/// Default number of calls required before the rates are evaluated.
const DEFAULT_MINIMUM_NUMBER_OF_CALLS: u32 = 100;
/// Default span of a time based window.
const DEFAULT_SLIDING_WINDOW_TIME: Duration = Duration::from_secs(60);
/// Default number of buckets a time based window is split into.
const DEFAULT_TIME_WINDOW_BUCKET_COUNT: u32 = 60;
/// Default time the breaker stays open before probing the resource again.
const DEFAULT_WAIT_DURATION_IN_OPEN_STATE: Duration = Duration::from_secs(60);
/// Default number of calls allowed to probe the resource while half-open.
const DEFAULT_PERMITTED_CALLS_IN_HALF_OPEN_STATE: u32 = 10;
/// Default limit on how long the breaker may stay half-open. Zero means
/// indefinitely.
const DEFAULT_MAX_WAIT_DURATION_IN_HALF_OPEN_STATE: Duration = Duration::ZERO;

/// The settings of a [`CircuitBreaker`](crate::circuitbreaker::CircuitBreaker).
///
/// Build one with [`CircuitBreakerConfig::builder`]. Invalid settings are
/// rejected by [`CircuitBreakerConfigBuilder::build`], which panics — this is
/// configuration time, so a mistake is a programming error rather than a
/// runtime condition to recover from.
#[derive(Clone)]
pub struct CircuitBreakerConfig {
    sliding_window_type: SlidingWindowType,
    sliding_window_size: u32,
    minimum_number_of_calls: u32,
    failure_rate_threshold: f32,
    slow_call_rate_threshold: f32,
    slow_call_duration_threshold: Duration,
    sliding_window_time: Duration,
    time_window_bucket_count: u32,
    wait_duration_in_open_state: Duration,
    permitted_number_of_calls_in_half_open_state: u32,
    max_wait_duration_in_half_open_state: Duration,
    automatic_transition_from_open_to_half_open_enabled: bool,
    record_failure_predicate: ErrorPredicate,
    record_slow_call_predicate: SlowCallPredicate,
    clock: SharedClock,
}

impl CircuitBreakerConfig {
    /// Starts building a configuration from the documented defaults.
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::default()
    }

    /// How the breaker aggregates the outcome of recent calls.
    pub fn sliding_window_type(&self) -> SlidingWindowType {
        self.sliding_window_type
    }

    /// Number of calls held by a count based window.
    pub fn sliding_window_size(&self) -> u32 {
        self.sliding_window_size
    }

    /// Number of calls required before the rates are evaluated.
    pub fn minimum_number_of_calls(&self) -> u32 {
        self.minimum_number_of_calls
    }

    /// Failure rate that opens the breaker, in percent.
    pub fn failure_rate_threshold(&self) -> f32 {
        self.failure_rate_threshold
    }

    /// Slow call rate that opens the breaker, in percent.
    pub fn slow_call_rate_threshold(&self) -> f32 {
        self.slow_call_rate_threshold
    }

    /// Duration at which a call counts as slow.
    pub fn slow_call_duration_threshold(&self) -> Duration {
        self.slow_call_duration_threshold
    }

    /// Span of a time based window.
    pub fn sliding_window_time(&self) -> Duration {
        self.sliding_window_time
    }

    /// Number of buckets a time based window is split into.
    pub fn time_window_bucket_count(&self) -> u32 {
        self.time_window_bucket_count
    }

    /// Time the breaker stays open before probing the resource again.
    pub fn wait_duration_in_open_state(&self) -> Duration {
        self.wait_duration_in_open_state
    }

    /// Number of calls allowed to probe the resource while half-open.
    pub fn permitted_number_of_calls_in_half_open_state(&self) -> u32 {
        self.permitted_number_of_calls_in_half_open_state
    }

    /// Limit on how long the breaker may stay half-open.
    pub fn max_wait_duration_in_half_open_state(&self) -> Duration {
        self.max_wait_duration_in_half_open_state
    }

    /// Whether the breaker leaves the open state on its own once the wait
    /// duration elapsed.
    pub fn automatic_transition_from_open_to_half_open_enabled(&self) -> bool {
        self.automatic_transition_from_open_to_half_open_enabled
    }

    /// Predicate deciding whether the failure of a call counts.
    pub fn record_failure_predicate(&self) -> &ErrorPredicate {
        &self.record_failure_predicate
    }

    /// Predicate deciding whether a call counts as slow.
    pub fn record_slow_call_predicate(&self) -> &SlowCallPredicate {
        &self.record_slow_call_predicate
    }

    /// Time source the breaker reads.
    pub fn clock(&self) -> &SharedClock {
        &self.clock
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            sliding_window_type: SlidingWindowType::CountBased,
            sliding_window_size: DEFAULT_SLIDING_WINDOW_SIZE,
            minimum_number_of_calls: DEFAULT_MINIMUM_NUMBER_OF_CALLS,
            failure_rate_threshold: DEFAULT_FAILURE_RATE_THRESHOLD,
            slow_call_rate_threshold: DEFAULT_SLOW_CALL_RATE_THRESHOLD,
            slow_call_duration_threshold: DEFAULT_SLOW_CALL_DURATION_THRESHOLD,
            sliding_window_time: DEFAULT_SLIDING_WINDOW_TIME,
            time_window_bucket_count: DEFAULT_TIME_WINDOW_BUCKET_COUNT,
            wait_duration_in_open_state: DEFAULT_WAIT_DURATION_IN_OPEN_STATE,
            permitted_number_of_calls_in_half_open_state:
                DEFAULT_PERMITTED_CALLS_IN_HALF_OPEN_STATE,
            max_wait_duration_in_half_open_state: DEFAULT_MAX_WAIT_DURATION_IN_HALF_OPEN_STATE,
            automatic_transition_from_open_to_half_open_enabled: true,
            record_failure_predicate: Arc::new(|_| true),
            record_slow_call_predicate: Arc::new(|duration| {
                duration >= DEFAULT_SLOW_CALL_DURATION_THRESHOLD
            }),
            clock: system_clock(),
        }
    }
}

impl std::fmt::Debug for CircuitBreakerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreakerConfig")
            .field("sliding_window_type", &self.sliding_window_type)
            .field("sliding_window_size", &self.sliding_window_size)
            .field("minimum_number_of_calls", &self.minimum_number_of_calls)
            .field("failure_rate_threshold", &self.failure_rate_threshold)
            .field("slow_call_rate_threshold", &self.slow_call_rate_threshold)
            .field(
                "slow_call_duration_threshold",
                &self.slow_call_duration_threshold,
            )
            .field("sliding_window_time", &self.sliding_window_time)
            .field("time_window_bucket_count", &self.time_window_bucket_count)
            .field(
                "wait_duration_in_open_state",
                &self.wait_duration_in_open_state,
            )
            .field(
                "permitted_number_of_calls_in_half_open_state",
                &self.permitted_number_of_calls_in_half_open_state,
            )
            .field(
                "max_wait_duration_in_half_open_state",
                &self.max_wait_duration_in_half_open_state,
            )
            .field(
                "automatic_transition_from_open_to_half_open_enabled",
                &self.automatic_transition_from_open_to_half_open_enabled,
            )
            .finish_non_exhaustive()
    }
}

/// Builder for [`CircuitBreakerConfig`].
#[derive(Default)]
pub struct CircuitBreakerConfigBuilder {
    config: CircuitBreakerConfig,
    slow_call_predicate_overridden: bool,
}

impl CircuitBreakerConfigBuilder {
    /// Selects how the breaker aggregates the outcome of recent calls.
    pub fn sliding_window_type(mut self, sliding_window_type: SlidingWindowType) -> Self {
        self.config.sliding_window_type = sliding_window_type;
        self
    }

    /// Sets the number of calls held by a count based window.
    pub fn sliding_window_size(mut self, sliding_window_size: u32) -> Self {
        self.config.sliding_window_size = sliding_window_size;
        self
    }

    /// Sets the number of calls required before the rates are evaluated.
    pub fn minimum_number_of_calls(mut self, minimum_number_of_calls: u32) -> Self {
        self.config.minimum_number_of_calls = minimum_number_of_calls;
        self
    }

    /// Sets the failure rate that opens the breaker, in percent.
    pub fn failure_rate_threshold(mut self, failure_rate_threshold: f32) -> Self {
        self.config.failure_rate_threshold = failure_rate_threshold;
        self
    }

    /// Sets the slow call rate that opens the breaker, in percent.
    ///
    /// The default of 100 percent means the slow call rate alone never opens
    /// the breaker; lower it to let sustained slowness do so.
    pub fn slow_call_rate_threshold(mut self, slow_call_rate_threshold: f32) -> Self {
        self.config.slow_call_rate_threshold = slow_call_rate_threshold;
        self
    }

    /// Sets the duration at which a call counts as slow.
    ///
    /// While no explicit slow call predicate is configured, this duration also
    /// defines one.
    pub fn slow_call_duration_threshold(mut self, slow_call_duration_threshold: Duration) -> Self {
        self.config.slow_call_duration_threshold = slow_call_duration_threshold;
        self
    }

    /// Sets the span of a time based window.
    pub fn sliding_window_time(mut self, sliding_window_time: Duration) -> Self {
        self.config.sliding_window_time = sliding_window_time;
        self
    }

    /// Sets the number of buckets a time based window is split into.
    pub fn time_window_bucket_count(mut self, time_window_bucket_count: u32) -> Self {
        self.config.time_window_bucket_count = time_window_bucket_count;
        self
    }

    /// Sets the time the breaker stays open before probing the resource again.
    pub fn wait_duration_in_open_state(mut self, wait_duration_in_open_state: Duration) -> Self {
        self.config.wait_duration_in_open_state = wait_duration_in_open_state;
        self
    }

    /// Sets the number of calls allowed to probe the resource while half-open.
    pub fn permitted_number_of_calls_in_half_open_state(
        mut self,
        permitted_number_of_calls_in_half_open_state: u32,
    ) -> Self {
        self.config.permitted_number_of_calls_in_half_open_state =
            permitted_number_of_calls_in_half_open_state;
        self
    }

    /// Sets the limit on how long the breaker may stay half-open.
    ///
    /// Zero, the default, keeps the breaker half-open until the permitted calls
    /// have been made.
    pub fn max_wait_duration_in_half_open_state(
        mut self,
        max_wait_duration_in_half_open_state: Duration,
    ) -> Self {
        self.config.max_wait_duration_in_half_open_state = max_wait_duration_in_half_open_state;
        self
    }

    /// Sets whether the breaker leaves the open state on its own.
    ///
    /// When disabled, the breaker stays open until it is reset explicitly.
    pub fn automatic_transition_from_open_to_half_open_enabled(
        mut self,
        automatic_transition_from_open_to_half_open_enabled: bool,
    ) -> Self {
        self.config
            .automatic_transition_from_open_to_half_open_enabled =
            automatic_transition_from_open_to_half_open_enabled;
        self
    }

    /// Sets the predicate deciding whether the failure of a call counts.
    pub fn record_failure_predicate(mut self, record_failure_predicate: ErrorPredicate) -> Self {
        self.config.record_failure_predicate = record_failure_predicate;
        self
    }

    /// Sets the predicate deciding whether a call counts as slow.
    pub fn record_slow_call_predicate(mut self, record_slow_call_predicate: SlowCallPredicate) -> Self {
        self.config.record_slow_call_predicate = record_slow_call_predicate;
        self.slow_call_predicate_overridden = true;
        self
    }

    /// Sets the time source the breaker reads.
    ///
    /// Install a [`ManualClock`](crate::clock::ManualClock) to drive the open
    /// and half-open timeouts deterministically in a test.
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
    pub fn build(mut self) -> CircuitBreakerConfig {
        self.validate();

        if !self.slow_call_predicate_overridden {
            let threshold = self.config.slow_call_duration_threshold;
            self.config.record_slow_call_predicate =
                Arc::new(move |duration| duration >= threshold);
        }

        self.config
    }

    fn validate(&self) {
        let config = &self.config;

        assert!(
            (0.0..=100.0).contains(&config.failure_rate_threshold),
            "failure rate threshold must be within 0.0..=100.0, got {}",
            config.failure_rate_threshold
        );
        assert!(
            (0.0..=100.0).contains(&config.slow_call_rate_threshold),
            "slow call rate threshold must be within 0.0..=100.0, got {}",
            config.slow_call_rate_threshold
        );
        assert!(
            config.sliding_window_size >= 1,
            "sliding window size must be at least 1, got {}",
            config.sliding_window_size
        );
        assert!(
            config.minimum_number_of_calls >= 1,
            "minimum number of calls must be at least 1, got {}",
            config.minimum_number_of_calls
        );
        assert!(
            !config.sliding_window_time.is_zero(),
            "sliding window time must be greater than zero"
        );
        assert!(
            config.time_window_bucket_count >= 1,
            "time window bucket count must be at least 1, got {}",
            config.time_window_bucket_count
        );
        assert!(
            !config.wait_duration_in_open_state.is_zero(),
            "wait duration in open state must be greater than zero"
        );
        assert!(
            config.permitted_number_of_calls_in_half_open_state >= 1,
            "permitted number of calls in half open state must be at least 1, got {}",
            config.permitted_number_of_calls_in_half_open_state
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{CircuitBreakerConfig, ErrorPredicate};
    use crate::sliding_window::SlidingWindowType;

    #[test]
    fn defaults_match_resilience4j() {
        let config = CircuitBreakerConfig::default();

        assert_eq!(config.sliding_window_type(), SlidingWindowType::CountBased);
        assert_eq!(config.sliding_window_size(), 100);
        assert_eq!(config.minimum_number_of_calls(), 100);
        assert_eq!(config.failure_rate_threshold(), 50.0);
        assert_eq!(config.slow_call_rate_threshold(), 100.0);
        assert_eq!(config.sliding_window_time(), Duration::from_secs(60));
        assert_eq!(config.wait_duration_in_open_state(), Duration::from_secs(60));
        assert_eq!(config.permitted_number_of_calls_in_half_open_state(), 10);
        assert_eq!(
            config.max_wait_duration_in_half_open_state(),
            Duration::ZERO
        );
        assert!(config.automatic_transition_from_open_to_half_open_enabled());
    }

    #[test]
    fn the_default_slow_call_predicate_honours_the_threshold() {
        let config = CircuitBreakerConfig::builder()
            .slow_call_duration_threshold(Duration::from_millis(100))
            .build();

        let predicate = config.record_slow_call_predicate();
        assert!(predicate(Duration::from_millis(100)));
        assert!(!predicate(Duration::from_millis(99)));
    }

    #[test]
    fn an_explicit_slow_call_predicate_replaces_the_threshold_one() {
        let config = CircuitBreakerConfig::builder()
            .slow_call_duration_threshold(Duration::from_secs(60))
            .record_slow_call_predicate(Arc::new(|_| true))
            .build();

        let predicate = config.record_slow_call_predicate();
        assert!(predicate(Duration::from_millis(1)));
    }

    #[test]
    fn the_default_failure_predicate_counts_every_error() {
        #[derive(Debug)]
        struct Boom;
        impl std::fmt::Display for Boom {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("boom")
            }
        }
        impl std::error::Error for Boom {}

        let config = CircuitBreakerConfig::default();
        let predicate = config.record_failure_predicate();

        assert!(predicate(&Boom));
    }

    #[test]
    fn configuration_is_moved_into_the_breaker() {
        let config = CircuitBreakerConfig::builder()
            .sliding_window_size(10)
            .minimum_number_of_calls(5)
            .failure_rate_threshold(25.0)
            .build();

        assert_eq!(config.sliding_window_size(), 10);
        assert_eq!(config.minimum_number_of_calls(), 5);
        assert_eq!(config.failure_rate_threshold(), 25.0);
    }

    #[test]
    fn an_explicit_predicate_can_exclude_a_business_error() {
        #[derive(Debug)]
        struct Rejected;
        impl std::fmt::Display for Rejected {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("rejected")
            }
        }
        impl std::error::Error for Rejected {}

        let predicate: ErrorPredicate =
            Arc::new(|error: &(dyn std::error::Error + 'static)| {
                error.downcast_ref::<Rejected>().is_none()
            });
        let config = CircuitBreakerConfig::builder()
            .record_failure_predicate(predicate)
            .build();

        assert!(!(config.record_failure_predicate())(&Rejected));
    }

    #[test]
    #[should_panic(expected = "failure rate threshold")]
    fn an_out_of_range_failure_rate_is_rejected() {
        let _ = CircuitBreakerConfig::builder()
            .failure_rate_threshold(101.0)
            .build();
    }

    #[test]
    #[should_panic(expected = "sliding window size")]
    fn a_zero_sized_window_is_rejected() {
        let _ = CircuitBreakerConfig::builder().sliding_window_size(0).build();
    }
}
