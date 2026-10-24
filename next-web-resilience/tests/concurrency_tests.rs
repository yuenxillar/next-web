//! The patterns under concurrent use.
//!
//! Every other test in this crate drives a pattern deterministically: one task, a
//! manual clock, and an assertion about exactly what happened. These do the
//! opposite — many tasks racing through one instance on a real clock — because
//! that is the shape a server runs them in, and it is where a shared counter
//! loses an update, a limit admits one call too many, or a lock turns into a
//! deadlock.
//!
//! What they do **not** cover: throughput or latency, memory over a long run, or
//! a real dependency failing. Those need a load harness, not a unit test.
//!
//! Each test is wrapped in a deadline, so a deadlock fails the test instead of
//! hanging the suite.

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::Duration,
};

use next_web_resilience::{
    bulkhead::{Bulkhead, BulkheadConfig},
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState},
    ratelimiter::{RateLimiter, RateLimiterConfig},
    retry::{Retry, RetryConfig},
};

/// How long a test may take before it counts as a deadlock.
const DEADLINE: Duration = Duration::from_secs(30);

/// Tasks running at once in every test.
const TASKS: u32 = 32;

/// Runs every task and fails the test if the whole thing outlasts the deadline.
async fn within_deadline<F>(what: &str, work: F)
where
    F: Future<Output = ()>,
{
    tokio::time::timeout(DEADLINE, work)
        .await
        .unwrap_or_else(|_| panic!("{what} did not finish within {DEADLINE:?}: a deadlock?"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_rate_limiter_grants_exactly_its_quota_when_many_tasks_race_for_it() {
    // A budget far larger than one period's worth of traffic, so the only thing
    // under test is whether a concurrent grant is ever lost or handed out twice.
    let quota = 5_000;
    let limiter = Arc::new(RateLimiter::new(
        "raced",
        RateLimiterConfig::builder()
            .limit_for_period(quota)
            .limit_refresh_period(Duration::from_secs(3_600))
            .timeout_duration(Duration::ZERO)
            .build(),
    ));

    let granted = Arc::new(AtomicU32::new(0));
    let attempted = quota * 2;

    within_deadline("racing the rate limiter", async {
        let mut tasks = Vec::new();
        for _ in 0..TASKS {
            let limiter = Arc::clone(&limiter);
            let granted = Arc::clone(&granted);

            tasks.push(tokio::spawn(async move {
                for _ in 0..(attempted / TASKS) {
                    if limiter.acquire_permission().await.is_ok() {
                        granted.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }

        for task in tasks {
            task.await.expect("the task finishes");
        }
    })
    .await;

    assert_eq!(
        granted.load(Ordering::Relaxed),
        quota,
        "the quota is granted exactly: {attempted} concurrent attempts, {quota} permissions"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_bulkhead_never_admits_more_than_its_limit() {
    let limit = 8;
    let bulkhead = Arc::new(Bulkhead::new(
        "capped",
        BulkheadConfig::builder()
            .max_concurrent_calls(limit)
            // Callers queue rather than being turned away at once, which is what
            // makes the limit contended.
            .max_wait_duration(Duration::from_millis(200))
            .build(),
    ));

    let inside = Arc::new(AtomicU32::new(0));
    let peak = Arc::new(AtomicU32::new(0));

    within_deadline("racing the bulkhead", async {
        let mut tasks = Vec::new();
        for _ in 0..TASKS * 4 {
            let bulkhead = Arc::clone(&bulkhead);
            let inside = Arc::clone(&inside);
            let peak = Arc::clone(&peak);

            tasks.push(tokio::spawn(async move {
                if let Ok(_permit) = bulkhead.acquire_permission().await {
                    let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);

                    tokio::task::yield_now().await;

                    inside.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }

        for task in tasks {
            task.await.expect("the task finishes");
        }
    })
    .await;

    let peak = peak.load(Ordering::SeqCst);
    assert!(
        peak <= limit,
        "at most {limit} calls may be inside at once, {peak} were"
    );
    assert_eq!(
        inside.load(Ordering::SeqCst),
        0,
        "every admitted call released its slot"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_circuit_breaker_counts_every_failure_when_many_calls_fail_at_once() {
    // The window is exactly as large as the burst, so a single lost update leaves
    // the breaker closed and the test fails.
    let failures = TASKS;
    let breaker = Arc::new(CircuitBreaker::new(
        "failing",
        CircuitBreakerConfig::builder()
            .sliding_window_size(failures)
            .minimum_number_of_calls(failures)
            .failure_rate_threshold(50.0)
            .build(),
    ));

    let recorded = Arc::new(AtomicU32::new(0));

    within_deadline("racing the circuit breaker", async {
        let mut tasks = Vec::new();
        for _ in 0..failures {
            let breaker = Arc::clone(&breaker);
            let recorded = Arc::clone(&recorded);

            tasks.push(tokio::spawn(async move {
                if breaker.acquire_permission().is_ok() {
                    breaker.on_error(
                        Duration::from_millis(1),
                        &std::io::Error::other("the dependency is down"),
                    );
                    recorded.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }

        for task in tasks {
            task.await.expect("the task finishes");
        }
    })
    .await;

    assert_eq!(
        recorded.load(Ordering::Relaxed),
        failures,
        "every call that was permitted recorded its outcome"
    );
    assert_eq!(
        breaker.state(),
        CircuitBreakerState::Open,
        "a window of nothing but failures opens the breaker"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn retries_never_exceed_their_budget_when_many_calls_fail_at_once() {
    let max_attempts = 3;
    let retry = Arc::new(Retry::new(
        "flaky",
        RetryConfig::builder()
            .max_attempts(max_attempts)
            .wait_duration(Duration::ZERO)
            .build(),
    ));

    let attempts = Arc::new(AtomicU64::new(0));

    within_deadline("racing the retry", async {
        let mut tasks = Vec::new();
        for _ in 0..TASKS {
            let retry = Arc::clone(&retry);
            let attempts = Arc::clone(&attempts);

            tasks.push(tokio::spawn(async move {
                let outcome: Result<(), _> = retry
                    .call(|| {
                        let attempts = Arc::clone(&attempts);

                        async move {
                            attempts.fetch_add(1, Ordering::Relaxed);

                            Err(std::io::Error::other("the dependency is down"))
                        }
                    })
                    .await;

                assert!(outcome.is_err(), "a call that always fails stays failed");
            }));
        }

        for task in tasks {
            task.await.expect("the task finishes");
        }
    })
    .await;

    assert_eq!(
        attempts.load(Ordering::Relaxed),
        u64::from(TASKS * max_attempts),
        "each call makes its budget of attempts and not one more"
    );
}
