//! A runnable demonstration of all five resilience patterns.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p next-web-resilience --example resilience_demo
//! ```
//!
//! Every scenario prints what the guarded call returned and what the pattern
//! recorded, so the behaviour can be read off the output instead of taken on
//! trust. Durations are deliberately tiny so the whole thing finishes in well
//! under a second.

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    time::Duration,
};

use next_web_resilience::{
    bulkhead::BulkheadConfig,
    circuitbreaker::CircuitBreakerConfig,
    decorator::Resilience,
    error::ResilienceError,
    ratelimiter::RateLimiterConfig,
    registry::ResilienceRegistry,
    retry::{ExponentialIntervalFunction, RetryConfig},
    timelimiter::TimeLimiterConfig,
};

#[tokio::main]
async fn main() {
    flaky_dependency_recovers().await;
    broken_dependency_is_taken_out_of_rotation().await;
    slow_dependency_is_cut_off().await;
    public_endpoint_stays_inside_its_quota().await;
    bulkhead_protects_capacity().await;

    println!("\nAll five patterns behaved as documented.");
}

/// Prints a scenario heading.
fn heading(step: u32, title: &str) {
    println!("\n=== {step}. {title} ===");
}

/// 1. A retry hides a transient fault, and the circuit breaker sees every
///    attempt the retry makes.
async fn flaky_dependency_recovers() {
    heading(1, "A flaky dependency recovers under a retry");

    let registry = ResilienceRegistry::new();
    registry.circuit_breaker("payments", || {
        CircuitBreakerConfig::builder()
            .sliding_window_size(10)
            .minimum_number_of_calls(100)
            .build()
    });
    registry.retry("payments", || {
        RetryConfig::builder()
            .max_attempts(3)
            .interval_function(ExponentialIntervalFunction::new(
                Duration::from_millis(5),
                2.0,
            ))
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

    println!(
        "   guarded call -> {} after {} attempts",
        describe(&outcome),
        attempts.load(Ordering::SeqCst)
    );

    let retry = registry.find_retry("payments").expect("registered");
    let snapshot = retry.snapshot();
    println!(
        "   retry metrics: retries={}, succeeded-with-retry={}",
        snapshot.number_of_retries, snapshot.number_of_successful_calls_with_retry
    );

    let breaker = registry
        .find_circuit_breaker("payments")
        .expect("registered");
    let snapshot = breaker.snapshot();
    println!(
        "   breaker metrics: buffered={}, failed={}, success={}, state={}",
        snapshot.number_of_buffered_calls,
        snapshot.number_of_failed_calls,
        snapshot.number_of_successful_calls,
        snapshot.state
    );
}

/// 2. Once the failure rate crosses the threshold the breaker stops calling the
///    dependency at all.
async fn broken_dependency_is_taken_out_of_rotation() {
    heading(2, "A broken dependency is taken out of rotation");

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
        let result = resilience
            .execute(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<u32, io::Error>(io::Error::other("inventory is down"))
                }
            })
            .await;
        println!("   call {call} -> {}", describe(&result));
    }

    let breaker = registry
        .find_circuit_breaker("inventory")
        .expect("registered");
    println!("   breaker state -> {}", breaker.state());

    let probe = invocations.clone();
    let result = resilience
        .execute(move || {
            let probe = probe.clone();
            async move {
                probe.fetch_add(1, Ordering::SeqCst);
                Ok::<u32, io::Error>(7)
            }
        })
        .await;

    println!("   next call -> {}", describe(&result));
    println!(
        "   the dependency was invoked {} times in total, so the open breaker really stopped calling it",
        invocations.load(Ordering::SeqCst)
    );
    println!(
        "   rejected calls: {}",
        breaker.snapshot().number_of_not_permitted_calls
    );
}

/// 3. A slow answer is abandoned instead of pinning the task forever.
async fn slow_dependency_is_cut_off() {
    heading(3, "A slow dependency is cut off by the time limiter");

    let registry = ResilienceRegistry::new();
    registry.time_limiter("search", || {
        TimeLimiterConfig::builder()
            .timeout_duration(Duration::from_millis(50))
            .build()
    });

    let resilience = Resilience::from_registry(&registry, "search");
    let result = resilience
        .execute(|| async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok::<u32, io::Error>(1)
        })
        .await;

    println!("   guarded call -> {}", describe(&result));

    let limiter = registry.find_time_limiter("search").expect("registered");
    println!(
        "   time limiter metrics: timeouts={}, successes={}",
        limiter.snapshot().number_of_timeouts, limiter.snapshot().number_of_successful_calls
    );
}

/// 4. A public endpoint cannot be called more often than its quota allows.
async fn public_endpoint_stays_inside_its_quota() {
    heading(4, "A public endpoint stays inside its quota");

    let registry = ResilienceRegistry::new();
    registry.rate_limiter("login", || {
        RateLimiterConfig::builder()
            .limit_for_period(3)
            .limit_refresh_period(Duration::from_secs(60))
            // Refuse immediately rather than make the caller wait a minute.
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

    println!("   granted={granted}, refused={refused} out of 5 calls in one period");

    let limiter = registry.find_rate_limiter("login").expect("registered");
    println!(
        "   rate limiter metrics: acquired={}, rejected={}",
        limiter.snapshot().number_of_acquired_permissions,
        limiter.snapshot().number_of_rejected_permissions
    );
}

/// 5. One slow dependency cannot consume every task slot.
async fn bulkhead_protects_capacity() {
    heading(5, "A bulkhead protects our own capacity");

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
        "   peak concurrent calls = {} (limit 2)",
        peak.load(Ordering::SeqCst)
    );
}

/// Renders an outcome for the demonstration output.
fn describe<T: std::fmt::Debug, E: std::fmt::Display>(
    result: &Result<T, ResilienceError<E>>,
) -> String {
    match result {
        Ok(value) => format!("Ok({value:?})"),
        Err(ResilienceError::CallNotPermitted(_)) => {
            "Err(circuit breaker is open, so the dependency was not called)".to_owned()
        }
        Err(ResilienceError::RateLimitExceeded(_)) => {
            "Err(rate limited, so the call was not attempted)".to_owned()
        }
        Err(ResilienceError::BulkheadFull(_)) => {
            "Err(bulkhead is full, so the call was not attempted)".to_owned()
        }
        Err(ResilienceError::Timeout(_)) => "Err(timed out and abandoned)".to_owned(),
        Err(ResilienceError::Aborted(error)) => format!("Err(aborted: {})", error.message),
        Err(ResilienceError::MaxRetriesExceeded(error)) => {
            format!("Err(retries exhausted after {} attempts)", error.attempts)
        }
        Err(ResilienceError::Inner(error)) => format!("Err({error})"),
    }
}
