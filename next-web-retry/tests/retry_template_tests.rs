//! End-to-end behaviour of the retry template and the interceptors.
//!
//! Every test here drives the public API the way an application would: build a template, run a
//! callback that fails a few times, and assert what the caller observes.

use std::{
    any::Any,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use next_web_core::any::{AnyError, AnyValue};
use next_web_retry::{
    backoff::{
        back_off_policy::BackOffPolicy, fixed_back_off_policy::FixedBackOffPolicy,
        sleeper::RecordingSleeper, sleeping_back_off_policy::SleepingBackOffPolicy,
    },
    error::retry_error::{RetryError, RetryErrorKind},
    interceptor::{
        method_invocation::MethodInvocation, method_invocation_recoverer::ConstantRecoverer,
        retry_interceptor_builder::RetryInterceptorBuilder,
    },
    policy::{
        circuit_breaker_retry_policy::CircuitBreakerRetryPolicy,
        max_attempts_retry_policy::MaxAttemptsRetryPolicy,
    },
    recovery_callback::RecoveryCallback,
    retry_callback::with_fn,
    retry_context::RetryContext,
    retry_listener::RetryListener,
    retry_operations::RetryOperations,
    retry_state::DefaultRetryState,
    stats::{
        default_statistics_repository::DefaultStatisticsRepository,
        statistics_listener::StatisticsListener, statistics_repository::StatisticsRepository,
    },
    support::{
        retry_synchronization_manager::RetrySynchronizationManager, retry_template::RetryTemplate,
    },
};

fn counter() -> Arc<AtomicU16> {
    Arc::new(AtomicU16::new(0))
}

fn custom(message: &str) -> RetryError {
    RetryError::custom(message)
}

#[tokio::test]
async fn a_successful_call_is_not_retried() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Ok::<_, RetryError>("ok")
            })
        }))
        .await;

    assert_eq!(result.expect("the call should succeed"), "ok");
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn a_failing_call_is_retried_until_it_succeeds() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(4)
        .build()
        .execute(with_fn(move |context| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                assert_eq!(context.get_retry_count(), attempt - 1);
                if attempt < 3 {
                    return Err(custom("transient"));
                }
                Ok::<_, RetryError>(attempt)
            })
        }))
        .await;

    assert_eq!(result.expect("the call should succeed"), 3);
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn exhausting_the_attempts_returns_the_last_error() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(custom("still failing"))
            })
        }))
        .await;

    assert_eq!(
        result.expect_err("the call should fail"),
        custom("still failing")
    );
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn recovery_produces_the_value_after_exhaustion() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();
    let recovery = TestRecovery { value: 42 };

    let result = RetryTemplate::builder()
        .max_attempts(2)
        .build()
        .execute_with_recovery(
            with_fn(move |_| {
                let attempts = attempts_in_call.clone();
                Box::pin(async move {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    Err::<u16, _>(custom("recoverable"))
                })
            }),
            &recovery,
        )
        .await;

    assert_eq!(result.expect("recovery should succeed"), 42);
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn a_not_retry_on_error_stops_after_the_first_attempt() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(5)
        .not_retry_on(custom("fatal"))
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(custom("fatal"))
            })
        }))
        .await;

    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn a_retry_on_white_list_retries_matching_errors_only() {
    let retryable_attempts = counter();
    let retryable_in_call = retryable_attempts.clone();
    let _ = RetryTemplate::builder()
        .max_attempts(3)
        .retry_on(custom("retryable"))
        .build()
        .execute(with_fn(move |_| {
            let attempts = retryable_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(custom("retryable"))
            })
        }))
        .await;

    let rejected_attempts = counter();
    let rejected_in_call = rejected_attempts.clone();
    let _ = RetryTemplate::builder()
        .max_attempts(3)
        .retry_on(custom("retryable"))
        .build()
        .execute(with_fn(move |_| {
            let attempts = rejected_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(custom("other"))
            })
        }))
        .await;

    assert_eq!(retryable_attempts.load(Ordering::Relaxed), 3);
    assert_eq!(rejected_attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn infinite_retry_keeps_going_until_the_call_succeeds() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .infinite_retry()
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt < 5 {
                    return Err(custom("not yet"));
                }
                Ok::<_, RetryError>(attempt)
            })
        }))
        .await;

    assert_eq!(result.expect("the call should succeed"), 5);
    assert_eq!(attempts.load(Ordering::Relaxed), 5);
}

