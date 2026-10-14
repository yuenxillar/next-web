//! The circuit breaker itself.
//!
//! A circuit breaker watches the calls made to a guarded resource. When too
//! many of them fail — or take too long — it opens and rejects further calls
//! without touching the resource, giving a struggling dependency room to
//! recover. After a wait it lets a few probing calls through; if they succeed
//! it closes again, otherwise it reopens.
//!
//! ```
//! use next_web_resilience::circuitbreaker::{CircuitBreaker, CircuitBreakerConfig};
//!
//! # async fn example() {
//! let breaker = CircuitBreaker::new(
//!     "payments",
//!     CircuitBreakerConfig::builder()
//!         .sliding_window_size(10)
//!         .minimum_number_of_calls(5)
//!         .failure_rate_threshold(50.0)
//!         .build(),
//! );
//!
//! let result = breaker
//!     .call(|| async { Ok::<_, std::io::Error>("charged") })
//!     .await;
//!
//! assert!(result.is_ok());
//! # }
//! ```

use std::{error::Error, future::Future, time::Duration};

use parking_lot::Mutex;
use tokio::sync::broadcast;
use tracing::{debug, info};

use super::{
    circuit_breaker_config::{CircuitBreakerConfig, CircuitBreakerConfigBuilder},
    circuit_breaker_error::CircuitBreakerError,
    circuit_breaker_event::CircuitBreakerEvent,
    circuit_breaker_snapshot::CircuitBreakerSnapshot,
    circuit_breaker_state::CircuitBreakerState,
};
use crate::{
    error::CallNotPermittedError,
    event::EventPublisher,
    sliding_window::{CallOutcome, SlidingWindow, SlidingWindowType},
};

/// Totals that accumulate since the breaker was created.
#[derive(Debug, Default)]
struct Counters {
    /// Successful calls, including the ones whose error was ignored.
    successful_calls: u64,
    /// Failures excluded by the failure predicate.
    ignored_errors: u64,
    /// Calls rejected because the breaker was not permitting calls.
    not_permitted_calls: u64,
}

/// Mutable state guarded by the breaker's lock.
#[derive(Debug)]
struct Inner {
    /// State the breaker is in.
    state: CircuitBreakerState,
    /// Outcome of the recent calls.
    window: SlidingWindow,
    /// When the breaker entered its current state.
    entered_state_at: Option<Duration>,
    /// Probing calls still allowed while half-open.
    half_open_permits_left: u32,
    /// Totals that survive a state transition.
    counters: Counters,
}

/// A circuit breaker guarding one resource.
///
/// Wrap it in an [`Arc`](std::sync::Arc) to share it: the breaker owns the
/// counters that decide its transitions, so cloning it would split that state.
///
/// [`CircuitBreaker::call`] is the convenient entry point; when the guarded
/// operation cannot be expressed as one closure, drive the breaker by hand with
/// [`CircuitBreaker::acquire_permission`], [`CircuitBreaker::on_success`] and
/// [`CircuitBreaker::on_error`].
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Name of the guarded resource, used in logs and metrics.
    name: String,
    /// Settings of this breaker.
    config: CircuitBreakerConfig,
    /// Counters and current state.
    inner: Mutex<Inner>,
    /// Fan-out of the events this breaker emits.
    events: EventPublisher<CircuitBreakerEvent>,
}

impl CircuitBreaker {
    /// Creates a breaker for the resource called `name`.
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let window = build_window(&config);

