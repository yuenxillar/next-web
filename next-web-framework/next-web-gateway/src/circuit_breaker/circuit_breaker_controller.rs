use super::circuit_breaker_error::CircuitBreakerError;
use super::circuit_state::CircuitState;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A circuit breaker that can be used to detect failures and encapsulate the logic of preventing a failure from constantly recurring.
///
/// The circuit breaker has three states:
/// - Closed: Requests are allowed through.
/// - Open: Requests are not allowed through.
/// - Half-Open: A limited number of requests are allowed through to test the system.
#[derive(Clone)]
pub struct CircuitBreakerController {
    failure_threshold: u32,
    // The time the fuse waits in the OPEN state.
    wait_duration_in_open_state: Duration,
    // The current state of the circuit breaker.
    state: Arc<Mutex<CircuitBreakerState>>,
}

struct CircuitBreakerState {
    state: CircuitState,
    failures: u32,
    last_failure_time: Option<Instant>,
    on_open: Option<Arc<dyn Fn() + Send + Sync>>,
    on_close: Option<Arc<dyn Fn() + Send + Sync>>,
    on_half_open: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl CircuitBreakerController {
    /// Creates a new `CircuitBreaker` with the specified failure threshold and reset timeout.
    ///
    /// # Arguments
    ///
    /// * `failure_threshold` - The number of failures that must occur before the circuit breaker opens.
    /// * `wait_duration_in_open_state` - The duration after which the circuit breaker will transition from Open to Half-Open.
    ///
    /// # Example
    ///
    /// ```
    /// use circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// ```
    pub fn new(failure_threshold: u32, wait_duration_in_open_state: Duration) -> Self {
        CircuitBreakerController {
            failure_threshold,
            wait_duration_in_open_state,
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

    /// Executes the given function within the circuit breaker.
    ///
    /// If the circuit is Open, this method will return an error without executing the function.
    /// If the circuit is Half-Open, it will allow the function to execute and transition to Closed on success.
    ///
    /// # Arguments
    ///
    /// * `f` - A function that returns a `Result`.
    ///
    /// # Returns
    ///
    /// Returns the result of the function if successful, or a `CircuitBreakerError` if the circuit is open.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::CircuitBreaker;
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// let result = cb.execute(|| {
    ///     // Simulating an operation that might fail
    ///     Ok::<_, std::io::Error>("Operation successful")
    /// });
    /// ```
    pub fn execute<F, T, E>(&self, f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + 'static,
    {
        let mut state = self.state.lock();

        match state.state {
            CircuitState::Open => {
                if let Some(last_failure_time) = state.last_failure_time {
                    if last_failure_time.elapsed() >= self.wait_duration_in_open_state {
                        state.state = CircuitState::HalfOpen;
                        if let Some(ref callback) = state.on_half_open {
                            callback();
                        }
                    } else {
                        return Err(Box::new(CircuitBreakerError::CircuitOpen));
                    }
                }
            }
            CircuitState::Closed => {}
            CircuitState::HalfOpen => {}
        }

        let current_state = state.state;
        drop(state);

        match f() {
            Ok(result) => {
                if current_state == CircuitState::HalfOpen {
                    self.handle_success();
                }
                Ok(result)
            }
            Err(e) => {
                self.handle_failure();
                Err(Box::new(e))
            }
        }
    }

    pub fn process(&self, is_ok: bool) {
        let mut state = self.state.lock();
        match state.state {
            CircuitState::Open => {
                if let Some(last_failure_time) = state.last_failure_time {
                    if last_failure_time.elapsed() >= self.wait_duration_in_open_state {
                        state.state = CircuitState::HalfOpen;
                        state.failures = 0;
                        println!("Circuit is half-open count: {}", state.failures);
                        if let Some(ref callback) = state.on_half_open {
                            callback();
                        }
                    } else {
                        return;
                    }
                }
            }
            CircuitState::Closed => {}
            CircuitState::HalfOpen => {}
        }

        drop(state);
        if !is_ok {
            self.handle_failure();
        } else {
            self.handle_success();
        }
    }

    /// Returns the current state of the circuit breaker.
    ///
    /// This method may transition the state from Open to Half-Open if the reset timeout has elapsed.
    ///
    /// # Returns
    ///
    /// Returns the current `CircuitState`.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::{CircuitBreaker, CircuitState};
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// assert_eq!(cb.state(), CircuitState::Closed);
    /// ```
    pub fn state(&self) -> CircuitState {
        let mut state = self.state.lock();
        if state.state == CircuitState::Open {
            if let Some(last_failure_time) = state.last_failure_time {
                if last_failure_time.elapsed() >= self.wait_duration_in_open_state {
                    state.state = CircuitState::HalfOpen;
                    state.failures = 0;
                    if let Some(ref callback) = state.on_half_open {
                        callback();
                    }
                }
            }
        }
        state.state
    }

    /// Handles a failure, incrementing the failure counter and potentially opening the circuit.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::CircuitBreaker;
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// cb.handle_failure();
    /// ```
    pub fn handle_failure(&self) {
        let mut state = self.state.lock();
        state.failures += 1;
        state.last_failure_time = Some(Instant::now());

        if state.failures >= self.failure_threshold {
            self.trip(&mut state);
        }
    }

    /// Handles a success, potentially closing the circuit if it was half-open.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::CircuitBreaker;
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// cb.handle_success();
    /// ```
    pub fn handle_success(&self) {
        let mut state = self.state.lock();
        state.failures = 0;
        if state.state == CircuitState::HalfOpen {
            self.reset(&mut state);
        }
    }

    fn trip(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::Open;
        if let Some(ref callback) = state.on_open {
            callback();
        }
    }

    fn reset(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::Closed;
        state.failures = 0;
        if let Some(ref callback) = state.on_close {
            callback();
        }
    }

    /// Sets a callback function to be executed when the circuit breaker opens.
    ///
    /// # Arguments
    ///
    /// * `callback` - A function to be called when the circuit opens.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::CircuitBreaker;
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// cb.set_on_open(|| {
    ///     println!("Circuit opened!");
    /// });
    /// ```
    pub fn set_on_open<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut state = self.state.lock();
        state.on_open = Some(Arc::new(callback));
    }

    /// Sets a callback function to be executed when the circuit breaker closes.
    ///
    /// # Arguments
    ///
    /// * `callback` - A function to be called when the circuit closes.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::CircuitBreaker;
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// cb.set_on_close(|| {
    ///     println!("Circuit closed!");
    /// });
    /// ```
    pub fn set_on_close<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut state = self.state.lock();
        state.on_close = Some(Arc::new(callback));
    }

    /// Sets a callback function to be executed when the circuit breaker transitions to half-open.
    ///
    /// # Arguments
    ///
    /// * `callback` - A function to be called when the circuit transitions to half-open.
    ///
    /// # Example
    ///
    /// ```
    /// # use circuit_breaker::CircuitBreaker;
    /// # use std::time::Duration;
    /// # let cb = CircuitBreaker::new(3, Duration::from_secs(60));
    /// cb.set_on_half_open(|| {
    ///     println!("Circuit is half-open!");
    /// });
    /// ```
    pub fn set_on_half_open<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut state = self.state.lock();
        state.on_half_open = Some(Arc::new(callback));
    }
}

fn test() {
    // pingora_limits::rate::RateComponents
}
