//! The composed resilience pipeline.
//!
//! [`Resilience`] applies several patterns to one call in the order
//! Resilience4j recommends:
//!
//! ```text
//! retry → circuit breaker → rate limiter → time limiter → bulkhead → call
//! ```
//!
//! Read from the outside in, that order means:
//!
//! * a retry re-runs the whole inner chain, so the circuit breaker, the rate
//!   limiter and the bulkhead each see every individual attempt;
//! * a call refused by the circuit breaker or the rate limiter never reaches
//!   the guarded operation;
//! * the bulkhead slot is held for the whole call and released before the retry
//!   waits, so a sleeping retry does not keep a slot busy.

use std::{error::Error, fmt, future::Future, sync::Arc};

use crate::{
    bulkhead::Bulkhead,
    circuitbreaker::CircuitBreaker,
    error::ResilienceError,
    ratelimiter::RateLimiter,
    registry::ResilienceRegistry,
    retry::Retry,
    timelimiter::{TimeLimiter, TimeLimiterError},
};

/// A guarded call carrying every configured resilience pattern.
///
/// Build one with [`Resilience::builder`], take one from a
/// [`ResilienceRegistry`] with [`Resilience::from_registry`], or guard a call
/// with no pattern at all through [`Resilience::new`].
///
/// Patterns are held behind an [`Arc`] rather than owned, so the same instance
/// — and therefore the same counters — can be shared by every call site and
/// still read back through the registry.
pub struct Resilience {
    /// Name of the guarded resource.
    name: String,
    /// Repeats the inner chain while it fails.
    retry: Option<Arc<Retry>>,
    /// Refuses the call when the resource is known to be unhealthy.
    circuit_breaker: Option<Arc<CircuitBreaker>>,
    /// Caps how often the call may be made.
    rate_limiter: Option<Arc<RateLimiter>>,
    /// Caps how long the call may take.
    time_limiter: Option<Arc<TimeLimiter>>,
    /// Caps how many calls may run at the same time.
    bulkhead: Option<Arc<Bulkhead>>,
}

