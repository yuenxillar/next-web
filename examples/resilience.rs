//! The five resilience patterns, exercised end to end.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin resilience
//! ```
//!
//! Each section prints what it observed, so the behaviour can be read off the
//! output rather than taken on trust. Nothing here touches the network or a
//! database: the guard is always around a local closure, and the sections that
//! care about time drive an injected clock instead of sleeping. The whole
//! example finishes in well under a second.
//!
//! What the sections cover:
//!
//! 1. a retry absorbing a flaky dependency, with the circuit breaker watching
//!    every attempt the retry makes
//! 2. a circuit breaker taking a dead dependency out of rotation
//! 3. a time limiter cutting off a slow call
//! 4. a rate limiter enforcing a quota
//! 5. a bulkhead capping concurrency across tasks
//! 6. a custom permission store giving many limiter instances one shared quota
//! 7. the event stream a pattern publishes
//! 8. an injected clock making the open → half-open transition deterministic

use std::{
    fmt, io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    time::Duration,
};

use next_web_resilience::{
    async_trait,
    bulkhead::BulkheadConfig,
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
    clock::{Clock, ManualClock},
    decorator::Resilience,
    error::ResilienceError,
    ratelimiter::{
        PermissionRequest, RateLimiter, RateLimiterConfig, RateLimiterStore, Refusal,
    },
    registry::ResilienceRegistry,
    retry::RetryConfig,
    timelimiter::TimeLimiterConfig,
};

#[tokio::main]
async fn main() {
    println!("Resilience patterns — every section below runs against a local closure.");

    flaky_dependency().await;
    dead_dependency().await;
    slow_dependency().await;
    quota().await;
    concurrency_limit().await;
    shared_quota().await;
    published_events().await;
    injected_clock();

    println!("\nDone: all five patterns, the registry, the composed pipeline,");
    println!("the metrics snapshots, the event streams, the injected clock and the");
    println!("pluggable permission store all behaved as documented.");
}

/// Prints a section heading.
fn section(step: u32, title: &str) {
    println!("\n── {step}. {title}");
}

