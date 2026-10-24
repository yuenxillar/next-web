//! A circuit breaker with closed, open and half open states.
//!
//! The breaker counts **consecutive failures** (transport errors, timeouts and
//! 5xx responses). `failure_threshold` consecutive failures open it for
//! `wait_duration_in_open_state`; after that at most `half_open_permits`
//! concurrent probes are admitted, and one success closes it again.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use super::circuit_state::CircuitState;

/// The result of one upstream attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The upstream answered with a non-5xx response.
    Success,
    /// The upstream failed: connect error, timeout or a 5xx response.
    Failure,
}

/// Why a request was rejected by the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircuitRejection {
    /// How long the caller should wait before retrying.
    pub retry_after: Duration,
}

struct CircuitBreakerState {
    state: CircuitState,
    failures: u32,
    last_failure_time: Option<Instant>,
    on_open: Option<Arc<dyn Fn() + Send + Sync>>,
    on_close: Option<Arc<dyn Fn() + Send + Sync>>,
    on_half_open: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// A circuit breaker shared by every route that references its id.
#[derive(Clone)]
pub struct CircuitBreakerController {
    enabled: bool,
    failure_threshold: u32,
    wait_duration_in_open_state: Duration,
    half_open_permits: usize,
    half_open_in_flight: Arc<AtomicUsize>,
    state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreakerController {
    /// Create an enabled breaker with a single half open probe.
    pub fn new(failure_threshold: u32, wait_duration_in_open_state: Duration) -> Self {
        Self::with_permits(true, failure_threshold, wait_duration_in_open_state, 1)
    }

    /// Create a breaker with explicit enablement and probe budget.
    pub fn with_permits(
        enabled: bool,
        failure_threshold: u32,
        wait_duration_in_open_state: Duration,
        half_open_permits: usize,
    ) -> Self {
        Self {
            enabled,
            failure_threshold: failure_threshold.max(1),
            wait_duration_in_open_state,
            half_open_permits: half_open_permits.max(1),
            half_open_in_flight: Arc::new(AtomicUsize::new(0)),
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failures: 0,
                last_failure_time: None,
                on_open: None,
                on_close: None,
                on_half_open: None,
            })),
        }
    }

    /// Whether the breaker is enabled at all; a disabled breaker never opens.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reserve the right to call the upstream.
    ///
    /// In the half open state at most `half_open_permits` callers hold a permit
    /// at the same time; a rejection means the caller should answer 503.
    pub async fn try_acquire(&self) -> Result<(), CircuitRejection> {
        if !self.enabled {
            return Ok(());
        }

        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::Closed => return Ok(()),
            CircuitState::Open => {
                let elapsed = state
                    .last_failure_time
                    .map(|time| time.elapsed())
                    .unwrap_or(self.wait_duration_in_open_state);

                if elapsed < self.wait_duration_in_open_state {
                    return Err(CircuitRejection {
                        retry_after: self.wait_duration_in_open_state.saturating_sub(elapsed),
                    });
                }

                state.state = CircuitState::HalfOpen;
                state.failures = 0;
                self.half_open_in_flight.store(0, Ordering::SeqCst);
                if let Some(callback) = state.on_half_open.as_ref() {
                    callback();
                }
            }
            CircuitState::HalfOpen => {}
        }

        drop(state);

        let in_flight = self.half_open_in_flight.fetch_add(1, Ordering::SeqCst);
        if in_flight >= self.half_open_permits {
            self.half_open_in_flight.fetch_sub(1, Ordering::SeqCst);
            return Err(CircuitRejection {
                retry_after: self.wait_duration_in_open_state,
            });
        }

        Ok(())
    }

    /// Record the outcome of one upstream attempt.
    pub async fn process(&self, outcome: Outcome) {
        if !self.enabled {
            return;
        }

        let mut state = self.state.lock().await;
        let was_half_open = state.state == CircuitState::HalfOpen;

        if was_half_open {
            self.release_probe();
        }

        match outcome {
            Outcome::Success => {
                if was_half_open {
                    self.reset(&mut state);
                } else {
                    state.failures = 0;
                }
            }
            Outcome::Failure => {
                state.failures = state.failures.saturating_add(1);
                state.last_failure_time = Some(Instant::now());

                if was_half_open || state.failures >= self.failure_threshold {
                    self.trip(&mut state);
                }
            }
        }
    }

    /// Current state, moving an expired open breaker into half open.
    pub async fn state(&self) -> CircuitState {
        if !self.enabled {
            return CircuitState::Closed;
        }

        let mut state = self.state.lock().await;
        if state.state == CircuitState::Open {
            let elapsed = state
                .last_failure_time
                .map(|time| time.elapsed())
                .unwrap_or(self.wait_duration_in_open_state);

            if elapsed >= self.wait_duration_in_open_state {
                state.state = CircuitState::HalfOpen;
                state.failures = 0;
                self.half_open_in_flight.store(0, Ordering::SeqCst);
                if let Some(callback) = state.on_half_open.as_ref() {
                    callback();
                }
            }
        }

        state.state
    }

    /// Number of failures recorded so far in the current window.
    pub async fn failures(&self) -> u32 {
        self.state.lock().await.failures
    }

    fn release_probe(&self) {
        let _ =
            self.half_open_in_flight
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                    value.checked_sub(1)
                });
    }

    fn trip(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::Open;
        if let Some(callback) = state.on_open.as_ref() {
            callback();
        }
    }

    fn reset(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::Closed;
        state.failures = 0;
        if let Some(callback) = state.on_close.as_ref() {
            callback();
        }
    }

    /// Install a callback invoked when the breaker opens.
    pub async fn set_on_open<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.state.lock().await.on_open = Some(Arc::new(callback));
    }

    /// Install a callback invoked when the breaker closes.
    pub async fn set_on_close<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.state.lock().await.on_close = Some(Arc::new(callback));
    }

    /// Install a callback invoked when the breaker starts probing.
    pub async fn set_on_half_open<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.state.lock().await.on_half_open = Some(Arc::new(callback));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{CircuitBreakerController, CircuitState, Outcome};

    fn breaker(threshold: u32) -> CircuitBreakerController {
        CircuitBreakerController::new(threshold, Duration::from_millis(50))
    }

    #[tokio::test]
    async fn five_xx_response_counts_as_failure() {
        let breaker = breaker(2);

        breaker.process(Outcome::Failure).await;
        assert_eq!(breaker.state().await, CircuitState::Closed);

        breaker.process(Outcome::Failure).await;
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn a_success_resets_the_consecutive_failure_count() {
        let breaker = breaker(2);

        breaker.process(Outcome::Failure).await;
        breaker.process(Outcome::Success).await;
        breaker.process(Outcome::Failure).await;

        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn open_breaker_rejects_with_a_retry_hint() {
        let breaker = breaker(1);
        breaker.process(Outcome::Failure).await;

        let rejection = breaker.try_acquire().await.unwrap_err();
        assert!(rejection.retry_after <= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn half_open_admits_a_single_probe() {
        let breaker = CircuitBreakerController::new(1, Duration::ZERO);
        breaker.process(Outcome::Failure).await;

        assert!(breaker.try_acquire().await.is_ok());
        assert!(breaker.try_acquire().await.is_err());
    }

    #[tokio::test]
    async fn half_open_success_closes_the_breaker() {
        let breaker = CircuitBreakerController::new(1, Duration::ZERO);
        breaker.process(Outcome::Failure).await;

        breaker.try_acquire().await.unwrap();
        breaker.process(Outcome::Success).await;

        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn half_open_failure_reopens_the_breaker() {
        let breaker = CircuitBreakerController::new(1, Duration::from_millis(50));
        breaker.process(Outcome::Failure).await;

        // Let the open window expire so the next acquire becomes the probe.
        std::thread::sleep(Duration::from_millis(60));
        breaker.try_acquire().await.unwrap();

        breaker.process(Outcome::Failure).await;

        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn disabled_breaker_never_opens() {
        let breaker = CircuitBreakerController::with_permits(false, 1, Duration::ZERO, 1);

        for _ in 0..10 {
            breaker.process(Outcome::Failure).await;
        }

        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(breaker.try_acquire().await.is_ok());
    }
}