impl Resilience {
    /// Creates a pipeline for `name` that guards nothing.
    ///
    /// This is useful as a starting point for [`Resilience::builder`] and in
    /// tests, where a pattern should be added one at a time.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            retry: None,
            circuit_breaker: None,
            rate_limiter: None,
            time_limiter: None,
            bulkhead: None,
        }
    }

    /// Starts building a pipeline for the resource called `name`.
    pub fn builder(name: impl Into<String>) -> ResilienceBuilder {
        ResilienceBuilder {
            inner: Self::new(name),
        }
    }

    /// Composes every pattern the registry holds under `name`.
    ///
    /// Only the patterns actually registered are applied, so a registry that
    /// knows a circuit breaker and a retry for `name` produces a pipeline of
    /// exactly those two.
    pub fn from_registry(registry: &ResilienceRegistry, name: &str) -> Self {
        Self {
            name: name.to_owned(),
            retry: registry.find_retry(name),
            circuit_breaker: registry.find_circuit_breaker(name),
            rate_limiter: registry.find_rate_limiter(name),
            time_limiter: registry.find_time_limiter(name),
            bulkhead: registry.find_bulkhead(name),
        }
    }

    /// Returns the name of the guarded resource.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the retry of this pipeline, if one is configured.
    pub fn retry(&self) -> Option<&Arc<Retry>> {
        self.retry.as_ref()
    }

    /// Returns the circuit breaker of this pipeline, if one is configured.
    pub fn circuit_breaker(&self) -> Option<&Arc<CircuitBreaker>> {
        self.circuit_breaker.as_ref()
    }

    /// Returns the rate limiter of this pipeline, if one is configured.
    pub fn rate_limiter(&self) -> Option<&Arc<RateLimiter>> {
        self.rate_limiter.as_ref()
    }

    /// Returns the time limiter of this pipeline, if one is configured.
    pub fn time_limiter(&self) -> Option<&Arc<TimeLimiter>> {
        self.time_limiter.as_ref()
    }

    /// Returns the bulkhead of this pipeline, if one is configured.
    pub fn bulkhead(&self) -> Option<&Arc<Bulkhead>> {
        self.bulkhead.as_ref()
    }

    /// Runs `guarded` under every configured pattern.
    ///
    /// The closure is called again for each retry attempt, so it must be safe
    /// to run more than once.
    ///
    /// A rejection raised by the circuit breaker, the rate limiter or the
    /// bulkhead ends the call immediately and is never retried: the patterns
    /// reject a call precisely because making it again right now cannot help.
    /// Only a failure of the guarded operation itself is retried.
    ///
    /// # Errors
    ///
    /// Returns the [`ResilienceError`] variant matching whichever pattern ended
    /// the call, or [`ResilienceError::Inner`] carrying the failure of the
    /// guarded operation.
    pub async fn execute<T, E, F, Fut>(&self, mut guarded: F) -> Result<T, ResilienceError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Error + 'static,
    {
        let max_attempts = self
            .retry
            .as_ref()
            .map_or(1, |retry| retry.config().max_attempts());
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;

            let started = tokio::time::Instant::now();
            let outcome = self.guard_once(&mut guarded).await;
            let duration = started.elapsed();

            match outcome {
                Ok(value) => {
                    if let Some(retry) = &self.retry {
                        retry.record_success(attempt, duration);
                    }
                    return Ok(value);
                }
                Err(error) => {
                    let Some(retry) = &self.retry else {
                        return Err(error);
                    };

                    if attempt >= max_attempts || !retry.should_retry(&error) {
                        retry.record_failure(attempt, Some(error.to_string()));
                        return Err(error);
                    }

                    let wait = retry.prepare_next_attempt(attempt, error.to_string());
                    if !wait.is_zero() {
                        tokio::time::sleep(wait).await;
                    }
                }
            }
        }
    }

    /// Runs one attempt through the gates, the time limiter and the operation.
    async fn guard_once<T, E, F, Fut>(&self, guarded: &mut F) -> Result<T, ResilienceError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Error + 'static,
    {
        // Refuse the call outright when the resource is known to be unhealthy.
        if let Some(circuit_breaker) = &self.circuit_breaker {
            circuit_breaker.acquire_permission()?;
        }

        // Refuse the call when this caller has spent its share of the period.
        if let Some(rate_limiter) = &self.rate_limiter {
            let timeout = rate_limiter.config().timeout_duration();
            rate_limiter
                .acquire_permission_with_timeout(timeout)
                .await?;
        }

        // Hold a bulkhead slot for the whole call, including the time limit.
        let permit = match &self.bulkhead {
            Some(bulkhead) => Some(bulkhead.acquire_permission().await?),
            None => None,
        };

        let started = tokio::time::Instant::now();

        let outcome = match &self.time_limiter {
            // A `&mut F` is itself an `FnOnce`, so the closure can be handed to
            // the time limiter by reference.
            Some(time_limiter) => time_limiter
                .call(&mut *guarded)
                .await
                .map_err(map_time_limiter_error),
            None => match guarded().await {
                Ok(value) => Ok(value),
                Err(error) => Err(ResilienceError::Inner(error)),
            },
        };

        let duration = started.elapsed();

        // Give the slot back before the outcome is recorded and, on the retry
        // path, before the pipeline starts waiting.
        drop(permit);

        // A rejection above never reaches this point, so the breaker only ever
        // sees the outcome of a call it let through.
        if let Some(circuit_breaker) = &self.circuit_breaker {
            match &outcome {
                Ok(_) => circuit_breaker.on_success(duration),
                Err(error) => circuit_breaker.on_error(duration, error),
            }
        }

        outcome
    }
}

