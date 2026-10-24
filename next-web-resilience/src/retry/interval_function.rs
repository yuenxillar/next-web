//! Interval functions deciding how long a retry waits before the next attempt.
//!
//! A fixed interval is enough to ride out a brief network hiccup. An
//! exponential one is better when a dependency is genuinely overloaded:
//! backing off further on every attempt gives it room to recover instead of
//! hammering it. Adding jitter avoids the case where every client retries in
//! lockstep and re-creates the spike that caused the failure.

use std::{fmt, sync::Arc, time::Duration};

/// How long a retry waits when nothing else is configured.
pub const DEFAULT_WAIT_DURATION: Duration = Duration::from_millis(500);

/// Multiplier of the exponential interval function when nothing else is
/// configured.
pub const DEFAULT_MULTIPLIER: f64 = 1.5;

/// Computes how long to wait before the next attempt.
///
/// Attempts are numbered from one, so `apply(1)` is the wait that follows the
/// first failure and precedes the second attempt.
pub trait IntervalFunction: Send + Sync + fmt::Debug {
    /// Returns the wait that follows attempt number `attempt`.
    fn apply(&self, attempt: u32) -> Duration;
}

/// A shared, type erased [`IntervalFunction`].
pub type BoxedIntervalFunction = Arc<dyn IntervalFunction>;

/// Waits the same amount of time after every failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedIntervalFunction {
    /// Wait applied after every attempt.
    interval: Duration,
}

impl FixedIntervalFunction {
    /// Creates a function that always waits `interval`.
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }

    /// Returns the constant wait.
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

impl Default for FixedIntervalFunction {
    fn default() -> Self {
        Self::new(DEFAULT_WAIT_DURATION)
    }
}

impl IntervalFunction for FixedIntervalFunction {
    fn apply(&self, _attempt: u32) -> Duration {
        self.interval
    }
}

/// Grows the wait geometrically, optionally capped and jittered.
///
/// The wait after attempt *n* is `initial_interval * multiplier^(n - 1)`, so
/// the default of 500 milliseconds and a multiplier of 1.5 produces 500 ms,
/// 750 ms, 1.1 s and so on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExponentialIntervalFunction {
    /// Wait after the first attempt.
    initial_interval: Duration,
    /// Factor the wait grows by on every further attempt.
    multiplier: f64,
    /// Upper bound of the wait, when one is configured.
    max_interval: Option<Duration>,
    /// Fraction by which each wait is randomly spread, in `0.0..=1.0`.
    randomization_factor: f64,
}

impl ExponentialIntervalFunction {
    /// Creates a function growing from `initial_interval` by `multiplier`.
    ///
    /// A multiplier below one would shrink the wait on every attempt, which is
    /// never what a retry wants, so it is raised to one.
    pub fn new(initial_interval: Duration, multiplier: f64) -> Self {
        let multiplier = if multiplier.is_finite() && multiplier >= 1.0 {
            multiplier
        } else {
            1.0
        };

        Self {
            initial_interval,
            multiplier,
            max_interval: None,
            randomization_factor: 0.0,
        }
    }

    /// Caps the wait at `max_interval`.
    pub fn with_max_interval(mut self, max_interval: Duration) -> Self {
        self.max_interval = Some(max_interval);
        self
    }

    /// Spreads every wait by `factor`, in `0.0..=1.0`.
    ///
    /// A factor of 0.5 makes each wait land uniformly in the band from half to
    /// one and a half times its nominal length. Values outside the range are
    /// clamped.
    pub fn with_randomization(mut self, factor: f64) -> Self {
        self.randomization_factor = if factor.is_finite() { factor } else { 0.0 };
        self
    }

    /// Returns the wait applied after the first attempt.
    pub fn initial_interval(&self) -> Duration {
        self.initial_interval
    }

    /// Returns the factor the wait grows by.
    pub fn multiplier(&self) -> f64 {
        self.multiplier
    }

    /// Returns the configured upper bound, if any.
    pub fn max_interval(&self) -> Option<Duration> {
        self.max_interval
    }

    /// Returns the configured jitter factor.
    pub fn randomization_factor(&self) -> f64 {
        self.randomization_factor
    }

    /// Spreads `seconds` by the configured jitter factor.
    fn randomize(&self, seconds: f64) -> f64 {
        let factor = self.randomization_factor.clamp(0.0, 1.0);
        if factor == 0.0 || !seconds.is_finite() {
            return seconds;
        }

        let delta = seconds * factor;
        let lower = (seconds - delta).max(0.0);
        let upper = seconds + delta;
        if upper <= lower {
            return seconds;
        }

        lower + rand::random::<f64>() * (upper - lower)
    }
}

impl Default for ExponentialIntervalFunction {
    fn default() -> Self {
        Self::new(DEFAULT_WAIT_DURATION, DEFAULT_MULTIPLIER)
    }
}