#[tokio::test]
async fn a_timeout_policy_stops_retrying_once_the_budget_is_spent() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .with_timeout(1)
        .fixed_backoff(5)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(custom("timeout"))
            })
        }))
        .await;

    assert!(result.is_err());
    assert!(attempts.load(Ordering::Relaxed) >= 1);
}

#[tokio::test]
async fn a_custom_back_off_runs_between_failed_attempts_only() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();
    let back_offs = Arc::new(AtomicU16::new(0));

    let result = RetryTemplate::builder()
        .max_attempts(4)
        .custom_backoff(CountingBackOffPolicy {
            calls: back_offs.clone(),
        })
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt < 4 {
                    return Err(custom("temporary"));
                }
                Ok::<_, RetryError>(attempt)
            })
        }))
        .await;

    assert_eq!(result.expect("the call should succeed"), 4);
    assert_eq!(back_offs.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn fixed_back_off_waits_the_configured_interval() {
    let recorder = Arc::new(RecordingSleeper::new());
    let mut policy = FixedBackOffPolicy::new();
    policy.set_sleeper(recorder.clone());
    policy.set_back_off_period(10);

    let start = Instant::now();
    let result = RetryTemplate::builder()
        .max_attempts(3)
        .custom_backoff(policy)
        .build()
        .execute(with_fn(|_| {
            Box::pin(async { Err::<(), _>(custom("always")) })
        }))
        .await;

    assert!(result.is_err());
    assert_eq!(recorder.periods(), vec![10, 10]);
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn listeners_observe_the_lifecycle_in_order() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();
    let events = Arc::new(Mutex::new(Vec::new()));

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(RecordingListener {
            events: events.clone(),
        })
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt == 1 {
                    return Err(custom("temporary"));
                }
                Ok::<_, RetryError>("done")
            })
        }))
        .await;

    assert_eq!(result.expect("the call should succeed"), "done");
    assert_eq!(
        events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default(),
        ["open", "error", "success", "close"]
    );
}

#[tokio::test]
async fn a_vetoing_listener_terminates_the_retry_before_the_first_attempt() {
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(VetoListener)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Ok::<_, RetryError>(())
            })
        }))
        .await;

    let error = result.expect_err("the retry should be terminated");
    assert_eq!(error.kind(), RetryErrorKind::TerminatedRetry);
    assert_eq!(attempts.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn context_attributes_survive_across_attempts() {
    let result = RetryTemplate::builder()
        .max_attempts(3)
        .build()
        .execute(with_fn(|context| {
            Box::pin(async move {
                let count = context
                    .get_attribute("attempts")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_default();
                context.set_attribute("attempts", (count + 1).into());

                if count < 2 {
                    return Err(custom("needs more attempts"));
                }

                Ok::<_, RetryError>(count + 1)
            })
        }))
        .await;

    assert_eq!(result.expect("the call should succeed"), 3);
}

#[tokio::test]
async fn a_nested_template_inherits_the_enclosing_context_as_parent() {
    let inner_parent_seen = Arc::new(Mutex::new(false));
    let inner_parent_in_call = inner_parent_seen.clone();
    let outer = RetryTemplate::builder().max_attempts(1).build();
    let inner = RetryTemplate::builder().max_attempts(1).build();

    let result = outer
        .execute(with_fn(move |_| {
            let inner = inner.clone();
            let seen = inner_parent_in_call.clone();
            Box::pin(async move {
                inner
                    .execute(with_fn(move |inner_context| {
                        let seen = seen.clone();
                        Box::pin(async move {
                            let parent_count = inner_context
                                .get_parent()
                                .map(|parent| parent.get_retry_count());
                            if let Ok(mut seen) = seen.lock() {
                                *seen = parent_count.is_some();
                            }
                            Ok::<_, RetryError>(1_u8)
                        })
                    }))
                    .await
            })
        }))
        .await;

    assert_eq!(result.expect("the nested retry should succeed"), 1);
    assert!(
        inner_parent_seen
            .lock()
            .map(|parent| *parent)
            .unwrap_or(false),
        "the inner retry must see the outer context as its parent"
    );
}

#[tokio::test]
async fn a_stateful_retry_makes_one_attempt_per_invocation() {
    let template = RetryTemplate::builder().max_attempts(2).build();
    let attempts = counter();
    let attempts_in_call = attempts.clone();
    let state = DefaultRetryState::new("item-1".to_owned());

    let fail = || {
        let attempts = attempts_in_call.clone();
        with_fn(move |_| {
            let attempts = attempts.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(custom("fatal"))
            })
        })
    };

    let first = template.execute_with_state(fail(), &state).await;
    assert!(first.is_err());
    assert_eq!(attempts.load(Ordering::Relaxed), 1);

    let second = template.execute_with_state(fail(), &state).await;
    assert!(second.is_err());
    assert_eq!(attempts.load(Ordering::Relaxed), 2);

    // The third invocation finds the retry exhausted and does not run the callback.
    let third = template.execute_with_state(fail(), &state).await;
    let error = third.expect_err("the retry should be exhausted");
    assert_eq!(error.kind(), RetryErrorKind::ExhaustedRetry);
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn a_stateful_retry_can_rethrow_the_last_error_instead() {
    let mut template = RetryTemplate::builder().max_attempts(1).build();
    template.set_throw_last_exception_on_exhausted(true);
    let state = DefaultRetryState::new("item-2".to_owned());

    let first = template
        .execute_with_state(
            with_fn(|_| Box::pin(async { Err::<u16, _>(custom("fatal")) })),
            &state,
        )
        .await;
    assert!(first.is_err());

    let second = template
        .execute_with_state(
            with_fn(|_| Box::pin(async { Ok::<u16, RetryError>(1) })),
            &state,
        )
        .await;

    assert_eq!(
        second.expect_err("the retry should rethrow the last error"),
        custom("fatal")
    );
}