impl fmt::Debug for Resilience {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resilience")
            .field("name", &self.name)
            .field("retry", &self.retry.as_ref().map(|retry| retry.name()))
            .field(
                "circuit_breaker",
                &self
                    .circuit_breaker
                    .as_ref()
                    .map(|breaker| breaker.name()),
            )
            .field(
                "rate_limiter",
                &self.rate_limiter.as_ref().map(|limiter| limiter.name()),
            )
            .field(
                "time_limiter",
                &self.time_limiter.as_ref().map(|limiter| limiter.name()),
            )
            .field(
                "bulkhead",
                &self.bulkhead.as_ref().map(|bulkhead| bulkhead.name()),
            )
            .finish()
    }
}

/// Builds a [`Resilience`] pipeline out of the patterns it is given.
pub struct ResilienceBuilder {
    inner: Resilience,
}

impl Default for ResilienceBuilder {
    /// Returns a builder whose pipeline is named but guards nothing.
    ///
    /// Prefer [`Resilience::builder`], which names the resource.
    fn default() -> Self {
        Self {
            inner: Resilience::new(String::new()),
        }
    }
}

impl ResilienceBuilder {
    /// Adds a retry, which re-runs the whole inner chain while it fails.
    pub fn retry(mut self, retry: Arc<Retry>) -> Self {
        self.inner.retry = Some(retry);
        self
    }

    /// Adds a circuit breaker, the outermost gate of one attempt.
    pub fn circuit_breaker(mut self, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        self.inner.circuit_breaker = Some(circuit_breaker);
        self
    }

    /// Adds a rate limiter, applied after the circuit breaker.
    pub fn rate_limiter(mut self, rate_limiter: Arc<RateLimiter>) -> Self {
        self.inner.rate_limiter = Some(rate_limiter);
        self
    }

    /// Adds a time limiter, applied to the guarded operation.
    pub fn time_limiter(mut self, time_limiter: Arc<TimeLimiter>) -> Self {
        self.inner.time_limiter = Some(time_limiter);
        self
    }

    /// Adds a bulkhead, the innermost gate before the guarded operation.
    pub fn bulkhead(mut self, bulkhead: Arc<Bulkhead>) -> Self {
        self.inner.bulkhead = Some(bulkhead);
        self
    }

    /// Returns the finished pipeline.
    pub fn build(self) -> Resilience {
        self.inner
    }
}