impl IntervalFunction for ExponentialIntervalFunction {
    fn apply(&self, attempt: u32) -> Duration {
        // `powi` takes an `i32`; a saturated exponent keeps an absurd attempt
        // count from wrapping around into a tiny wait.
        let exponent = i32::try_from(attempt.saturating_sub(1)).unwrap_or(i32::MAX);
        let mut seconds = self.initial_interval.as_secs_f64() * self.multiplier.powi(exponent);

        if let Some(max_interval) = self.max_interval {
            seconds = seconds.min(max_interval.as_secs_f64());
        }

        duration_from_secs_f64(self.randomize(seconds))
    }
}

/// Converts a floating point number of seconds into a [`Duration`] without
/// panicking on a negative, non-finite or out of range value.
///
/// A wait that overflows is saturated rather than dropped: turning it into zero
/// would make the retry spin with no delay at all.
fn duration_from_secs_f64(seconds: f64) -> Duration {
    if seconds.is_nan() || seconds <= 0.0 {
        return Duration::ZERO;
    }
    if seconds.is_infinite() {
        return Duration::MAX;
    }

    Duration::try_from_secs_f64(seconds).unwrap_or(Duration::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        DEFAULT_MULTIPLIER, DEFAULT_WAIT_DURATION, ExponentialIntervalFunction,
        FixedIntervalFunction, IntervalFunction,
    };

    #[test]
    fn a_fixed_interval_never_changes() {
        let function = FixedIntervalFunction::new(Duration::from_millis(250));

        for attempt in 1..=5 {
            assert_eq!(function.apply(attempt), Duration::from_millis(250));
        }
    }

    #[test]
    fn the_default_fixed_interval_is_500_milliseconds() {
        assert_eq!(FixedIntervalFunction::default().interval(), DEFAULT_WAIT_DURATION);
    }

    #[test]
    fn an_exponential_interval_grows_by_the_multiplier() {
        let function =
            ExponentialIntervalFunction::new(Duration::from_millis(100), 2.0);

        assert_eq!(function.apply(1), Duration::from_millis(100));
        assert_eq!(function.apply(2), Duration::from_millis(200));
        assert_eq!(function.apply(3), Duration::from_millis(400));
        assert_eq!(function.apply(4), Duration::from_millis(800));
    }

    #[test]
    fn the_default_exponential_interval_matches_resilience4j() {
        let function = ExponentialIntervalFunction::default();

        assert_eq!(function.initial_interval(), DEFAULT_WAIT_DURATION);
        assert_eq!(function.multiplier(), DEFAULT_MULTIPLIER);
        assert_eq!(function.apply(1), DEFAULT_WAIT_DURATION);
        assert_eq!(function.apply(2), Duration::from_millis(750));
    }

    #[test]
    fn the_maximum_interval_caps_the_growth() {
        let function = ExponentialIntervalFunction::new(Duration::from_millis(100), 10.0)
            .with_max_interval(Duration::from_millis(500));

        assert_eq!(function.apply(1), Duration::from_millis(100));
        assert_eq!(function.apply(2), Duration::from_millis(500));
        assert_eq!(function.apply(9), Duration::from_millis(500));
    }

    #[test]
    fn a_multiplier_below_one_is_raised_to_one() {
        let function = ExponentialIntervalFunction::new(Duration::from_millis(100), 0.5);

        assert_eq!(function.multiplier(), 1.0);
        assert_eq!(function.apply(3), Duration::from_millis(100));
    }

    #[test]
    fn randomization_stays_within_the_configured_band() {
        let base = Duration::from_millis(1000);
        let function = ExponentialIntervalFunction::new(base, 1.0).with_randomization(0.5);

        for _ in 0..200 {
            let waited = function.apply(1);
            assert!(
                waited >= Duration::from_millis(500) && waited <= Duration::from_millis(1500),
                "wait {waited:?} left the expected band"
            );
        }
    }

    #[test]
    fn zero_randomization_is_deterministic() {
        let function = ExponentialIntervalFunction::new(Duration::from_millis(100), 2.0);

        assert_eq!(function.randomization_factor(), 0.0);
        assert_eq!(function.apply(2), Duration::from_millis(200));
    }

    #[test]
    fn a_huge_attempt_count_does_not_overflow() {
        let function = ExponentialIntervalFunction::new(Duration::from_secs(1), 2.0);

        // Must saturate rather than wrap around to a short wait or panic.
        assert!(function.apply(u32::MAX) >= Duration::from_secs(1));
    }

    #[test]
    fn a_non_finite_multiplier_is_rejected() {
        let function = ExponentialIntervalFunction::new(Duration::from_millis(100), f64::NAN);

        assert_eq!(function.multiplier(), 1.0);
    }

    #[test]
    fn a_zero_initial_interval_yields_a_zero_wait() {
        let function = ExponentialIntervalFunction::new(Duration::ZERO, 2.0);

        assert_eq!(function.apply(1), Duration::ZERO);
    }
}
