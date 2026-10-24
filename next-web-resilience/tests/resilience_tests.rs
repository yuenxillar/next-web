//! End-to-end tests that drive the crate the way an application does: through
//! its public API, with every pattern registered under a resource name and
//! composed by the decorator.
//!
//! The unit tests inside each module can reach private items; these cannot, so
//! they are also what keeps the public surface honest.

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use next_web_resilience::{
    bulkhead::BulkheadConfig,
    circuitbreaker::{CircuitBreakerConfig, CircuitBreakerState},
    decorator::Resilience,
    error::ResilienceError,
    ratelimiter::RateLimiterConfig,
    registry::ResilienceRegistry,
    retry::{FixedIntervalFunction, RetryConfig},
    timelimiter::TimeLimiterConfig,
};

/// A failure a dependency might produce.
fn failure() -> io::Error {
    io::Error::other("dependency is unhappy")
}

#[test]
fn the_registry_hands_out_one_instance_per_name() {
    let registry = ResilienceRegistry::new();

    let first = registry.circuit_breaker_or_default("payments");
    let second = registry.circuit_breaker_or_default("payments");

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(registry.circuit_breaker_names(), vec!["payments".to_owned()]);
}

#[test]
fn every_pattern_reports_a_snapshot() {
    let registry = ResilienceRegistry::new();

    let breaker = registry.circuit_breaker_or_default("resource");
    assert_eq!(breaker.snapshot().number_of_buffered_calls, 0);

    let limiter = registry.rate_limiter_or_default("resource");
    assert_eq!(limiter.snapshot().limit_for_period, 50);

    let bulkhead = registry.bulkhead_or_default("resource");
    assert_eq!(bulkhead.snapshot().max_allowed_concurrent_calls, 25);

    let time_limiter = registry.time_limiter_or_default("resource");
    assert_eq!(time_limiter.snapshot().number_of_timeouts, 0);

    let retry = registry.retry_or_default("resource");
    assert_eq!(retry.snapshot().number_of_calls(), 0);
}

#[tokio::test]
async fn a_flaky_dependency_recovers_within_one_pipeline_call() {
    let registry = ResilienceRegistry::new();
    // The window is large enough that the breaker never opens during this test.
    registry.circuit_breaker("inventory", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(10)
            .minimum_number_of_calls(100)
            .build()
    });
    registry.retry("inventory", || {
        RetryConfig::builder()
            .max_attempts(3)
            .interval_function(FixedIntervalFunction::new(Duration::ZERO))
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "inventory");
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

    // The retry wraps the breaker, so the breaker observes every attempt.
    let breaker = registry
        .find_circuit_breaker("inventory")
        .expect("the breaker is registered");
    let snapshot = breaker.snapshot();
    assert_eq!(snapshot.number_of_failed_calls, 2);
    assert_eq!(snapshot.number_of_successful_calls, 1);

    let retry = registry
        .find_retry("inventory")
        .expect("the retry is registered");
    assert_eq!(retry.snapshot().number_of_retries, 2);
    assert_eq!(retry.snapshot().number_of_successful_calls_with_retry, 1);
}

#[tokio::test]
async fn a_broken_dependency_eventually_stops_being_called() {
    let registry = ResilienceRegistry::new();
    registry.circuit_breaker("payments", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(4)
            .minimum_number_of_calls(4)
            .failure_rate_threshold(50.0)
            .wait_duration_in_open_state(Duration::from_secs(30))
            .build()
    });
    let resilience = Resilience::from_registry(&registry, "payments");
    let calls = Arc::new(AtomicU32::new(0));

    for _ in 0..4 {
        let probe = calls.clone();
        let _ = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(failure())
                }
            })
            .await;
    }

    let breaker = registry
        .find_circuit_breaker("payments")
        .expect("the breaker is registered");
    assert_eq!(breaker.state(), CircuitBreakerState::Open);

    // The open breaker must reject the call without reaching the dependency.
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
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(breaker.snapshot().number_of_not_permitted_calls, 1);
}

