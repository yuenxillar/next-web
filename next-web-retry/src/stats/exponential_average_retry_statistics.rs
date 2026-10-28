//! Statistics with rolling exponential averages.

use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use next_web_core::Named;

use crate::stats::{
    default_retry_statistics::DefaultRetryStatistics,
    mutable_retry_statistics::MutableRetryStatistics, retry_statistics::RetryStatistics,
};

/// Decay window, in milliseconds, used when none is configured.
pub const DEFAULT_WINDOW: u64 = 15_000;

/// Source of the current time in milliseconds.
///
/// It is a trait object rather than a plain function pointer so tests can
/// supply a deterministic clock without patching the system time.
type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Returns the current wall-clock time as milliseconds since the Unix epoch.
///
/// A clock that is set before the epoch, or a duration that does not fit in
/// `u64` milliseconds, degrades to zero rather than failing: the value is only
/// used to compute elapsed time between two samples.
fn system_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .unwrap_or_default()
}

/// One exponentially weighted event average.
///
/// The average carries a single value that decays by `exp(-alpha * elapsed)`
/// for every millisecond that passes, and a single `increment` adds one to the
/// decayed value. `alpha` is `1 / window`, so after one full window an event
/// has decayed to roughly 37% of its original weight.
struct ExponentialAverage {
    alpha: f64,
    clock: Clock,
    state: Mutex<AverageState>,
}

/// The pair of fields that must change together.
///
/// The pair cannot be packed into one atomic word (`f64` plus `u64` is 128
/// bits and `AtomicU128` is not stable), so a `std::sync::Mutex` guards it.
/// The critical section is a clock read, at most two floating point
/// operations, and two field writes; it is never held across an `.await`
/// because none of these methods is asynchronous. The mutex is poisoned only
/// if a panic occurs inside that tiny section, in which case the stored average
/// is still a valid number and is reused.
struct AverageState {
    last_time: u64,
    value: f64,
}

impl ExponentialAverage {
    fn new(window: u64, clock: Clock) -> Self {
        let last_time = clock();
        Self {
            alpha: 1.0 / window as f64,
            clock,
            state: Mutex::new(AverageState {
                last_time,
                value: 0.0,
            }),
        }
    }

    /// Decays the stored value to `now` and adds one event to it.
    fn increment(&self) {
        let now = (self.clock)();
        let mut state = self.lock_state();
        let decayed = self.decay(&state, now);
        state.value = decayed + 1.0;
        state.last_time = now;
    }

    /// Returns the stored value decayed to the current time.
    ///
    /// Reading does not advance `last_time`: repeated reads at the same
    /// instant return the same value, and each read independently accounts for
    /// the time elapsed since the last increment.
    fn value(&self) -> f64 {
        let now = (self.clock)();
        let state = self.lock_state();
        self.decay(&state, now)
    }

    fn decay(&self, state: &AverageState, now: u64) -> f64 {
        let elapsed = now.saturating_sub(state.last_time);
        state.value * (-self.alpha * elapsed as f64).exp()
    }

    fn lock_state(&self) -> MutexGuard<'_, AverageState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Statistics that keep, besides the plain counters, a rolling count of the
/// events observed during the last few seconds.
///
/// Every counter of [`DefaultRetryStatistics`] is maintained unchanged; in
/// addition each increment feeds an exponential moving average whose weight
/// halves every `window * ln(2)` milliseconds. The rolling getters return the
/// averages rounded to whole events, so they answer "how many of these events
/// happened recently" rather than "how many happened ever".
///
/// The decay window defaults to [`DEFAULT_WINDOW`]. A window is clamped to at
/// least one millisecond: a zero window would make the decay factor infinite
/// and poison the averages with `NaN`.
pub struct ExponentialAverageRetryStatistics {
    inner: DefaultRetryStatistics,
    clock: Clock,
    window: u64,
    started: ExponentialAverage,
    error: ExponentialAverage,
    complete: ExponentialAverage,
    recovery: ExponentialAverage,
    abort: ExponentialAverage,
}

impl ExponentialAverageRetryStatistics {
    /// Creates statistics with the default decay window.
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_window(name, DEFAULT_WINDOW)
    }

    /// Creates statistics with an explicit decay window in milliseconds.
    pub fn with_window(name: impl Into<String>, window: u64) -> Self {
        let clock: Clock = Arc::new(system_time_millis);
        Self::with_clock_and_window(name, window, clock)
    }

    fn with_clock_and_window(name: impl Into<String>, window: u64, clock: Clock) -> Self {
        let window = window.max(1);
        Self {
            inner: DefaultRetryStatistics::new(name),
            window,
            started: ExponentialAverage::new(window, Arc::clone(&clock)),
            error: ExponentialAverage::new(window, Arc::clone(&clock)),
            complete: ExponentialAverage::new(window, Arc::clone(&clock)),
            recovery: ExponentialAverage::new(window, Arc::clone(&clock)),
            abort: ExponentialAverage::new(window, Arc::clone(&clock)),
            clock,
        }
    }