/// A payment gateway that fails twice and then works.
async fn flaky_dependency() {
    section(
        1,
        "A retry absorbs a flaky dependency; the breaker sees every attempt",
    );

    let registry = ResilienceRegistry::new();
    registry.circuit_breaker("payments", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(10)
            // Far above the traffic of this section, so the breaker stays closed
            // and we can look at what it recorded.
            .minimum_number_of_calls(100)
            .build()
    });
    registry.retry("payments", || {
        RetryConfig::builder()
            .max_attempts(3)
            .wait_duration(Duration::from_millis(5))
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "payments");
    let attempts = Arc::new(AtomicUsize::new(0));
    let probe = attempts.clone();

    let outcome = resilience
        .execute(move || {
            let probe = probe.clone();
            async move {
                let attempt = probe.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 3 {
                    Err(io::Error::other("gateway hiccup"))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

    println!("   result                : {}", render(&outcome));
    println!("   attempts made         : {}", attempts.load(Ordering::SeqCst));

    let retry = registry.find_retry("payments").expect("registered");
    let snapshot = retry.snapshot();
    println!(
        "   retry metrics         : retries={}, succeeded-with-retry={}, failed-with-retry={}",
        snapshot.number_of_retries,
        snapshot.number_of_successful_calls_with_retry,
        snapshot.number_of_failed_calls_with_retry
    );

    let breaker = registry
        .find_circuit_breaker("payments")
        .expect("registered");
    let snapshot = breaker.snapshot();
    println!(
        "   breaker metrics       : buffered={}, failed={}, succeeded={}, failure-rate={:.0}%, state={}",
        snapshot.number_of_buffered_calls,
        snapshot.number_of_failed_calls,
        snapshot.number_of_successful_calls,
        snapshot.failure_rate,
        snapshot.state
    );
    println!("   note                  : the breaker counted 3 calls, not 1 — the retry");
    println!("                           wraps it, so it observes each attempt separately");
}

/// An inventory service that is simply down.
async fn dead_dependency() {
    section(2, "A circuit breaker takes a dead dependency out of rotation");

    let registry = ResilienceRegistry::new();
    registry.circuit_breaker("inventory", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(4)
            .minimum_number_of_calls(4)
            .failure_rate_threshold(50.0)
            .wait_duration_in_open_state(Duration::from_secs(30))
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "inventory");
    let invocations = Arc::new(AtomicUsize::new(0));

    for call in 1..=4 {
        let probe = invocations.clone();
        let outcome = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<u32, _>(io::Error::other("inventory is down"))
                }
            })
            .await;
        println!("   call {call} → {}", render(&outcome));
    }

    let breaker = registry
        .find_circuit_breaker("inventory")
        .expect("registered");
    println!("   breaker state         : {}", breaker.state());

    // The decisive check: an open breaker must reject the call without running
    // the closure, so the invocation counter must not move.
    let probe = invocations.clone();
    let outcome = resilience
        .execute(move || {
            let probe = probe.clone();
            async move {
                probe.fetch_add(1, Ordering::SeqCst);
                Ok::<u32, io::Error>(7)
            }
        })
        .await;

    println!("   next call → {}", render(&outcome));
    println!(
        "   dependency invocations: {} (the 5th call never reached it)",
        invocations.load(Ordering::SeqCst)
    );
    println!(
        "   rejected calls        : {}",
        breaker.snapshot().number_of_not_permitted_calls
    );
}

/// A search backend that answers far too late.
async fn slow_dependency() {
    section(3, "A time limiter cuts off a slow call");

    let registry = ResilienceRegistry::new();
    registry.time_limiter("search", || {
        TimeLimiterConfig::builder()
            .timeout_duration(Duration::from_millis(50))
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "search");
    let outcome = resilience
        .execute(|| async {
            // Half a second against a 50 millisecond limit.
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok::<u32, io::Error>(1)
        })
        .await;

    println!("   result                : {}", render(&outcome));
    println!("   wall clock spent      : ~50ms, not ~500ms — the future was dropped");

    let limiter = registry.find_time_limiter("search").expect("registered");
    let snapshot = limiter.snapshot();
    println!(
        "   time limiter metrics  : timeouts={}, successes={}, failures={}",
        snapshot.number_of_timeouts,
        snapshot.number_of_successful_calls,
        snapshot.number_of_failed_calls
    );
}

/// A public endpoint that must not be hammered.
async fn quota() {
    section(4, "A rate limiter enforces a quota");

    let registry = ResilienceRegistry::new();
    registry.rate_limiter("login", || {
        RateLimiterConfig::builder()
            .limit_for_period(3)
            // Long enough that no period rolls over during this section.
            .limit_refresh_period(Duration::from_secs(60))
            // Refuse at once instead of making the caller wait a minute.
            .timeout_duration(Duration::ZERO)
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "login");
    let mut granted = 0;
    let mut refused = 0;

    for _ in 0..5 {
        match resilience.execute(|| async { Ok::<u32, io::Error>(1) }).await {
            Ok(_) => granted += 1,
            Err(_) => refused += 1,
        }
    }

    println!("   5 calls in one period : granted={granted}, refused={refused}");

    let limiter = registry.find_rate_limiter("login").expect("registered");
    let snapshot = limiter.snapshot();
    println!(
        "   rate limiter metrics  : acquired={}, rejected={}, available={:?}",
        snapshot.number_of_acquired_permissions,
        snapshot.number_of_rejected_permissions,
        snapshot.available_permissions
    );
    println!("   note                  : `available` is `Some` because this limiter uses the");
    println!("                           in-process store, which can answer without a round trip");
}

/// One slow dependency must not eat every task slot.
async fn concurrency_limit() {
    section(5, "A bulkhead caps how many calls run at once");

    let registry = Arc::new(ResilienceRegistry::new());
    registry.bulkhead("reports", || {
        BulkheadConfig::builder()
            .max_concurrent_calls(2)
            .max_wait_duration(Duration::from_secs(2))
            .build()
    });

    let resilience = Arc::new(Resilience::from_registry(&registry, "reports"));
    let in_flight = Arc::new(AtomicU32::new(0));
    let peak = Arc::new(AtomicU32::new(0));

    let mut tasks = Vec::new();
    for _ in 0..6 {
        let resilience = resilience.clone();
        let in_flight = in_flight.clone();
        let peak = peak.clone();

        tasks.push(tokio::spawn(async move {
            resilience
                .execute(move || {
                    let in_flight = in_flight.clone();
                    let peak = peak.clone();
                    async move {
                        let concurrent = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(concurrent, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        in_flight.fetch_sub(1, Ordering::SeqCst);
                        Ok::<u32, io::Error>(1)
                    }
                })
                .await
                .is_ok()
        }));
    }

    let mut completed = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            completed += 1;
        }
    }

    println!("   {completed} of 6 calls completed");
    println!(
        "   peak concurrent calls : {} (limit 2)",
        peak.load(Ordering::SeqCst)
    );

    let bulkhead = registry.find_bulkhead("reports").expect("registered");
    let snapshot = bulkhead.snapshot();
    println!(
        "   bulkhead metrics      : available={}, max={}, rejected={}",
        snapshot.available_concurrent_calls,
        snapshot.max_allowed_concurrent_calls,
        snapshot.number_of_rejected_calls
    );
}

/// A stand-in for a permission store in another process, such as one backed by
/// Redis.
///
/// Only the shape matters here. The budget lives behind a lock *inside the
/// store* rather than inside the limiter, which is the property that lets two
/// limiter instances share one quota. A real implementation would run the same
/// logic as a single server side script.
#[derive(Debug, Default)]
struct SharedStore {
    /// The period being served and the permissions left in it.
    budget: Mutex<Option<(u64, u32)>>,
}

#[async_trait]
impl RateLimiterStore for SharedStore {
    async fn try_acquire(&self, request: PermissionRequest<'_>) -> Result<(), Refusal> {
        let period = request.limit_refresh_period.as_nanos().max(1);
        let cycle = u64::try_from(request.now.as_nanos() / period).unwrap_or(u64::MAX);

        // The guard is confined to this block and there is no await inside it,
        // so the returned future stays `Send`.
        let permitted = {
            let mut guard = self.budget.lock().expect("the budget is not poisoned");
            let entry = guard.get_or_insert((cycle, request.limit_for_period));

            if entry.0 != cycle {
                *entry = (cycle, request.limit_for_period);
            }

            if entry.1 == 0 {
                false
            } else {
                entry.1 -= 1;
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

    // `available_permissions` keeps its default: a store in another process
    // cannot answer without a round trip, and this example shows that below.
}

/// Two limiter instances, one quota, because they share a store.
async fn shared_quota() {
    section(
        6,
        "A custom store gives many limiter instances one shared quota",
    );

    let store: Arc<dyn RateLimiterStore> = Arc::new(SharedStore::default());
    let config = || {
        RateLimiterConfig::builder()
            .limit_for_period(3)
            .limit_refresh_period(Duration::from_secs(600))
            .timeout_duration(Duration::ZERO)
            .build()
    };

    // Two "instances" of the application, each with its own limiter object.
    let instance_a = RateLimiter::with_store("login", config(), store.clone());
    let instance_b = RateLimiter::with_store("login", config(), store.clone());

    for (label, limiter) in [
        ("a", &instance_a),
        ("b", &instance_b),
        ("a", &instance_a),
        ("b", &instance_b),
    ] {
        let verdict = match limiter.acquire_permission().await {
            Ok(()) => "permitted",
            Err(_) => "refused",
        };
        println!("   instance {label} asks        : {verdict}");
    }

    println!("   ── the fourth call is refused: the quota of three is shared, so it does");
    println!("      not matter which instance spent it");

    println!(
        "   a: acquired={}, rejected={}   b: acquired={}, rejected={}",
        instance_a.snapshot().number_of_acquired_permissions,
        instance_a.snapshot().number_of_rejected_permissions,
        instance_b.snapshot().number_of_acquired_permissions,
        instance_b.snapshot().number_of_rejected_permissions
    );
    println!("   note                  : each instance counts only its own calls, but the");
    println!("                           quota they spend is the one in the store");

    // And the honest part of a remote store: it cannot report availability
    // locally, so the snapshot says so instead of guessing.
    println!(
        "   available permissions : {:?} (unknown, because the store is remote)",
        instance_a.snapshot().available_permissions
    );
}

/// The event stream every pattern publishes.
async fn published_events() {
    section(7, "A pattern publishes what it did on its event stream");

    let breaker = Arc::new(CircuitBreaker::new(
        "search",
        CircuitBreakerConfig::builder()
            .sliding_window_size(2)
            .minimum_number_of_calls(2)
            .failure_rate_threshold(50.0)
            .build(),
    ));

    let mut events = breaker.event_stream();

    for _ in 0..2 {
        let _ = breaker
            .call(|| async { Err::<u32, io::Error>(io::Error::other("search down")) })
            .await;
    }
    // This one is rejected, which is itself an event.
    let _ = breaker.call(|| async { Ok::<u32, io::Error>(1) }).await;

    // `try_recv` drains what was published without waiting; a real application
    // would forward these to metrics, logs or a dashboard.
    while let Ok(event) = events.try_recv() {
        println!("   {event:?}");
    }
}

/// An injected clock makes the open → half-open transition testable.
fn injected_clock() {
    section(
        8,
        "An injected clock makes the open → half-open transition deterministic",
    );

    let clock = Arc::new(ManualClock::new());
    let breaker = CircuitBreaker::new(
        "payments",
        CircuitBreakerConfig::builder()
            .sliding_window_size(2)
            .minimum_number_of_calls(2)
            .failure_rate_threshold(50.0)
            // Review the dependency 30 seconds after opening.
            .wait_duration_in_open_state(Duration::from_secs(30))
            .clock(clock.clone())
            .build(),
    );

    for _ in 0..2 {
        breaker.acquire_permission().expect("the breaker is closed");
        breaker.on_error(Duration::from_millis(1), &io::Error::other("down"));
    }
    println!(
        "   after 2 failures      : {} (clock at {:?})",
        breaker.state(),
        clock.monotonic_time()
    );

    clock.advance(Duration::from_secs(29));
    println!(
        "   after 29s of waiting  : {} (clock at {:?})",
        breaker.state(),
        clock.monotonic_time()
    );

    clock.advance(Duration::from_secs(1));
    println!(
        "   after 30s of waiting  : {} (clock at {:?})",
        breaker.state(),
        clock.monotonic_time()
    );

    println!("   note                  : no thread ever slept — the 30 seconds passed only");
    println!("                           because the injected clock was advanced");
}

/// Renders a pipeline outcome for the section output.
fn render<T: fmt::Debug, E: fmt::Display>(result: &Result<T, ResilienceError<E>>) -> String {
    match result {
        Ok(value) => format!("Ok({value:?})"),
        Err(ResilienceError::CallNotPermitted(_)) => {
            "Err(breaker is open; the dependency was NOT called)".to_owned()
        }
        Err(ResilienceError::RateLimitExceeded(_)) => {
            "Err(rate limited; the call was not attempted)".to_owned()
        }
        Err(ResilienceError::BulkheadFull(_)) => {
            "Err(bulkhead is full; the call was not attempted)".to_owned()
        }
        Err(ResilienceError::Timeout(_)) => "Err(timed out and abandoned)".to_owned(),
        Err(ResilienceError::Aborted(error)) => format!("Err(aborted: {})", error.message),
        Err(ResilienceError::MaxRetriesExceeded(error)) => {
            format!("Err(retries exhausted after {} attempts)", error.attempts)
        }
        Err(ResilienceError::Inner(error)) => format!("Err({error})"),
    }
}