        Self {
            name: name.into(),
            config,
            inner: Mutex::new(Inner {
                state: CircuitBreakerState::Closed,
                window,
                entered_state_at: None,
                half_open_permits_left: 0,
                counters: Counters::default(),
            }),
            events: EventPublisher::new(),
        }
    }

    /// Creates a breaker for `name` using the default settings.
    pub fn of(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Starts building a breaker for the resource called `name`.
    pub fn builder(name: impl Into<String>) -> CircuitBreakerBuilder {
        CircuitBreakerBuilder {
            name: name.into(),
            config: CircuitBreakerConfigBuilder::default(),
        }
    }

    /// Returns the name of the guarded resource.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the settings of this breaker.
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Subscribes to the events this breaker emits from now on.
    pub fn event_stream(&self) -> broadcast::Receiver<CircuitBreakerEvent> {
        self.events.subscribe()
    }

    /// Returns the current state.
    ///
    /// A breaker that has been open for longer than its wait duration moves to
    /// half-open as a side effect of this call, so the returned value reflects
    /// the passage of time rather than the last recorded transition.
    pub fn state(&self) -> CircuitBreakerState {
        let mut inner = self.inner.lock();
        self.refresh_state(&mut inner);
        inner.state
    }

    /// Returns whether the breaker currently lets a call through.
    ///
    /// This answers the question without consuming one of the half-open
    /// permits; [`Self::acquire_permission`] is the one that does.
    pub fn is_call_permitted(&self) -> bool {
        self.state().is_call_permitted()
    }

    /// Takes permission to call the guarded resource.
    ///
    /// While half-open, each successful call to this method consumes one of the
    /// permits configured by
    /// [`permitted_number_of_calls_in_half_open_state`].
    ///
    /// # Errors
    ///
    /// Returns [`CallNotPermittedError`] when the breaker is open or forced
    /// open, or when it is half-open and the probing calls are used up. Every
    /// rejection is counted and published as
    /// [`CircuitBreakerEvent::CallNotPermitted`].
    ///
    /// [`permitted_number_of_calls_in_half_open_state`]: CircuitBreakerConfigBuilder::permitted_number_of_calls_in_half_open_state
    pub fn acquire_permission(&self) -> Result<(), CallNotPermittedError> {
        let state = {
            let mut inner = self.inner.lock();
            self.refresh_state(&mut inner);

            match inner.state {
                CircuitBreakerState::Closed | CircuitBreakerState::Disabled => None,
                CircuitBreakerState::HalfOpen if inner.half_open_permits_left > 0 => {
                    inner.half_open_permits_left -= 1;
                    None
                }
                rejecting => {
                    inner.counters.not_permitted_calls += 1;
                    Some(rejecting)
                }
            }
        };

        match state {
            None => Ok(()),
            Some(state) => {
                debug!(
                    circuit_breaker = %self.name,
                    state = %state,
                    "call not permitted by circuit breaker"
                );
                self.events
                    .publish(CircuitBreakerEvent::CallNotPermitted { state });
                Err(CallNotPermittedError)
            }
        }
    }

    /// Records that a permitted call succeeded after `duration`.
    ///
    /// Calling this without a matching [`Self::acquire_permission`] — or on a
    /// breaker that is open, forced open or disabled — records nothing.
    pub fn on_success(&self, duration: Duration) {
        let slow = (self.config.record_slow_call_predicate())(duration);

        let mut inner = self.inner.lock();
        self.refresh_state(&mut inner);

        if !inner.state.records_metrics() {
            return;
        }

        inner.window.record(CallOutcome::new(false, slow));
        inner.counters.successful_calls += 1;

        self.events.publish(CircuitBreakerEvent::Success { duration });
        if slow {
            self.events.publish(CircuitBreakerEvent::SlowCall { duration });
        }

        self.evaluate_after_call(&mut inner);
    }

    /// Records that a permitted call failed with `error` after `duration`.
    ///
    /// The failure predicate decides whether the error counts against the
    /// failure rate. An excluded error is recorded as a non-failure and
    /// published as [`CircuitBreakerEvent::IgnoredError`], so business
    /// rejections cannot open the breaker.
    pub fn on_error(&self, duration: Duration, error: &(dyn Error + 'static)) {
        // Run the user supplied predicates before taking the lock: they may be
        // arbitrary code and must not be able to deadlock the breaker.
        let counts = (self.config.record_failure_predicate())(error);
        let slow = (self.config.record_slow_call_predicate())(duration);
        let message = error.to_string();

        let mut inner = self.inner.lock();
        self.refresh_state(&mut inner);

        if !inner.state.records_metrics() {
            return;
        }

        if counts {
            inner.window.record(CallOutcome::new(true, slow));
            self.events
                .publish(CircuitBreakerEvent::Error { duration, message });
            if slow {
                self.events.publish(CircuitBreakerEvent::SlowCall { duration });
            }
        } else {
            inner.window.record(CallOutcome::new(false, slow));
            inner.counters.ignored_errors += 1;
            self.events.publish(CircuitBreakerEvent::IgnoredError {
                duration,
                message,
            });
        }

        self.evaluate_after_call(&mut inner);
    }

    /// Returns the counters of this breaker.
    pub fn snapshot(&self) -> CircuitBreakerSnapshot {
        let mut inner = self.inner.lock();
        self.refresh_state(&mut inner);

        let window = inner.window.snapshot();
        CircuitBreakerSnapshot {
            state: inner.state,
            failure_rate: window.failure_rate(),
            slow_call_rate: window.slow_call_rate(),
            number_of_buffered_calls: window.total_calls,
            number_of_failed_calls: window.failed_calls,
            number_of_slow_calls: window.slow_calls,
            number_of_not_permitted_calls: inner.counters.not_permitted_calls,
            number_of_successful_calls: inner.counters.successful_calls,
            number_of_ignored_errors: inner.counters.ignored_errors,
        }
    }

    /// Calls `guarded` under the protection of this breaker.
    ///
    /// The closure runs only when the breaker permits a call. Its outcome is
    /// recorded, and its failure is returned as
    /// [`CircuitBreakerError::Inner`].
    ///
    /// # Errors
    ///
    /// Returns [`CircuitBreakerError::CallNotPermitted`] without running the
    /// closure when the breaker rejects the call, and
    /// [`CircuitBreakerError::Inner`] when the closure fails.
    pub async fn call<T, E, F, Fut>(&self, guarded: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Error + 'static,
    {
        self.acquire_permission()?;

        let started = self.config.clock().monotonic_time();
        let result = guarded().await;
        let duration = self
            .config
            .clock()
            .monotonic_time()
            .saturating_sub(started);

        match result {
            Ok(value) => {
                self.on_success(duration);
                Ok(value)
            }
            Err(error) => {
                self.on_error(duration, &error);
                Err(CircuitBreakerError::Inner(error))
            }
        }
    }

    /// Closes the breaker and clears the recorded outcomes.
    ///
    /// Use this after fixing the cause of a failure, or in a test to start from
    /// a known state.
    pub fn reset(&self) {
        let mut inner = self.inner.lock();

        if inner.state == CircuitBreakerState::Closed {
            inner.window.reset();
            inner.half_open_permits_left = 0;
            return;
        }

        self.apply_transition(&mut inner, CircuitBreakerState::Closed);
    }

    /// Moves the breaker to [`CircuitBreakerState::Closed`].
    ///
    /// Equivalent to [`Self::reset`].
    pub fn transition_to_closed_state(&self) {
        self.reset();
    }

    /// Rejects every call until the breaker is reset.
    pub fn transition_to_forced_open_state(&self) {
        let mut inner = self.inner.lock();
        self.apply_transition(&mut inner, CircuitBreakerState::ForcedOpen);
    }

    /// Lets every call through without recording its outcome.
    ///
    /// Use this to take a breaker out of service without changing the
    /// configuration of the resource it guards.
    pub fn transition_to_disabled_state(&self) {
        let mut inner = self.inner.lock();
        self.apply_transition(&mut inner, CircuitBreakerState::Disabled);
    }

    /// Opens the breaker immediately.
    pub fn transition_to_open_state(&self) {
        let mut inner = self.inner.lock();
        self.refresh_state(&mut inner);
        self.apply_transition(&mut inner, CircuitBreakerState::Open);
    }

    /// Applies the transition that the passage of time alone calls for.
    fn refresh_state(&self, inner: &mut Inner) {
        let now = self.config.clock().monotonic_time();
        let entered_at = inner.entered_state_at.unwrap_or(now);
        let elapsed = now.saturating_sub(entered_at);

        match inner.state {
            CircuitBreakerState::Open => {
                if self
                    .config
                    .automatic_transition_from_open_to_half_open_enabled()
                    && elapsed >= self.config.wait_duration_in_open_state()
                {
                    self.apply_transition(inner, CircuitBreakerState::HalfOpen);
                }
            }
            CircuitBreakerState::HalfOpen => {
                let max_wait = self.config.max_wait_duration_in_half_open_state();
                if !max_wait.is_zero() && elapsed >= max_wait {
                    self.apply_transition(inner, CircuitBreakerState::Open);
                }
            }
            CircuitBreakerState::Closed
            | CircuitBreakerState::ForcedOpen
            | CircuitBreakerState::Disabled => {}
        }
    }

    /// Moves the breaker to `to`, clearing the state that no longer applies.
    fn apply_transition(&self, inner: &mut Inner, to: CircuitBreakerState) {
        let from = inner.state;
        if from == to {
            return;
        }

        inner.state = to;
        inner.entered_state_at = Some(self.config.clock().monotonic_time());

        match to {
            CircuitBreakerState::HalfOpen => {
                // Probing starts from a clean window so that the decision to
                // close is based on the probes alone.
                inner.window.reset();
                inner.half_open_permits_left =
                    self.config.permitted_number_of_calls_in_half_open_state();
            }
            // A closed breaker starts from a clean window, and the two manual
            // overrides keep no window at all.
            CircuitBreakerState::Closed
            | CircuitBreakerState::ForcedOpen
            | CircuitBreakerState::Disabled => {
                inner.window.reset();
                inner.half_open_permits_left = 0;
            }
            CircuitBreakerState::Open => {
                // The window is kept so an operator can still see why the
                // breaker opened; it is cleared when probing begins.
                inner.half_open_permits_left = 0;
            }
        }

        info!(
            circuit_breaker = %self.name,
            from = %from,
            to = %to,
            "circuit breaker state transition"
        );
        self.events
            .publish(CircuitBreakerEvent::StateTransition { from, to });
    }

    /// Decides whether the recorded outcomes call for a transition.
    fn evaluate_after_call(&self, inner: &mut Inner) {
        let snapshot = inner.window.snapshot();

        match inner.state {
            CircuitBreakerState::Closed => {
                if snapshot.total_calls < u64::from(self.config.minimum_number_of_calls()) {
                    return;
                }

                if snapshot.failure_rate() >= self.config.failure_rate_threshold() {
                    self.events.publish(CircuitBreakerEvent::FailureRateExceeded {
                        failure_rate: snapshot.failure_rate(),
                    });
                    self.apply_transition(inner, CircuitBreakerState::Open);
                    return;
                }

                if snapshot.slow_call_rate() >= self.config.slow_call_rate_threshold() {
                    self.events
                        .publish(CircuitBreakerEvent::SlowCallRateExceeded {
                            slow_call_rate: snapshot.slow_call_rate(),
                        });
                    self.apply_transition(inner, CircuitBreakerState::Open);
                }
            }
            CircuitBreakerState::HalfOpen => {
                if snapshot.total_calls
                    < u64::from(self.config.permitted_number_of_calls_in_half_open_state())
                {
                    return;
                }

                let recovered = snapshot.failure_rate() < self.config.failure_rate_threshold();
                let next = if recovered {
                    CircuitBreakerState::Closed
                } else {
                    CircuitBreakerState::Open
                };
                self.apply_transition(inner, next);
            }
            CircuitBreakerState::Open
            | CircuitBreakerState::ForcedOpen
            | CircuitBreakerState::Disabled => {}
        }
    }
}

/// Builds a [`CircuitBreaker`] together with its configuration.
pub struct CircuitBreakerBuilder {
    name: String,
    config: CircuitBreakerConfigBuilder,
}

impl CircuitBreakerBuilder {
    /// Finishes the configuration and creates the breaker.
    ///
    /// # Panics
    ///
    /// Panics when a setting is outside its documented range.
    pub fn build(self) -> CircuitBreaker {
        CircuitBreaker::new(self.name, self.config.build())
    }

    /// Delegates to [`CircuitBreakerConfigBuilder`], so every configuration
    /// setter is available directly on the breaker builder.
    pub fn config(mut self, configure: impl FnOnce(CircuitBreakerConfigBuilder) -> CircuitBreakerConfigBuilder) -> Self {
        self.config = configure(self.config);
        self
    }
}

/// Creates the sliding window the configuration asks for.
fn build_window(config: &CircuitBreakerConfig) -> SlidingWindow {
    match config.sliding_window_type() {
        SlidingWindowType::CountBased => SlidingWindow::count_based(config.sliding_window_size()),
        SlidingWindowType::TimeBased => SlidingWindow::time_based(
            config.sliding_window_time(),
            config.time_window_bucket_count(),
            config.clock().clone(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::{io, sync::Arc, time::Duration};

    use super::CircuitBreaker;
    use crate::{
        circuitbreaker::{
            circuit_breaker_config::CircuitBreakerConfig,
            circuit_breaker_state::CircuitBreakerState,
        },
        clock::{Clock, ManualClock},
    };

    /// Builds a breaker whose windows hold `size` calls and which opens as soon
    /// as `minimum` calls have been recorded.
    fn breaker_with(clock: &ManualClock, size: u32, minimum: u32, threshold: f32) -> CircuitBreaker {
        CircuitBreaker::new(
            "test",
            CircuitBreakerConfig::builder()
                .sliding_window_size(size)
                .minimum_number_of_calls(minimum)
                .failure_rate_threshold(threshold)
                .wait_duration_in_open_state(Duration::from_secs(10))
                .permitted_number_of_calls_in_half_open_state(2)
                .clock(Arc::new(clock.clone()))
                .build(),
        )
    }

    fn failure() -> io::Error {
        io::Error::other("boom")
    }

    #[test]
    fn a_new_breaker_is_closed_and_permits_calls() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 5, 50.0);

        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
        assert!(breaker.is_call_permitted());
        assert!(breaker.acquire_permission().is_ok());
    }

    #[test]
    fn the_breaker_opens_once_the_failure_rate_crosses_the_threshold() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 4, 50.0);

        for _ in 0..4 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Open);
        assert!(breaker.acquire_permission().is_err());

        let snapshot = breaker.snapshot();
        assert_eq!(snapshot.failure_rate, 100.0);
        assert_eq!(snapshot.number_of_not_permitted_calls, 1);
        assert_eq!(snapshot.number_of_failed_calls, 4);
    }

    #[test]
    fn the_minimum_number_of_calls_delays_the_decision() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 5, 50.0);

        for _ in 0..4 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }

        // Every call failed, but only four of the five required calls happened.
        assert_eq!(breaker.state(), CircuitBreakerState::Closed);

        breaker.acquire_permission().expect("call permitted");
        breaker.on_error(Duration::from_millis(1), &failure());

        assert_eq!(breaker.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn an_open_breaker_probes_the_resource_again_after_the_wait_duration() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }
        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        clock.advance(Duration::from_secs(10));

        assert_eq!(breaker.state(), CircuitBreakerState::HalfOpen);
        assert_eq!(breaker.snapshot().number_of_buffered_calls, 0);
    }

    #[test]
    fn successful_probes_close_the_breaker_again() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }
        clock.advance(Duration::from_secs(10));

        // Two permits are configured, so two probes decide the outcome.
        for _ in 0..2 {
            breaker.acquire_permission().expect("probe permitted");
            breaker.on_success(Duration::from_millis(1));
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn half_open_permits_are_limited() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }
        clock.advance(Duration::from_secs(10));

        assert!(breaker.acquire_permission().is_ok());
        assert!(breaker.acquire_permission().is_ok());
        // Both configured probes are in flight.
        assert!(breaker.acquire_permission().is_err());
    }

    #[test]
    fn a_failing_probe_reopens_the_breaker() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }
        clock.advance(Duration::from_secs(10));

        for _ in 0..2 {
            breaker.acquire_permission().expect("probe permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn a_half_open_breaker_reopens_when_it_waits_too_long() {
        let clock = ManualClock::new();
        let breaker = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig::builder()
                .sliding_window_size(10)
                .minimum_number_of_calls(2)
                .wait_duration_in_open_state(Duration::from_secs(10))
                .permitted_number_of_calls_in_half_open_state(5)
                .max_wait_duration_in_half_open_state(Duration::from_secs(3))
                .clock(Arc::new(clock.clone()))
                .build(),
        );

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }

        clock.advance(Duration::from_secs(10));
        assert_eq!(breaker.state(), CircuitBreakerState::HalfOpen);

        clock.advance(Duration::from_secs(3));
        assert_eq!(breaker.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn an_ignored_error_never_opens_the_breaker() {
        let clock = ManualClock::new();
        let breaker = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig::builder()
                .sliding_window_size(10)
                .minimum_number_of_calls(2)
                .failure_rate_threshold(50.0)
                .record_failure_predicate(Arc::new(|_| false))
                .clock(Arc::new(clock))
                .build(),
        );

        for _ in 0..5 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
        assert_eq!(breaker.snapshot().number_of_ignored_errors, 5);
        assert_eq!(breaker.snapshot().number_of_failed_calls, 0);
    }

    #[test]
    fn slow_calls_open_the_breaker_when_the_slow_rate_is_configured() {
        let clock = ManualClock::new();
        let breaker = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig::builder()
                .sliding_window_size(10)
                .minimum_number_of_calls(2)
                .slow_call_duration_threshold(Duration::from_millis(100))
                .slow_call_rate_threshold(50.0)
                .clock(Arc::new(clock))
                .build(),
        );

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_success(Duration::from_millis(500));
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Open);
        assert_eq!(breaker.snapshot().number_of_slow_calls, 2);
    }

    #[test]
    fn forcing_the_breaker_open_rejects_calls_until_it_is_reset() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 5, 50.0);

        breaker.transition_to_forced_open_state();
        assert!(breaker.acquire_permission().is_err());

        clock.advance(Duration::from_secs(3600));
        // The wait duration must not rescue a forced open breaker.
        assert!(breaker.acquire_permission().is_err());

        breaker.reset();
        assert!(breaker.acquire_permission().is_ok());
    }

    #[test]
    fn a_disabled_breaker_permits_calls_without_recording_them() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        breaker.transition_to_disabled_state();
        for _ in 0..5 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Disabled);
        assert_eq!(breaker.snapshot().number_of_buffered_calls, 0);
        assert_eq!(breaker.snapshot().number_of_successful_calls, 0);
    }

    #[tokio::test]
    async fn call_returns_the_value_of_a_successful_call() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 5, 50.0);

        let result = breaker
            .call(|| async { Ok::<_, io::Error>(41) })
            .await
            .expect("call should succeed");

        assert_eq!(result, 41);
        assert_eq!(breaker.snapshot().number_of_successful_calls, 1);
    }

    #[tokio::test]
    async fn call_wraps_the_failure_of_the_guarded_closure() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 5, 50.0);

        let error = breaker
            .call(|| async { Err::<(), _>(failure()) })
            .await
            .expect_err("call should fail");

        assert!(!error.is_call_not_permitted());
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
    }

    #[tokio::test]
    async fn call_does_not_run_the_closure_when_the_breaker_is_open() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>(failure()) }).await;
        }

        let executed = std::sync::atomic::AtomicBool::new(false);
        let error = breaker
            .call(|| async {
                executed.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok::<_, io::Error>(())
            })
            .await
            .expect_err("call should be rejected");

        assert!(error.is_call_not_permitted());
        assert!(!executed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn events_describe_what_the_breaker_did() {
        use crate::circuitbreaker::circuit_breaker_event::CircuitBreakerEvent;

        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);
        let mut events = breaker.event_stream();

        breaker.acquire_permission().expect("call permitted");
        breaker.on_error(Duration::from_millis(1), &failure());
        breaker.acquire_permission().expect("call permitted");
        breaker.on_error(Duration::from_millis(1), &failure());

        assert!(matches!(
            events.recv().await,
            Ok(CircuitBreakerEvent::Error { .. })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(CircuitBreakerEvent::Error { .. })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(CircuitBreakerEvent::FailureRateExceeded { .. })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(CircuitBreakerEvent::StateTransition {
                to: CircuitBreakerState::Open,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn a_rejected_call_is_announced_on_the_event_stream() {
        use crate::circuitbreaker::circuit_breaker_event::CircuitBreakerEvent;

        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 5, 50.0);
        breaker.transition_to_forced_open_state();
        let mut events = breaker.event_stream();

        assert!(breaker.acquire_permission().is_err());

        assert!(matches!(
            events.recv().await,
            Ok(CircuitBreakerEvent::CallNotPermitted { .. })
        ));
    }

    #[test]
    fn the_time_based_window_ages_out_old_outcomes() {
        let clock = ManualClock::new();
        let breaker = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig::builder()
                .sliding_window_type(crate::sliding_window::SlidingWindowType::TimeBased)
                .sliding_window_time(Duration::from_secs(4))
                .time_window_bucket_count(4)
                .minimum_number_of_calls(100)
                .clock(Arc::new(clock.clone()))
                .build(),
        );

        breaker.acquire_permission().expect("call permitted");
        breaker.on_error(Duration::from_millis(1), &failure());
        assert_eq!(breaker.snapshot().number_of_failed_calls, 1);

        clock.advance(Duration::from_secs(5));
        assert_eq!(breaker.snapshot().number_of_failed_calls, 0);
    }

    #[test]
    fn the_state_keeps_moving_with_an_injected_clock() {
        let clock = ManualClock::new();
        let breaker = breaker_with(&clock, 10, 2, 50.0);

        for _ in 0..2 {
            breaker.acquire_permission().expect("call permitted");
            breaker.on_error(Duration::from_millis(1), &failure());
        }
        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        clock.advance(Duration::from_secs(9));
        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        clock.advance(Duration::from_secs(1));
        assert_eq!(breaker.state(), CircuitBreakerState::HalfOpen);
        assert_eq!(clock.monotonic_time(), Duration::from_secs(10));
    }
}
