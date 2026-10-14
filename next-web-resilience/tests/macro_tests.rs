//! Behaviour of the five attribute macros.
//!
//! These drive the macros the way an application does — on methods of a struct
//! that holds its pattern instances — and then check both what the caller saw
//! and what the pattern recorded.

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

use next_web_resilience::{
    AbortedError, BulkheadFullError, CallNotPermittedError, RateLimitExceededError, TimeoutError,
    bulkhead,
    bulkhead::{Bulkhead, BulkheadConfig},
    circuit_breaker,
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
    rate_limiter,
    ratelimiter::{RateLimiter, RateLimiterConfig},
    retry,
    retry::{Retry, RetryConfig},
    time_limiter,
    timelimiter::{TimeLimiter, TimeLimiterConfig},
};

/// The error the guarded methods return.
///
/// It implements `From` for every rejection the macros can produce, which is what
/// lets them be used without a `fallback`.
#[derive(Debug, PartialEq, Eq)]
enum ApiError {
    /// The circuit breaker refused the call.
    BreakerOpen,
    /// The limiter had no permission left.
    Throttled,
    /// The bulkhead had no slot left.
    Overloaded,
    /// The call took too long.
    TooSlow,
    /// The call ended without a result.
    Aborted,
    /// The dependency failed.
    Failed(&'static str),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BreakerOpen => f.write_str("the circuit breaker is open"),
            Self::Throttled => f.write_str("the rate limiter refused the call"),
            Self::Overloaded => f.write_str("the bulkhead is full"),
            Self::TooSlow => f.write_str("the call took too long"),
            Self::Aborted => f.write_str("the call ended without a result"),
            Self::Failed(reason) => write!(f, "the dependency failed: {reason}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<CallNotPermittedError> for ApiError {
    fn from(_: CallNotPermittedError) -> Self {
        Self::BreakerOpen
    }
}

impl From<RateLimitExceededError> for ApiError {
    fn from(_: RateLimitExceededError) -> Self {
        Self::Throttled
    }
}

impl From<BulkheadFullError> for ApiError {
    fn from(_: BulkheadFullError) -> Self {
        Self::Overloaded
    }
}

impl From<TimeoutError> for ApiError {
    fn from(_: TimeoutError) -> Self {
        Self::TooSlow
    }
}

impl From<AbortedError> for ApiError {
    fn from(_: AbortedError) -> Self {
        Self::Aborted
    }
}

/// A service holding one instance of every pattern, guarded by the macros.
struct Service {
    /// Opens only after a failure rate these tests never reach, so the breaker
    /// stays closed and its counters can be inspected.
    breaker: Arc<CircuitBreaker>,
    /// Three calls per period.
    limiter: Arc<RateLimiter>,
    /// Two calls at a time.
    bulkhead: Arc<Bulkhead>,
    /// Fifty milliseconds per call.
    time_limiter: Arc<TimeLimiter>,
    /// Three attempts.
    retry: Arc<Retry>,
    /// How many times a guarded body actually ran.
    calls: Arc<AtomicU32>,
    /// How many guarded bodies are running right now.
    in_flight: Arc<AtomicU32>,
    /// The most that ever ran at once.
    peak: Arc<AtomicU32>,
    /// Whether the guarded bodies should fail.
    failing: Arc<AtomicBool>,
}

impl Service {
    /// A service whose bulkhead waits for a permit, so its calls are admitted.
    fn new() -> Self {
        Self::with_bulkhead_wait(Duration::from_secs(2))
    }

    /// A service whose bulkhead refuses a call the moment it cannot get a permit.
    fn with_bulkhead_wait(bulkhead_wait: Duration) -> Self {
        Self {
            breaker: Arc::new(CircuitBreaker::new(
                "payments",
                CircuitBreakerConfig::builder()
                    .sliding_window_size(100)
                    .minimum_number_of_calls(100)
                    .build(),
            )),
            limiter: Arc::new(RateLimiter::new(
                "login",
                RateLimiterConfig::builder()
                    .limit_for_period(3)
                    .limit_refresh_period(Duration::from_secs(600))
                    .timeout_duration(Duration::ZERO)
                    .build(),
            )),
            bulkhead: Arc::new(Bulkhead::new(
                "reports",
                BulkheadConfig::builder()
                    .max_concurrent_calls(2)
                    .max_wait_duration(bulkhead_wait)
                    .build(),
            )),
            time_limiter: Arc::new(TimeLimiter::new(
                "search",
                TimeLimiterConfig::builder()
                    .timeout_duration(Duration::from_millis(50))
                    .build(),
            )),
            retry: Arc::new(Retry::new(
                "inventory",
                RetryConfig::builder()
                    .max_attempts(3)
                    .wait_duration(Duration::ZERO)
                    .build(),
            )),
            calls: Arc::new(AtomicU32::new(0)),
            in_flight: Arc::new(AtomicU32::new(0)),
            peak: Arc::new(AtomicU32::new(0)),
            failing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Makes the guarded bodies fail, or stop failing.
    fn fail(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    /// The outcome every guarded body but `report` produces.
    fn outcome(&self) -> Result<u32, ApiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.failing.load(Ordering::SeqCst) {
            Err(ApiError::Failed("gateway"))
        } else {
            Ok(7)
        }
    }

    /// Fails until the third attempt.
    fn flaky(&self, attempt: u32) -> Result<u32, ApiError> {
        if attempt < 3 {
            Err(ApiError::Failed("flaky"))
        } else {
            Ok(42)
        }
    }

    #[circuit_breaker(self.breaker)]
    async fn charge(&self) -> Result<u32, ApiError> {
        self.outcome()
    }

    #[rate_limiter(self.limiter)]
    async fn login(&self) -> Result<u32, ApiError> {
        self.outcome()
    }

    #[bulkhead(self.bulkhead)]
    async fn report(&self) -> Result<u32, ApiError> {
        let concurrent = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(concurrent, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(5)).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.outcome()
    }

    #[time_limiter(self.time_limiter)]
    async fn search(&self) -> Result<u32, ApiError> {
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.outcome()
    }

    #[retry(self.retry)]
    async fn stock(&self) -> Result<u32, ApiError> {
        self.outcome()
    }

    /// A body that uses `?`, which must still be retried as a whole.
    #[retry(self.retry)]
    async fn stock_with_question_mark(&self) -> Result<u32, ApiError> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let value = self.flaky(attempt)?;
        Ok(value)
    }

    #[retry(self.retry, fallback = Ok(99))]
    async fn stock_or_default(&self) -> Result<u32, ApiError> {
        self.outcome()
    }

    /// The retry is written *below* the breaker, so it wraps it: the breaker sees
    /// every attempt the retry makes.
    #[circuit_breaker(self.breaker)]
    #[retry(self.retry)]
    async fn retry_outside_breaker(&self) -> Result<u32, ApiError> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.flaky(attempt)
    }

    /// The breaker is written below, so it wraps the whole retry loop and sees a
    /// single call however many attempts are made.
    #[retry(self.retry)]
    #[circuit_breaker(self.breaker)]
    async fn breaker_outside_retry(&self) -> Result<u32, ApiError> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.flaky(attempt)
    }
}

/// A service whose breaker opens quickly, so the refusal paths can be reached.
struct Widget {
    breaker: Arc<CircuitBreaker>,
    calls: Arc<AtomicU32>,
}

impl Widget {
    fn new() -> Self {
        Self {
            breaker: Arc::new(CircuitBreaker::new(
                "widgets",
                CircuitBreakerConfig::builder()
                    .sliding_window_size(2)
                    .minimum_number_of_calls(2)
                    .failure_rate_threshold(50.0)
                    .wait_duration_in_open_state(Duration::from_secs(30))
                    .build(),
            )),
            calls: Arc::new(AtomicU32::new(0)),
        }
    }