    /// Returns the decay window in milliseconds.
    pub fn get_window(&self) -> u64 {
        self.window
    }

    /// Replaces the decay window and resets every rolling average.
    ///
    /// The plain counters are preserved; only the rolling view restarts, which
    /// matches the meaning of a window change: past events were weighted with
    /// the old decay and cannot be reinterpreted under the new one.
    pub fn set_window(&mut self, window: u64) {
        let window = window.max(1);
        let clock = Arc::clone(&self.clock);
        self.started = ExponentialAverage::new(window, Arc::clone(&clock));
        self.error = ExponentialAverage::new(window, Arc::clone(&clock));
        self.complete = ExponentialAverage::new(window, Arc::clone(&clock));
        self.recovery = ExponentialAverage::new(window, Arc::clone(&clock));
        self.abort = ExponentialAverage::new(window, clock);
        self.window = window;
    }

    /// Returns the rolling number of started retry blocks.
    pub fn get_rolling_started_count(&self) -> u64 {
        self.started.value().round() as u64
    }

    /// Returns the rolling number of observed errors.
    pub fn get_rolling_error_count(&self) -> u64 {
        self.error.value().round() as u64
    }

    /// Returns the rolling number of aborted retry blocks.
    pub fn get_rolling_abort_count(&self) -> u64 {
        self.abort.value().round() as u64
    }

    /// Returns the rolling number of recovered retry blocks.
    pub fn get_rolling_recovery_count(&self) -> u64 {
        self.recovery.value().round() as u64
    }

    /// Returns the rolling number of successfully completed retry blocks.
    pub fn get_rolling_complete_count(&self) -> u64 {
        self.complete.value().round() as u64
    }

    /// Returns the rolling ratio of blocks that needed recovery or were
    /// aborted to the rolling number of started blocks.
    ///
    /// The ratio is computed from the unrounded averages, so a small number of
    /// very recent events still produces a meaningful ratio. It is zero while
    /// the rolling started count rounds to zero, and is not bounded by one: if
    /// a single block is both started and aborted, the ratio is one, and a
    /// recovery that arrives after the start has decayed can push it above
    /// one.
    pub fn get_rolling_error_rate(&self) -> f64 {
        let started = self.started.value();
        if !started.is_finite() || started.round() == 0.0 {
            return 0.0;
        }
        (self.abort.value() + self.recovery.value()) / self.started.value()
    }
}

/// Gives access to the plain counter API and the [`std::fmt::Display`]
/// implementation of the wrapped [`DefaultRetryStatistics`].
impl Deref for ExponentialAverageRetryStatistics {
    type Target = DefaultRetryStatistics;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ExponentialAverageRetryStatistics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Named for ExponentialAverageRetryStatistics {
    fn name(&self) -> &str {
        self.inner.name()
    }
}

impl RetryStatistics for ExponentialAverageRetryStatistics {
    fn get_complete_count(&self) -> u64 {
        self.inner.get_complete_count()
    }

    fn get_started_count(&self) -> u64 {
        self.inner.get_started_count()
    }

    fn get_error_count(&self) -> u64 {
        self.inner.get_error_count()
    }

    fn get_abort_count(&self) -> u64 {
        self.inner.get_abort_count()
    }

    fn get_recovery_count(&self) -> u64 {
        self.inner.get_recovery_count()
    }
}

impl MutableRetryStatistics for ExponentialAverageRetryStatistics {
    fn increment_started_count(&self) {
        self.inner.increment_started_count();
        self.started.increment();
    }

    fn increment_complete_count(&self) {
        self.inner.increment_complete_count();
        self.complete.increment();
    }

    fn increment_recovery_count(&self) {
        self.inner.increment_recovery_count();
        self.recovery.increment();
    }

    fn increment_error_count(&self) {
        self.inner.increment_error_count();
        self.error.increment();
    }

    fn increment_abort_count(&self) {
        self.inner.increment_abort_count();
        self.abort.increment();
    }
}

impl fmt::Display for ExponentialAverageRetryStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    use next_web_core::Named;

    use super::{DEFAULT_WINDOW, ExponentialAverageRetryStatistics};
    use crate::stats::{
        mutable_retry_statistics::MutableRetryStatistics, retry_statistics::RetryStatistics,
    };

    /// Builds statistics over a manually advanced clock, expressed in
    /// milliseconds.
    fn statistics_with_clock(
        window: u64,
        clock: &Arc<AtomicU64>,
    ) -> ExponentialAverageRetryStatistics {
        let clock = Arc::clone(clock);
        let read = Arc::new(move || clock.load(Ordering::Relaxed));
        ExponentialAverageRetryStatistics::with_clock_and_window("test", window, read)
    }