#[tokio::test]
async fn the_rate_limiter_caps_the_calls_of_a_period() {
    let registry = ResilienceRegistry::new();
    registry.rate_limiter("login", || {
        RateLimiterConfig::builder()
            .limit_for_period(3)
            // Long enough that the period cannot roll over during the test.
            .limit_refresh_period(Duration::from_secs(600))
            .timeout_duration(Duration::ZERO)
            .build()
    });
    let resilience = Resilience::from_registry(&registry, "login");

    for _ in 0..3 {
        resilience
            .execute(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect("the call is inside the budget");
    }

    let error = resilience
        .execute(|| async { Ok::<_, io::Error>(()) })
        .await
        .expect_err("the budget of the period is spent");

    assert!(matches!(error, ResilienceError::RateLimitExceeded(_)));

    let limiter = registry
        .find_rate_limiter("login")
        .expect("the limiter is registered");
    assert_eq!(limiter.snapshot().number_of_acquired_permissions, 3);
    assert_eq!(limiter.snapshot().number_of_rejected_permissions, 1);
}

/// Issues one call that records how many calls are in flight at the same time.
async fn probe_concurrency(
    resilience: &Resilience,
    in_flight: Arc<AtomicU32>,
    peak: Arc<AtomicU32>,
) -> Result<(), ResilienceError<io::Error>> {
    resilience
        .execute(move || {
            let in_flight = in_flight.clone();
            let peak = peak.clone();
            async move {
                let concurrent = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(concurrent, Ordering::SeqCst);
                // Paused time, so this costs nothing and never yields in a way
                // that would let a fourth call slip in.
                tokio::time::sleep(Duration::from_millis(100)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
}

#[tokio::test(start_paused = true)]
async fn the_bulkhead_caps_how_many_calls_run_at_once() {
    let registry = ResilienceRegistry::new();
    registry.bulkhead("search", || {
        BulkheadConfig::builder()
            .max_concurrent_calls(2)
            // No waiting, so the third call is refused at once.
            .max_wait_duration(Duration::ZERO)
            .build()
    });
    let resilience = Resilience::from_registry(&registry, "search");

    let in_flight = Arc::new(AtomicU32::new(0));
    let peak = Arc::new(AtomicU32::new(0));

    let (first, second, third) = tokio::join!(
        probe_concurrency(&resilience, in_flight.clone(), peak.clone()),
        probe_concurrency(&resilience, in_flight.clone(), peak.clone()),
        probe_concurrency(&resilience, in_flight.clone(), peak.clone()),
    );

    assert!(first.is_ok(), "the first call fits in the bulkhead");
    assert!(second.is_ok(), "the second call fits in the bulkhead");
    assert!(
        matches!(third, Err(ResilienceError::BulkheadFull(_))),
        "the third call must be refused, got {third:?}"
    );
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    assert_eq!(in_flight.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn the_time_limiter_bounds_a_slow_call() {
    let registry = ResilienceRegistry::new();
    registry.time_limiter("search", || {
        TimeLimiterConfig::builder()
            .timeout_duration(Duration::from_millis(100))
            .build()
    });
    let resilience = Resilience::from_registry(&registry, "search");

    let error = resilience
        .execute(|| async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok::<_, io::Error>(())
        })
        .await
        .expect_err("the call outlives its limit");

    assert!(matches!(error, ResilienceError::Timeout(_)));

    let limiter = registry
        .find_time_limiter("search")
        .expect("the limiter is registered");
    assert_eq!(limiter.snapshot().number_of_timeouts, 1);
}

#[tokio::test]
async fn every_pattern_of_a_pipeline_can_let_a_call_through() {
    let registry = ResilienceRegistry::new();
    registry.circuit_breaker("payments", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(10)
            .minimum_number_of_calls(100)
            .build()
    });
    registry.rate_limiter("payments", || {
        RateLimiterConfig::builder()
            .limit_for_period(10)
            .limit_refresh_period(Duration::from_secs(600))
            .timeout_duration(Duration::ZERO)
            .build()
    });
    registry.bulkhead("payments", || {
        BulkheadConfig::builder().max_concurrent_calls(4).build()
    });
    registry.time_limiter("payments", || {
        TimeLimiterConfig::builder()
            .timeout_duration(Duration::from_secs(5))
            .build()
    });
    registry.retry("payments", || {
        RetryConfig::builder()
            .max_attempts(2)
            .interval_function(FixedIntervalFunction::new(Duration::ZERO))
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "payments");
    assert_eq!(resilience.name(), "payments");

    let value = resilience
        .execute(|| async { Ok::<_, io::Error>(7) })
        .await
        .expect("every pattern lets a healthy call through");

    assert_eq!(value, 7);

    let retry = registry
        .find_retry("payments")
        .expect("the retry is registered");
    assert_eq!(retry.snapshot().number_of_successful_calls_without_retry, 1);
}