/// Maps a time limiter failure onto the shared pipeline error.
fn map_time_limiter_error<E>(error: TimeLimiterError<E>) -> ResilienceError<E> {
    match error {
        TimeLimiterError::Timeout(timeout) => ResilienceError::Timeout(timeout),
        TimeLimiterError::Aborted(error) => ResilienceError::Aborted(error),
        TimeLimiterError::Inner(inner) => ResilienceError::Inner(inner),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU32, Ordering},
        },
        time::Duration,
    };

    use super::Resilience;
    use crate::{
        bulkhead::{Bulkhead, BulkheadConfig},
        circuitbreaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState},
        error::ResilienceError,
        ratelimiter::{RateLimiter, RateLimiterConfig},
        registry::ResilienceRegistry,
        retry::{FixedIntervalFunction, Retry, RetryConfig},
        timelimiter::{TimeLimiter, TimeLimiterConfig},
    };

    fn failure() -> io::Error {
        io::Error::other("boom")
    }

    #[tokio::test]
    async fn a_pipeline_without_patterns_is_transparent() {
        let resilience = Resilience::new("plain");

        let value = resilience
            .execute(|| async { Ok::<_, io::Error>(5) })
            .await
            .expect("nothing guards the call");
        assert_eq!(value, 5);

        let error = resilience
            .execute(|| async { Err::<(), _>(failure()) })
            .await
            .expect_err("the failure surfaces unchanged");
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
    }

    #[tokio::test]
    async fn an_open_circuit_breaker_blocks_the_call() {
        let breaker = Arc::new(CircuitBreaker::new(
            "payments",
            CircuitBreakerConfig::builder()
                .sliding_window_size(2)
                .minimum_number_of_calls(2)
                .failure_rate_threshold(50.0)
                .build(),
        ));
        let resilience = Resilience::builder("payments")
            .circuit_breaker(breaker.clone())
            .build();

        for _ in 0..2 {
            let _ = resilience.execute(|| async { Err::<(), _>(failure()) }).await;
        }
        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        let executed = Arc::new(AtomicBool::new(false));
        let probe = executed.clone();
        let error = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    probe.store(true, Ordering::SeqCst);
                    Ok::<_, io::Error>(())
                }
            })
            .await
            .expect_err("the breaker is open");

        assert!(matches!(error, ResilienceError::CallNotPermitted(_)));
        assert!(!executed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn the_circuit_breaker_records_the_outcome_of_each_attempt() {
        let breaker = Arc::new(CircuitBreaker::new(
            "payments",
            CircuitBreakerConfig::builder()
                .sliding_window_size(10)
                .minimum_number_of_calls(100)
                .build(),
        ));
        let resilience = Resilience::builder("payments")
            .circuit_breaker(breaker.clone())
            .build();

        let _ = resilience.execute(|| async { Ok::<_, io::Error>(1) }).await;
        let _ = resilience
            .execute(|| async { Err::<(), _>(failure()) })
            .await;

        let snapshot = breaker.snapshot();
        assert_eq!(snapshot.number_of_successful_calls, 1);
        assert_eq!(snapshot.number_of_failed_calls, 1);
    }

    #[tokio::test]
    async fn an_exhausted_rate_limiter_blocks_the_call() {
        let limiter = Arc::new(RateLimiter::new(
            "public-api",
            RateLimiterConfig::builder()
                .limit_for_period(1)
                .limit_refresh_period(Duration::from_secs(600))
                .timeout_duration(Duration::ZERO)
                .build(),
        ));
        let resilience = Resilience::builder("public-api")
            .rate_limiter(limiter.clone())
            .build();

        // Spend the only permission of the period.
        resilience
            .execute(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect("the first call is permitted");

        let error = resilience
            .execute(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect_err("the period is spent");

        assert!(matches!(error, ResilienceError::RateLimitExceeded(_)));
    }

    #[tokio::test]
    async fn a_full_bulkhead_blocks_the_call() {
        let bulkhead = Arc::new(Bulkhead::new(
            "search",
            BulkheadConfig::builder().max_concurrent_calls(1).build(),
        ));
        let resilience = Resilience::builder("search")
            .bulkhead(bulkhead.clone())
            .build();

        // Hold the only permit for the duration of one call.
        let held = bulkhead.acquire_permission().await.expect("a permit is free");

        let error = resilience
            .execute(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect_err("the bulkhead is full");

        assert!(matches!(error, ResilienceError::BulkheadFull(_)));

        drop(held);
        resilience
            .execute(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect("the slot was released");
    }

    #[tokio::test]
    async fn the_bulkhead_slot_is_released_even_when_the_call_fails() {
        let bulkhead = Arc::new(Bulkhead::new(
            "search",
            BulkheadConfig::builder().max_concurrent_calls(1).build(),
        ));
        let resilience = Resilience::builder("search")
            .bulkhead(bulkhead.clone())
            .build();

        let _ = resilience
            .execute(|| async { Err::<(), _>(failure()) })
            .await;

        assert_eq!(bulkhead.available_concurrent_calls(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_slow_call_times_out() {
        let limiter = Arc::new(TimeLimiter::new(
            "payments",
            TimeLimiterConfig::builder()
                .timeout_duration(Duration::from_millis(50))
                .build(),
        ));
        let resilience = Resilience::builder("payments")
            .time_limiter(limiter)
            .build();

        let error = resilience
            .execute(|| async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok::<_, io::Error>(())
            })
            .await
            .expect_err("the call outlives its limit");

        assert!(matches!(error, ResilienceError::Timeout(_)));
    }

    #[tokio::test]
    async fn the_retry_repeats_the_inner_chain_until_it_succeeds() {
        let retry = Arc::new(Retry::new(
            "inventory",
            RetryConfig::builder()
                .max_attempts(4)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .build(),
        ));
        let resilience = Resilience::builder("inventory")
            .retry(retry.clone())
            .build();
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let value = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    let attempt = probe.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 3 {
                        Err(failure())
                    } else {
                        Ok(attempt)
                    }
                }
            })
            .await
            .expect("the third attempt succeeds");

        assert_eq!(value, 3);
        assert_eq!(retry.snapshot().number_of_successful_calls_with_retry, 1);
    }

    #[tokio::test]
    async fn an_exhausted_retry_reports_the_last_failure_by_default() {
        let retry = Arc::new(Retry::new(
            "inventory",
            RetryConfig::builder()
                .max_attempts(2)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .build(),
        ));
        let resilience = Resilience::builder("inventory")
            .retry(retry.clone())
            .build();
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let error = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(failure())
                }
            })
            .await
            .expect_err("every attempt fails");

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
        assert_eq!(retry.snapshot().number_of_failed_calls_with_retry, 1);
    }

    #[tokio::test]
    async fn a_rejection_is_not_retried() {
        let breaker = Arc::new(CircuitBreaker::new(
            "payments",
            CircuitBreakerConfig::builder()
                .sliding_window_size(2)
                .minimum_number_of_calls(2)
                .failure_rate_threshold(50.0)
                .build(),
        ));
        let retry = Arc::new(Retry::new(
            "payments",
            RetryConfig::builder()
                .max_attempts(5)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .build(),
        ));
        let resilience = Resilience::builder("payments")
            .retry(retry.clone())
            .circuit_breaker(breaker)
            .build();

        // Two failing attempts open the breaker.
        let _ = resilience.execute(|| async { Err::<(), _>(failure()) }).await;

        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();
        let error = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, io::Error>(())
                }
            })
            .await
            .expect_err("the breaker is open");

        assert!(matches!(error, ResilienceError::CallNotPermitted(_)));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "an open breaker must not reach the guarded call"
        );
    }

    #[tokio::test]
    async fn a_pipeline_can_be_composed_from_a_registry() {
        let registry = ResilienceRegistry::new();
        let _ = registry.circuit_breaker("payments", || {
            CircuitBreakerConfig::builder()
                .sliding_window_size(4)
                .minimum_number_of_calls(100)
                .build()
        });
        let _ = registry.retry("payments", || {
            RetryConfig::builder()
                .max_attempts(2)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .build()
        });

        let resilience = Resilience::from_registry(&registry, "payments");

        assert!(resilience.circuit_breaker().is_some());
        assert!(resilience.retry().is_some());
        assert!(resilience.rate_limiter().is_none());
        assert!(resilience.time_limiter().is_none());
        assert!(resilience.bulkhead().is_none());

        let value = resilience
            .execute(|| async { Ok::<_, io::Error>(9) })
            .await
            .expect("the registered patterns let the call through");
        assert_eq!(value, 9);
    }

    #[tokio::test]
    async fn a_pipeline_reports_which_patterns_it_carries() {
        let resilience = Resilience::builder("payments")
            .circuit_breaker(Arc::new(CircuitBreaker::of("payments")))
            .bulkhead(Arc::new(Bulkhead::of("payments")))
            .build();

        assert_eq!(resilience.name(), "payments");
        assert!(resilience.circuit_breaker().is_some());
        assert!(resilience.bulkhead().is_some());
        assert!(resilience.retry().is_none());

        let rendered = format!("{resilience:?}");
        assert!(rendered.contains("payments"));
    }
}