    #[test]
    fn plain_counters_are_maintained_alongside_the_averages() {
        let clock = Arc::new(AtomicU64::new(0));
        let statistics = statistics_with_clock(1_000, &clock);

        statistics.increment_abort_count();
        statistics.increment_error_count();
        statistics.increment_started_count();
        statistics.increment_complete_count();
        statistics.increment_recovery_count();

        assert_eq!(statistics.name(), "test");
        assert_eq!(statistics.get_abort_count(), 1);
        assert_eq!(statistics.get_error_count(), 1);
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_complete_count(), 1);
        assert_eq!(statistics.get_recovery_count(), 1);
        assert_eq!(statistics.get_window(), 1_000);
    }

    #[test]
    fn a_single_event_rounds_up_to_one() {
        let clock = Arc::new(AtomicU64::new(0));
        let statistics = statistics_with_clock(15_000, &clock);

        statistics.increment_started_count();

        assert_eq!(statistics.get_rolling_started_count(), 1);
        assert_eq!(statistics.get_rolling_error_count(), 0);
    }

    #[test]
    fn the_average_decays_by_one_e_over_one_window() {
        let clock = Arc::new(AtomicU64::new(0));
        let statistics = statistics_with_clock(1_000, &clock);

        statistics.increment_started_count();
        assert_eq!(statistics.started.value(), 1.0);

        // One window later a single event has decayed to exp(-1).
        clock.store(1_000, Ordering::Relaxed);
        let decayed = statistics.started.value();
        assert!(
            (decayed - (-1.0_f64).exp()).abs() < 1e-12,
            "expected exp(-1), got {decayed}"
        );
        // exp(-1) is about 0.368, which rounds to zero events.
        assert_eq!(statistics.get_rolling_started_count(), 0);

        // Incrementing at that point decays the stored value and adds one.
        statistics.increment_started_count();
        let expected = (-1.0_f64).exp() + 1.0;
        assert!(
            (statistics.started.value() - expected).abs() < 1e-12,
            "expected {expected}, got {}",
            statistics.started.value()
        );
        // About 1.368, which rounds to one event.
        assert_eq!(statistics.get_rolling_started_count(), 1);

        // Two windows after the second event the total has decayed below one
        // half, so it rounds to zero again.
        clock.store(2_000, Ordering::Relaxed);
        assert_eq!(statistics.get_rolling_started_count(), 1);
        clock.store(3_000, Ordering::Relaxed);
        assert_eq!(statistics.get_rolling_started_count(), 0);
    }

    #[test]
    fn the_error_rate_is_recovery_plus_abort_over_started() {
        let clock = Arc::new(AtomicU64::new(0));
        let statistics = statistics_with_clock(1_000, &clock);

        assert_eq!(statistics.get_rolling_error_rate(), 0.0);

        statistics.increment_started_count();
        statistics.increment_abort_count();
        statistics.increment_recovery_count();

        // A single block that both started and recovered has a rate of two,
        // because both the abort and the recovery average are counted.
        assert_eq!(statistics.get_rolling_error_rate(), 2.0);

        // Decaying every average by the same factor leaves the ratio alone, as
        // long as the started average still rounds to at least one block.
        clock.store(100, Ordering::Relaxed);
        assert!((statistics.get_rolling_error_rate() - 2.0).abs() < 1e-12);

        // A full window later the started average rounds to zero, so the rate
        // is reported as zero even though the ratio of the averages is still
        // two.
        clock.store(1_000, Ordering::Relaxed);
        assert_eq!(statistics.get_rolling_error_rate(), 0.0);
    }

    #[test]
    fn the_error_rate_is_zero_when_nothing_started_recently() {
        let clock = Arc::new(AtomicU64::new(0));
        let statistics = statistics_with_clock(1_000, &clock);

        statistics.increment_abort_count();
        statistics.increment_recovery_count();

        // Only the started average gates the ratio, so a recovery without a
        // start yields zero rather than an infinite rate.
        assert_eq!(statistics.get_rolling_error_rate(), 0.0);
    }

    #[test]
    fn changing_the_window_resets_the_averages_but_keeps_the_counters() {
        let clock = Arc::new(AtomicU64::new(0));
        let mut statistics = statistics_with_clock(1_000, &clock);

        statistics.increment_started_count();
        assert_eq!(statistics.get_rolling_started_count(), 1);

        statistics.set_window(30_000);

        assert_eq!(statistics.get_window(), 30_000);
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_rolling_started_count(), 0);
    }

    #[test]
    fn a_zero_window_is_clamped_to_one_millisecond() {
        let statistics = ExponentialAverageRetryStatistics::with_window("test", 0);

        assert_eq!(statistics.get_window(), 1);
        statistics.increment_started_count();
        assert_eq!(statistics.get_rolling_started_count(), 1);
    }

    #[test]
    fn default_window_is_fifteen_seconds() {
        let statistics = ExponentialAverageRetryStatistics::new("test");

        assert_eq!(statistics.get_window(), DEFAULT_WINDOW);
        assert_eq!(DEFAULT_WINDOW, 15_000);
    }

    #[test]
    fn display_delegates_to_the_plain_statistics() {
        let statistics = ExponentialAverageRetryStatistics::new("test");
        statistics.increment_started_count();

        assert_eq!(
            statistics.to_string(),
            "DefaultRetryStatistics [name=test, startedCount=1, completeCount=0, \
             recoveryCount=0, errorCount=0, abortCount=0]"
        );
    }
}
