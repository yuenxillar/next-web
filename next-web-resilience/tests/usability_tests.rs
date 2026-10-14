//! Usability tests: the guarantees an application depends on before it can put
//! this crate in a request path.
//!
//! The other integration file checks that each pattern *behaves* as documented.
//! This one checks that the crate can actually be *used*: that every public type
//! can be shared across threads, that the guard futures can be spawned, that
//! shared instances really share their counters under real concurrency, and that
//! the accessors and builders an application touches are wired up.

use std::{
    error::Error,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use next_web_resilience::async_trait;

use next_web_resilience::{
    bulkhead::{Bulkhead, BulkheadConfig, BulkheadError, BulkheadEvent, BulkheadPermit, BulkheadSnapshot},
    circuitbreaker::{
        CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerEvent,
        CircuitBreakerSnapshot, CircuitBreakerState,
    },
    decorator::{Resilience, ResilienceBuilder},
    error::{AbortedError, ResilienceError},
    ratelimiter::{
        InMemoryRateLimiterStore, PermissionRequest, RateLimiter, RateLimiterConfig,
        RateLimiterError, RateLimiterEvent, RateLimiterSnapshot, RateLimiterStore, Refusal,
    },
    registry::ResilienceRegistry,
    retry::{
        ExponentialIntervalFunction, FixedIntervalFunction, Retry, RetryConfig, RetryError,
        RetryEvent, RetrySnapshot,
    },
    timelimiter::{
        TimeLimiter, TimeLimiterConfig, TimeLimiterError, TimeLimiterEvent, TimeLimiterSnapshot,
    },
    CallOutcome, ManualClock, RateLimitExceededError, WindowSnapshot,
};

/// Compiles only when `T` can be moved between threads, shared between threads
/// and stored in a `'static` context.
fn assert_shareable<T: Send + Sync + 'static>() {}

/// Compiles only when `F` is a future that may be handed to `tokio::spawn`,
/// which needs `Send`. The `'static` half of that requirement is already
/// covered by `every_public_type_can_be_shared_across_threads`, and the
/// multi-threaded tests below exercise the whole thing for real.
fn assert_send_future<F: std::future::Future + Send>(_: F) {}

#[test]
fn every_public_type_can_be_shared_across_threads() {
    assert_shareable::<CircuitBreaker>();
    assert_shareable::<CircuitBreakerConfig>();
    assert_shareable::<CircuitBreakerState>();
    assert_shareable::<CircuitBreakerEvent>();
    assert_shareable::<CircuitBreakerSnapshot>();
    assert_shareable::<CircuitBreakerError<io::Error>>();

    assert_shareable::<RateLimiter>();
    assert_shareable::<RateLimiterConfig>();
    assert_shareable::<RateLimiterEvent>();
    assert_shareable::<RateLimiterSnapshot>();
    assert_shareable::<RateLimiterError<io::Error>>();
    // The store seam has to be shareable too: a store is held behind an `Arc`
    // and used from every task that calls the limiter.
    assert_shareable::<Arc<dyn RateLimiterStore>>();
    assert_shareable::<InMemoryRateLimiterStore>();
    assert_shareable::<PermissionRequest<'static>>();
    assert_shareable::<Refusal>();

    assert_shareable::<Bulkhead>();
    assert_shareable::<BulkheadConfig>();
    assert_shareable::<BulkheadEvent>();
    assert_shareable::<BulkheadSnapshot>();
    assert_shareable::<BulkheadError<io::Error>>();
    // A permit is moved into whatever task holds the slot, so it must travel.
    assert_shareable::<BulkheadPermit>();

    assert_shareable::<Retry>();
    assert_shareable::<RetryConfig>();
    assert_shareable::<RetryEvent>();
    assert_shareable::<RetrySnapshot>();
    assert_shareable::<RetryError<io::Error>>();
    assert_shareable::<ExponentialIntervalFunction>();
    assert_shareable::<FixedIntervalFunction>();

    assert_shareable::<TimeLimiter>();
    assert_shareable::<TimeLimiterConfig>();
    assert_shareable::<TimeLimiterEvent>();
    assert_shareable::<TimeLimiterSnapshot>();
    assert_shareable::<TimeLimiterError<io::Error>>();

    assert_shareable::<Resilience>();
    assert_shareable::<ResilienceBuilder>();
    assert_shareable::<ResilienceRegistry>();
    assert_shareable::<ResilienceError<io::Error>>();

    assert_shareable::<ManualClock>();
    assert_shareable::<CallOutcome>();
    assert_shareable::<WindowSnapshot>();
    assert_shareable::<next_web_resilience::error::CallNotPermittedError>();
    assert_shareable::<RateLimitExceededError>();
    assert_shareable::<next_web_resilience::error::BulkheadFullError>();
    assert_shareable::<next_web_resilience::error::TimeoutError>();
    assert_shareable::<next_web_resilience::error::MaxRetriesExceededError<io::Error>>();
}

#[tokio::test]
async fn every_guard_future_can_be_spawned() {
    // Constructing the future and handing it to a `Send + 'static` bound is the
    // compile-time proof; it is never polled.
    assert_send_future(CircuitBreaker::of("payments").call(|| async {
        Ok::<_, io::Error>(())
    }));

    assert_send_future(RateLimiter::of("api").call(|| async { Ok::<_, io::Error>(()) }));
    assert_send_future(Bulkhead::of("search").call(|| async { Ok::<_, io::Error>(()) }));
    assert_send_future(TimeLimiter::of("payments").call(|| async { Ok::<_, io::Error>(()) }));
    assert_send_future(Retry::of("inventory").call(|| async { Ok::<_, io::Error>(()) }));
    assert_send_future(
        Retry::of("inventory")
            .call_on_result(|| async { Ok::<_, io::Error>(0u32) }, |_: &u32| false),
    );
    assert_send_future(Resilience::new("all").execute(|| async { Ok::<_, io::Error>(()) }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_rate_limiter_keeps_one_budget_across_spawned_tasks() {
    let registry = Arc::new(ResilienceRegistry::new());
    let _ = registry.rate_limiter("api", || {
        RateLimiterConfig::builder()
            .limit_for_period(3)
            .limit_refresh_period(Duration::from_secs(600))
            .timeout_duration(Duration::ZERO)
            .build()
    });
    let resilience = Arc::new(Resilience::from_registry(&registry, "api"));

    // Eight tasks race for three permissions. The compare-and-swap in the limiter
    // is what makes the total exactly three instead of "three or so".
    let mut handles = Vec::new();
    for _ in 0..8 {
        let resilience = resilience.clone();
        handles.push(tokio::spawn(async move {
            resilience
                .execute(|| async { Ok::<_, io::Error>(()) })
                .await
                .is_ok()
        }));
    }

    let mut granted = 0;
    for handle in handles {
        if handle.await.expect("the task must not panic") {
            granted += 1;
        }
    }

    assert_eq!(granted, 3, "the period budget must be granted exactly once each");

    let limiter = registry
        .find_rate_limiter("api")
        .expect("the limiter is registered");
    assert_eq!(limiter.snapshot().number_of_acquired_permissions, 3);
    assert_eq!(limiter.snapshot().number_of_rejected_permissions, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_bulkhead_bounds_concurrency_across_spawned_tasks() {
    let bulkhead = Arc::new(Bulkhead::new(
        "search",
        BulkheadConfig::builder()
            .max_concurrent_calls(4)
            // Generous, so every call is eventually admitted and the count of
            // successes is deterministic.
            .max_wait_duration(Duration::from_secs(5))
            .build(),
    ));

    let in_flight = Arc::new(AtomicU32::new(0));
    let peak = Arc::new(AtomicU32::new(0));

    let mut handles = Vec::new();
    for _ in 0..12 {
        let bulkhead = bulkhead.clone();
        let in_flight = in_flight.clone();
        let peak = peak.clone();

        handles.push(tokio::spawn(async move {
            bulkhead
                .call(|| {
                    let in_flight = in_flight.clone();
                    let peak = peak.clone();
                    async move {
                        let concurrent = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(concurrent, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        in_flight.fetch_sub(1, Ordering::SeqCst);
                        Ok::<_, io::Error>(())
                    }
                })
                .await
                .is_ok()
        }));
    }

    let mut succeeded = 0;
    for handle in handles {
        if handle.await.expect("the task must not panic") {
            succeeded += 1;
        }
    }

    assert_eq!(succeeded, 12, "every call is admitted once a slot frees up");
    let observed = peak.load(Ordering::SeqCst);
    assert!(
        observed <= 4,
        "the bulkhead let {observed} calls run at once for a limit of 4"
    );
    assert_eq!(in_flight.load(Ordering::SeqCst), 0);
    assert_eq!(bulkhead.available_concurrent_calls(), 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shared_circuit_breaker_observes_calls_from_every_task() {
    let registry = Arc::new(ResilienceRegistry::new());
    let _ = registry.circuit_breaker("payments", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(4)
            .minimum_number_of_calls(4)
            .failure_rate_threshold(50.0)
            .wait_duration_in_open_state(Duration::from_secs(30))
            .build()
    });
    let resilience = Arc::new(Resilience::from_registry(&registry, "payments"));

    let mut handles = Vec::new();
    for _ in 0..4 {
        let resilience = resilience.clone();
        handles.push(tokio::spawn(async move {
            let _ = resilience
                .execute(|| async { Err::<(), _>(io::Error::other("down")) })
                .await;
        }));
    }
    for handle in handles {
        handle.await.expect("the task must not panic");
    }

    // The four failures may arrive in any order, but they must all land in the
    // one shared window.
    let breaker = registry
        .find_circuit_breaker("payments")
        .expect("the breaker is registered");
    assert_eq!(breaker.state(), CircuitBreakerState::Open);
    assert_eq!(breaker.snapshot().number_of_failed_calls, 4);
}

#[test]
fn the_registry_finds_and_removes_every_pattern() {
    let registry = ResilienceRegistry::new();

    let breaker = registry.circuit_breaker_or_default("resource");
    let limiter = registry.rate_limiter_or_default("resource");
    let bulkhead = registry.bulkhead_or_default("resource");
    let time_limiter = registry.time_limiter_or_default("resource");
    let retry = registry.retry_or_default("resource");

    assert!(Arc::ptr_eq(
        &breaker,
        &registry
            .find_circuit_breaker("resource")
            .expect("the breaker is registered")
    ));
    assert!(Arc::ptr_eq(
        &limiter,
        &registry
            .find_rate_limiter("resource")
            .expect("the limiter is registered")
    ));
    assert!(Arc::ptr_eq(
        &bulkhead,
        &registry
            .find_bulkhead("resource")
            .expect("the bulkhead is registered")
    ));
    assert!(Arc::ptr_eq(
        &time_limiter,
        &registry
            .find_time_limiter("resource")
            .expect("the time limiter is registered")
    ));
    assert!(Arc::ptr_eq(
        &retry,
        &registry
            .find_retry("resource")
            .expect("the retry is registered")
    ));

    assert_eq!(registry.len(), 5);

    assert!(registry.remove_rate_limiter("resource").is_some());
    assert!(registry.remove_bulkhead("resource").is_some());
    assert!(registry.remove_time_limiter("resource").is_some());
    assert!(registry.remove_retry("resource").is_some());
    assert!(registry.remove_circuit_breaker("resource").is_some());

    assert!(registry.is_empty());
    assert!(registry.find_rate_limiter("resource").is_none());
    assert!(registry.find_bulkhead("resource").is_none());
    assert!(registry.find_time_limiter("resource").is_none());
    assert!(registry.find_retry("resource").is_none());
}

#[test]
fn the_pattern_builders_accept_a_configuration_closure() {
    let breaker = CircuitBreaker::builder("payments")
        .config(|config| {
            config
                .sliding_window_size(7)
                .minimum_number_of_calls(3)
                .failure_rate_threshold(25.0)
        })
        .build();
    assert_eq!(breaker.name(), "payments");
    assert_eq!(breaker.config().sliding_window_size(), 7);
    assert_eq!(breaker.config().minimum_number_of_calls(), 3);
    assert_eq!(breaker.config().failure_rate_threshold(), 25.0);

    let retry = Retry::builder("inventory")
        .config(|config| {
            config
                .max_attempts(9)
                .wait_duration(Duration::from_millis(7))
                .fail_after_max_attempts(true)
        })
        .build();
    assert_eq!(retry.name(), "inventory");
    assert_eq!(retry.config().max_attempts(), 9);
    assert_eq!(retry.config().wait_duration(), Duration::from_millis(7));
    assert!(retry.config().fail_after_max_attempts());
}

#[test]
fn every_pattern_reports_its_name_and_settings() {
    let breaker = CircuitBreaker::new(
        "cb",
        CircuitBreakerConfig::builder().sliding_window_size(11).build(),
    );
    assert_eq!(breaker.name(), "cb");
    assert_eq!(breaker.config().sliding_window_size(), 11);
    assert!(breaker.is_call_permitted());

    let limiter = RateLimiter::new(
        "rl",
        RateLimiterConfig::builder().limit_for_period(5).build(),
    );
    assert_eq!(limiter.name(), "rl");
    assert_eq!(limiter.config().limit_for_period(), 5);

    let bulkhead = Bulkhead::new(
        "bh",
        BulkheadConfig::builder().max_concurrent_calls(3).build(),
    );
    assert_eq!(bulkhead.name(), "bh");
    assert_eq!(bulkhead.config().max_concurrent_calls(), 3);

    let time_limiter = TimeLimiter::new(
        "tl",
        TimeLimiterConfig::builder()
            .timeout_duration(Duration::from_millis(9))
            .cancel_running_future(false)
            .build(),
    );
    assert_eq!(time_limiter.name(), "tl");
    assert_eq!(
        time_limiter.config().timeout_duration(),
        Duration::from_millis(9)
    );
    assert!(!time_limiter.cancel_running_future());

    let retry = Retry::new("rt", RetryConfig::builder().max_attempts(6).build());
    assert_eq!(retry.name(), "rt");
    assert_eq!(retry.config().max_attempts(), 6);
}

#[test]
fn a_breaker_can_be_opened_and_closed_explicitly() {
    let breaker = CircuitBreaker::of("payments");
    assert_eq!(breaker.state(), CircuitBreakerState::Closed);

    breaker.transition_to_open_state();
    assert_eq!(breaker.state(), CircuitBreakerState::Open);
    assert!(breaker.acquire_permission().is_err());

    breaker.transition_to_closed_state();
    assert_eq!(breaker.state(), CircuitBreakerState::Closed);
    assert!(breaker.acquire_permission().is_ok());
}

#[test]
fn an_exponential_interval_reports_its_limits() {
    let function = ExponentialIntervalFunction::new(Duration::from_millis(50), 3.0)
        .with_max_interval(Duration::from_secs(2))
        .with_randomization(0.25);

    assert_eq!(function.initial_interval(), Duration::from_millis(50));
    assert_eq!(function.multiplier(), 3.0);
    assert_eq!(function.max_interval(), Some(Duration::from_secs(2)));
    assert_eq!(function.randomization_factor(), 0.25);
}

#[test]
fn an_aborted_call_is_reported_as_a_rejection() {
    let error: ResilienceError<io::Error> =
        ResilienceError::Aborted(AbortedError::new("task 7 panicked"));

    assert!(error.is_rejection());
    assert!(error.inner_error().is_none());
    assert!(error.to_string().contains("task 7 panicked"));
    // The runtime's report is the source of the rejection, so the cause chain
    // does not stop at the pipeline error.
    assert_eq!(
        error.source().map(ToString::to_string),
        Some("call ended without a result: task 7 panicked".to_owned())
    );
}

#[tokio::test]
async fn a_default_builder_produces_a_usable_pipeline() {
    let resilience = Resilience::builder("unnamed").build();
    assert_eq!(resilience.name(), "unnamed");
    assert!(resilience.retry().is_none());
    assert_eq!(
        resilience
            .execute(|| async { Ok::<_, io::Error>(3) })
            .await
            .expect("a pipeline without patterns is transparent"),
        3
    );

    // `Default` exists so generic setup code that has nothing to add can still
    // hand back a pipeline.
    let from_default = ResilienceBuilder::default().build();
    assert_eq!(
        from_default
            .execute(|| async { Ok::<_, io::Error>(4) })
            .await
            .expect("a default pipeline is transparent"),
        4
    );
}

#[tokio::test]
async fn a_retry_can_be_driven_by_hand_through_its_primitives() {
    // The decorator drives a retry this way, so the primitives are part of the
    // public contract and have to keep working on their own.
    let retry = Retry::new(
        "manual",
        RetryConfig::builder()
            .max_attempts(2)
            .interval_function(FixedIntervalFunction::new(Duration::ZERO))
            .build(),
    );
    let mut events = retry.event_stream();

    let down = io::Error::other("down");
    assert!(retry.should_retry(&down), "the default policy retries errors");

    let wait = retry.prepare_next_attempt(1, down.to_string());
    assert_eq!(wait, Duration::ZERO);
    retry.record_success(2, Duration::from_millis(3));

    assert!(matches!(
        events.recv().await,
        Ok(RetryEvent::Retry { attempt: 1, .. })
    ));
    assert!(matches!(
        events.recv().await,
        Ok(RetryEvent::Success { attempts: 2, .. })
    ));

    let snapshot = retry.snapshot();
    assert_eq!(snapshot.number_of_retries, 1);
    assert_eq!(snapshot.number_of_successful_calls_with_retry, 1);

    retry.record_failure(2, Some("still down".to_owned()));
    assert!(matches!(
        events.recv().await,
        Ok(RetryEvent::MaxAttemptsReached { attempts: 2, .. })
    ));
    assert_eq!(retry.snapshot().number_of_failed_calls_with_retry, 1);
}

/// A stand-in for a store in another process, such as one backed by Redis.
///
/// The budget lives behind a lock inside the store rather than inside the
/// limiter, which is the property that matters: two limiters built over the same
/// store share one budget, and neither can say what is left without asking it.
#[derive(Debug, Default)]
struct SharedStore {
    /// `None` until the first request installs a period.
    budget: Mutex<Option<SharedBudget>>,
}

/// The budget of the period the store is currently serving.
#[derive(Debug)]
struct SharedBudget {
    /// Period the counters below belong to.
    cycle: u64,
    /// Permissions left in that period.
    remaining: u32,
}

#[async_trait]
impl RateLimiterStore for SharedStore {
    async fn try_acquire(&self, request: PermissionRequest<'_>) -> Result<(), Refusal> {
        let period = request.limit_refresh_period.as_nanos().max(1);
        let cycle = u64::try_from(request.now.as_nanos() / period).unwrap_or(u64::MAX);

        // The guard is confined to this block and there is no await inside it,
        // so the future stays `Send` and the store needs no async lock.
        let permitted = {
            let mut guard = self.budget.lock().expect("the budget is not poisoned");
            let budget = guard.get_or_insert(SharedBudget {
                cycle,
                remaining: request.limit_for_period,
            });

            if budget.cycle != cycle {
                budget.cycle = cycle;
                budget.remaining = request.limit_for_period;
            }

            if budget.remaining == 0 {
                false
            } else {
                budget.remaining -= 1;
                true
            }
        };

        if permitted {
            return Ok(());
        }

        let boundary = (u128::from(cycle) + 1).saturating_mul(period);
        let wait = boundary.saturating_sub(request.now.as_nanos());
        Err(Refusal {
            wait_until_next_period: Duration::from_nanos(u64::try_from(wait).unwrap_or(u64::MAX)),
        })
    }

    // `available_permissions` is deliberately left at its default: a store in
    // another process cannot answer without a round trip.
}

/// A configuration whose budget cannot roll over during a test.
fn shared_config() -> RateLimiterConfig {
    RateLimiterConfig::builder()
        .limit_for_period(3)
        .limit_refresh_period(Duration::from_secs(600))
        .timeout_duration(Duration::ZERO)
        .build()
}

#[tokio::test]
async fn two_limiters_over_one_store_share_one_budget() {
    let store: Arc<dyn RateLimiterStore> = Arc::new(SharedStore::default());
    let first = RateLimiter::with_store("login", shared_config(), store.clone());
    let second = RateLimiter::with_store("login", shared_config(), store.clone());

    first
        .acquire_permission()
        .await
        .expect("the shared budget covers two calls");
    first
        .acquire_permission()
        .await
        .expect("the shared budget covers two calls");
    second
        .acquire_permission()
        .await
        .expect("the third permission is shared as well");

    assert!(
        second.acquire_permission().await.is_err(),
        "a budget of three is spent whichever instance asks"
    );

    // Each instance counts only what it did itself.
    assert_eq!(first.snapshot().number_of_acquired_permissions, 2);
    assert_eq!(second.snapshot().number_of_acquired_permissions, 1);
    assert_eq!(first.snapshot().number_of_rejected_permissions, 0);
    assert_eq!(second.snapshot().number_of_rejected_permissions, 1);
}

#[tokio::test]
async fn a_store_in_another_process_reports_unknown_availability() {
    let store: Arc<dyn RateLimiterStore> = Arc::new(SharedStore::default());
    let limiter = RateLimiter::with_store("login", shared_config(), store.clone());
    limiter
        .acquire_permission()
        .await
        .expect("within the budget");

    assert!(Arc::ptr_eq(limiter.store(), &store));

    let snapshot = limiter.snapshot();
    assert_eq!(snapshot.available_permissions, None);
    assert_eq!(
        snapshot.is_permission_available(),
        None,
        "not knowing must never be reported as nothing left"
    );
    assert_eq!(snapshot.number_of_acquired_permissions, 1);
    assert_eq!(snapshot.limit_for_period, 3);
}

#[tokio::test]
async fn the_local_default_store_does_report_availability() {
    let limiter = RateLimiter::new("login", shared_config());

    assert_eq!(limiter.snapshot().available_permissions, Some(3));
    limiter
        .acquire_permission()
        .await
        .expect("within the budget");
    assert_eq!(limiter.snapshot().available_permissions, Some(2));
    assert_eq!(limiter.snapshot().is_permission_available(), Some(true));
}

#[tokio::test]
async fn a_store_backed_limiter_can_be_registered_and_composed() {
    let registry = ResilienceRegistry::new();
    let store: Arc<dyn RateLimiterStore> = Arc::new(SharedStore::default());
    let limiter = Arc::new(RateLimiter::with_store(
        "login",
        shared_config(),
        store.clone(),
    ));

    let registered = registry.register_rate_limiter("login", limiter.clone());
    assert!(Arc::ptr_eq(&registered, &limiter));

    // The composed pipeline must use the registered limiter rather than build an
    // in-memory one of its own.
    let resilience = Resilience::from_registry(&registry, "login");
    for _ in 0..3 {
        resilience
            .execute(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect("within the shared budget");
    }

    let error = resilience
        .execute(|| async { Ok::<_, io::Error>(()) })
        .await
        .expect_err("the shared budget is spent");
    assert!(matches!(error, ResilienceError::RateLimitExceeded(_)));
    assert_eq!(limiter.snapshot().number_of_acquired_permissions, 3);
}