#[tokio::test]
async fn a_circuit_breaker_short_circuits_after_the_delegate_gives_up() {
    let mut template = RetryTemplate::default();
    template.set_retry_policy(CircuitBreakerRetryPolicy::new(Arc::new(
        MaxAttemptsRetryPolicy::new(1),
    )));
    let attempts = counter();
    let attempts_in_call = attempts.clone();
    let state = DefaultRetryState::new("breaker".to_owned());

    let first = template
        .execute_with_state(
            with_fn(move |_| {
                let attempts = attempts_in_call.clone();
                Box::pin(async move {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    Err::<u16, _>(custom("boom"))
                })
            }),
            &state,
        )
        .await;
    assert!(first.is_err());

    let second = template
        .execute_with_state(
            with_fn({
                let attempts = attempts.clone();
                move |_| {
                    let attempts = attempts.clone();
                    Box::pin(async move {
                        attempts.fetch_add(1, Ordering::Relaxed);
                        Ok::<u16, RetryError>(1)
                    })
                }
            }),
            &state,
        )
        .await;

    assert!(second.is_err(), "an open circuit must fail without calling");
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn statistics_are_recorded_through_a_listener() {
    let repository = Arc::new(DefaultStatisticsRepository::new());
    let listener = StatisticsListener::with_default_name(repository.clone(), "orders");

    let result = RetryTemplate::builder()
        .max_attempts(2)
        .with_listener(listener)
        .build()
        .execute(with_fn(|_| {
            Box::pin(async { Err::<(), _>(custom("nope")) })
        }))
        .await;
    assert!(result.is_err());

    let statistics = repository
        .find_one("orders")
        .expect("the statistics should be recorded");
    // A stateless retry re-enters the callback once per error, so each error counts as a
    // started block; the close of the exhausted block adds the abort.
    assert_eq!(statistics.get_started_count(), 2);
    assert_eq!(statistics.get_error_count(), 2);
    assert_eq!(statistics.get_abort_count(), 1);
}

#[tokio::test]
async fn the_stateless_interceptor_retries_and_recovers() {
    let mut interceptor = RetryInterceptorBuilder::stateless::<u64>()
        .max_attempts(3)
        .label("catalog")
        .build();
    interceptor.set_recoverer(Arc::new(ConstantRecoverer::new(7_u64)));
    let attempts = counter();
    let attempts_in_call = attempts.clone();

    let result = interceptor
        .invoke(
            MethodInvocation::new("Catalog::lookup", vec![AnyValue::from(1_u64)]),
            move || {
                let attempts = attempts_in_call.clone();
                async move {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    Err::<u64, _>(custom("unavailable"))
                }
            },
        )
        .await;

    assert_eq!(result.expect("recovery should succeed"), 7);
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn the_stateful_interceptor_resumes_across_invocations() {
    let interceptor = RetryInterceptorBuilder::stateful::<u64>()
        .max_attempts(2)
        .label("orders")
        .build();
    let attempts = counter();
    let attempts_in_call = attempts.clone();
    let invocation = || MethodInvocation::new("Orders::process", vec![AnyValue::from("o-1")]);

    let first = interceptor
        .invoke(invocation(), move || {
            let attempts = attempts_in_call.clone();
            async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u64, _>(custom("conflict"))
            }
        })
        .await;
    assert!(first.is_err());

    let second = interceptor
        .invoke(invocation(), {
            let attempts = attempts.clone();
            move || {
                let attempts = attempts.clone();
                async move {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    Ok::<u64, RetryError>(9)
                }
            }
        })
        .await;

    assert_eq!(second.expect("the second attempt should succeed"), 9);
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn one_template_serves_concurrent_tasks() {
    let template = RetryTemplate::builder().max_attempts(3).build();
    let successes = Arc::new(AtomicUsize::new(0));

    let mut tasks = Vec::new();
    for task in 0..8_u16 {
        let template = template.clone();
        let successes = successes.clone();
        tasks.push(tokio::spawn(async move {
            let attempts = Arc::new(AtomicU16::new(0));
            let attempts_in_call = attempts.clone();
            let result = template
                .execute(with_fn(move |context| {
                    let attempts = attempts_in_call.clone();
                    Box::pin(async move {
                        let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                        if attempt <= task % 3 {
                            return Err(custom("transient"));
                        }
                        assert!(context.get_retry_count() < 3);
                        Ok::<_, RetryError>(attempt)
                    })
                }))
                .await;
            if result.is_ok() {
                successes.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for task in tasks {
        task.await.expect("the task should finish");
    }

    assert_eq!(successes.load(Ordering::Relaxed), 8);
}

#[tokio::test]
async fn the_current_context_is_visible_inside_a_callback() {
    let seen = Arc::new(Mutex::new(false));
    let seen_in_call = seen.clone();

    let result = RetryTemplate::builder()
        .max_attempts(1)
        .build()
        .execute(with_fn(move |_| {
            let seen = seen_in_call.clone();
            Box::pin(async move {
                if let Ok(mut seen) = seen.lock() {
                    *seen = RetrySynchronizationManager::get_context().is_some();
                }
                Ok::<_, RetryError>(())
            })
        }))
        .await;

    assert!(result.is_ok());
    assert!(
        seen.lock().map(|seen| *seen).unwrap_or(false),
        "the callback must see its own retry as the current one"
    );
    assert!(
        RetrySynchronizationManager::get_context().is_none(),
        "the context must be cleared once the retry returns"
    );
}

struct TestRecovery {
    value: u16,
}

#[next_web_core::async_trait]
impl RecoveryCallback<u16> for TestRecovery {
    async fn recover(&self, context: &dyn RetryContext) -> Result<u16, RetryError> {
        assert_eq!(context.get_retry_count(), 2);
        assert!(context.is_exhausted());
        assert!(context.get_last_error().is_some());
        Ok(self.value)
    }
}

#[derive(Clone)]
struct CountingBackOffPolicy {
    calls: Arc<AtomicU16>,
}

#[next_web_core::async_trait]
impl BackOffPolicy for CountingBackOffPolicy {
    async fn start(
        &self,
        _context: &dyn RetryContext,
    ) -> Option<Arc<dyn next_web_retry::backoff::back_off_context::BackOffContext>> {
        None
    }

    async fn backoff(
        &self,
        _context: Option<&dyn next_web_retry::backoff::back_off_context::BackOffContext>,
    ) -> Result<(), RetryError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

struct RecordingListener {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl RetryListener for RecordingListener {
    fn open(&self, _context: &dyn RetryContext) -> bool {
        if let Ok(mut events) = self.events.lock() {
            events.push("open");
        }
        true
    }

    fn close(&self, _context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        if let Ok(mut events) = self.events.lock() {
            events.push("close");
        }
    }

    fn on_success(&self, _context: &dyn RetryContext, _result: &dyn Any) {
        if let Ok(mut events) = self.events.lock() {
            events.push("success");
        }
    }

    fn on_error(&self, _context: &dyn RetryContext, _error: &dyn AnyError) {
        if let Ok(mut events) = self.events.lock() {
            events.push("error");
        }
    }
}

struct VetoListener;

impl RetryListener for VetoListener {
    fn open(&self, _context: &dyn RetryContext) -> bool {
        false
    }
}

/// Silences the unused-import warning for the sleeper trait in builds without the timing test.
#[allow(dead_code)]
fn assert_sleeping_policy<P: SleepingBackOffPolicy>(_policy: &P) {}