    #[circuit_breaker(self.breaker)]
    async fn always_fails(&self) -> Result<u32, ApiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ApiError::Failed("widgets are down"))
    }

    #[circuit_breaker(self.breaker, fallback = Ok(0))]
    async fn fails_or_free(&self) -> Result<u32, ApiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ApiError::Failed("widgets are down"))
    }
}

#[tokio::test]
async fn the_circuit_breaker_records_a_success() {
    let service = Service::new();

    assert_eq!(service.charge().await.expect("the breaker is closed"), 7);

    let snapshot = service.breaker.snapshot();
    assert_eq!(snapshot.number_of_successful_calls, 1);
    assert_eq!(snapshot.number_of_buffered_calls, 1);
    assert_eq!(service.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn an_open_circuit_breaker_rejects_without_running_the_body() {
    let widget = Widget::new();

    for _ in 0..2 {
        assert_eq!(
            widget.always_fails().await.expect_err("widgets are down"),
            ApiError::Failed("widgets are down")
        );
    }
    assert_eq!(widget.breaker.state().to_string(), "OPEN");

    assert_eq!(
        widget.always_fails().await.expect_err("the breaker is open"),
        ApiError::BreakerOpen
    );
    assert_eq!(
        widget.calls.load(Ordering::SeqCst),
        2,
        "the body must not run once the breaker is open"
    );
}

#[tokio::test]
async fn a_fallback_replaces_the_rejection() {
    let widget = Widget::new();

    for _ in 0..2 {
        let _ = widget.fails_or_free().await;
    }

    // The third call is refused, and degrades instead of failing.
    assert_eq!(widget.fails_or_free().await.expect("the fallback answers"), 0);
    assert_eq!(widget.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn the_rate_limiter_refuses_past_the_quota() {
    let service = Service::new();

    for call in 1..=3 {
        assert_eq!(
            service.login().await.expect("within the quota"),
            7,
            "call {call}"
        );
    }

    assert_eq!(
        service.login().await.expect_err("the quota is spent"),
        ApiError::Throttled
    );
    assert_eq!(service.calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        service.limiter.snapshot().number_of_rejected_permissions,
        1
    );
}

#[tokio::test]
async fn a_full_bulkhead_rejects_the_call() {
    let service = Service::with_bulkhead_wait(Duration::ZERO);

    // Hold both permits by hand, so the guarded call cannot get one.
    let first = service
        .bulkhead
        .acquire_permission()
        .await
        .expect("the first slot is free");
    let second = service
        .bulkhead
        .acquire_permission()
        .await
        .expect("the second slot is free");

    assert_eq!(
        service.report().await.expect_err("the bulkhead is full"),
        ApiError::Overloaded
    );
    assert_eq!(service.calls.load(Ordering::SeqCst), 0);

    drop((first, second));
    assert_eq!(
        service.report().await.expect("the slot was released"),
        7
    );
}

#[tokio::test(start_paused = true)]
async fn the_time_limiter_abandons_a_slow_call() {
    let service = Service::new();

    assert_eq!(
        service.search().await.expect_err("the call is too slow"),
        ApiError::TooSlow
    );
    assert_eq!(service.time_limiter.snapshot().number_of_timeouts, 1);
    assert_eq!(
        service.calls.load(Ordering::SeqCst),
        0,
        "the body never got as far as reporting an outcome"
    );
}

#[tokio::test]
async fn the_retry_makes_exactly_the_configured_attempts() {
    let service = Service::new();
    service.fail(true);

    assert_eq!(
        service.stock().await.expect_err("every attempt fails"),
        ApiError::Failed("gateway")
    );
    assert_eq!(
        service.calls.load(Ordering::SeqCst),
        3,
        "the retry should have made exactly the configured number of attempts"
    );

    let snapshot = service.retry.snapshot();
    assert_eq!(snapshot.number_of_retries, 2);
    assert_eq!(snapshot.number_of_failed_calls_with_retry, 1);
}

#[tokio::test]
async fn a_question_mark_inside_the_body_does_not_escape_the_retry() {
    // The body of `stock_with_question_mark` uses `?`. If the macro spliced the
    // body in unwrapped, that `?` would leave the whole method and the retry
    // would never run a second attempt.
    let service = Service::new();

    assert_eq!(
        service
            .stock_with_question_mark()
            .await
            .expect("the third attempt succeeds"),
        42
    );
    assert_eq!(
        service.calls.load(Ordering::SeqCst),
        3,
        "the body must have run three times"
    );
    assert_eq!(service.retry.snapshot().number_of_retries, 2);
    assert_eq!(
        service.retry.snapshot().number_of_successful_calls_with_retry,
        1
    );
}

#[tokio::test]
async fn the_retry_degrades_once_the_budget_is_spent() {
    let service = Service::new();
    service.fail(true);

    assert_eq!(
        service.stock_or_default().await.expect("the fallback answers"),
        99
    );
    assert_eq!(service.calls.load(Ordering::SeqCst), 3);
    assert_eq!(service.retry.snapshot().number_of_failed_calls_with_retry, 1);
}

#[tokio::test]
async fn the_lower_attribute_ends_up_outermost() {
    // Retry written below the breaker: the retry wraps the breaker, so the
    // breaker sees each of the three attempts.
    let retry_outer = Service::new();
    retry_outer.fail(true);
    let _ = retry_outer.retry_outside_breaker().await;
    assert_eq!(
        retry_outer.breaker.snapshot().number_of_buffered_calls,
        3,
        "with the retry outside, the breaker should have seen all three attempts"
    );

    // Breaker written below the retry: the breaker wraps the retry loop, so it
    // sees a single call however many attempts the retry makes.
    let breaker_outer = Service::new();
    breaker_outer.fail(true);
    let _ = breaker_outer.breaker_outside_retry().await;
    assert_eq!(
        breaker_outer.breaker.snapshot().number_of_buffered_calls,
        1,
        "with the breaker outside, it should have seen one call"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_bulkhead_bounds_concurrency_across_tasks() {
    let service = Arc::new(Service::new());

    let mut tasks = Vec::new();
    for _ in 0..6 {
        let service = Arc::clone(&service);
        tasks.push(tokio::spawn(async move { service.report().await.is_ok() }));
    }

    let mut completed = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            completed += 1;
        }
    }

    // Six calls through a bulkhead of two: every one is admitted, because the
    // bulkhead waits for a permit for up to two seconds.
    assert_eq!(completed, 6);
    assert_eq!(service.bulkhead.snapshot().number_of_rejected_calls, 0);

    // The peak is measured inside the guarded body, so it really did go through
    // the macro's permit.
    let peak = service.peak.load(Ordering::SeqCst);
    assert!(peak <= 2, "the bulkhead let {peak} calls run at once");
    assert_eq!(service.in_flight.load(Ordering::SeqCst), 0);
    assert_eq!(service.bulkhead.available_concurrent_calls(), 2);
}

#[tokio::test]
async fn a_name_resolves_through_the_process_registry() {
    use next_web_resilience::registry::global_registry;

    struct Named;

    impl Named {
        #[rate_limiter(name = "macro_named_login")]
        async fn login(&self) -> Result<u32, ApiError> {
            Ok(1)
        }
    }

    // Fifty permissions per five hundred milliseconds is the default, so a
    // handful of calls all fit and the instance is the registry's.
    let named = Named;
    for _ in 0..6 {
        assert_eq!(named.login().await.expect("within the default quota"), 1);
    }

    let limiter = global_registry()
        .find_rate_limiter("macro_named_login")
        .expect("the macro should have registered the limiter");
    assert_eq!(limiter.snapshot().number_of_acquired_permissions, 6);
}

/// A guarded function keeps its visibility and its generics, and the attributes
/// stacked beneath it survive.
pub mod signature {
    use std::fmt;

    use next_web_resilience::{
        CallNotPermittedError, circuit_breaker, circuitbreaker::CircuitBreaker,
    };

    /// The error the guarded method returns.
    #[derive(Debug)]
    pub struct Boom;

    impl fmt::Display for Boom {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("boom")
        }
    }

    impl std::error::Error for Boom {}

    // Without a `fallback`, the macro converts a rejection into the function's
    // own error, so this is what the documented contract asks for.
    impl From<CallNotPermittedError> for Boom {
        fn from(_: CallNotPermittedError) -> Self {
            Self
        }
    }

    /// A type whose guarded method is deliberately public and generic.
    pub struct Generic {
        /// The breaker the macro resolves through `self.breaker`.
        breaker: CircuitBreaker,
    }

    impl Default for Generic {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Generic {
        /// Creates the type with an unconfigured breaker.
        pub fn new() -> Self {
            Self {
                breaker: CircuitBreaker::of("generic"),
            }
        }

        /// Public, generic and carrying another attribute, all of which the
        /// expansion has to leave alone.
        #[inline]
        #[allow(clippy::needless_pass_by_value)]
        #[circuit_breaker(self.breaker)]
        pub async fn echo<T: Send + fmt::Debug>(&self, value: T) -> Result<T, Boom> {
            Ok(value)
        }
    }
}

#[tokio::test]
async fn the_signature_survives_the_expansion() {
    // Calling the public, generic method from another module is the check: it
    // would not resolve if the expansion had dropped `pub` or the generics.
    let generic = signature::Generic::new();

    assert_eq!(generic.echo(5u8).await.expect("the breaker is closed"), 5);
    assert_eq!(
        generic.echo("text").await.expect("the breaker is closed"),
        "text"
    );
}
