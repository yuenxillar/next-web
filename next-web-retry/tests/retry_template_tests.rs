use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    },
};

use next_web_core::{
    anys::{any_error::AnyError, any_value::AnyValue},
    async_trait,
};
use next_web_retry::{
    backoff::{back_off_context::BackOffContext, back_off_policy::BackOffPolicy},
    error::retry_error::RetryError,
    recovery_callback::RecoveryCallback,
    retry_callback::with_fn,
    retry_context::RetryContext,
    retry_listener::RetryListener,
    retry_operations::RetryOperations,
    support::retry_template::RetryTemplate,
};

fn retry_error(msg: &str) -> RetryError {
    RetryError::Custom(msg.to_string())
}

#[tokio::test]
async fn succeeds_without_retrying() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Ok::<_, RetryError>("ok")
            })
        }))
        .await;

    assert_eq!(result.unwrap(), "ok");
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn retries_until_callback_succeeds() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(4)
        .build()
        .execute(with_fn(move |context| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                assert_eq!(context.get_retry_count(), attempt - 1);
                if attempt < 3 {
                    return Err(retry_error("transient"));
                }
                Ok::<_, RetryError>(attempt)
            })
        }))
        .await;

    assert_eq!(result.unwrap(), 3);
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn stops_after_max_attempts_are_exhausted() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(retry_error("still failing"))
            })
        }))
        .await;

    assert!(matches!(result, Err(RetryError::Default(_))));
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn recovery_callback_runs_after_exhaustion() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();
    let recovery = TestRecovery {
        retry_count_seen: Arc::new(AtomicU16::new(0)),
    };
    let retry_count_seen = recovery.retry_count_seen.clone();

    let result = RetryTemplate::builder()
        .max_attempts(2)
        .build()
        .execute_with_recovery(
            with_fn(move |_| {
                let attempts = attempts_for_callback.clone();
                Box::pin(async move {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    Err::<u16, _>(retry_error("recoverable"))
                })
            }),
            &recovery,
        )
        .await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    assert_eq!(retry_count_seen.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn not_retry_on_stops_after_first_matching_error() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(5)
        .not_retry_on(retry_error("fatal"))
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(retry_error("fatal"))
            })
        }))
        .await;

    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn retry_on_retries_matching_error_and_rejects_other_errors() {
    let retryable_attempts = Arc::new(AtomicU16::new(0));
    let retryable_attempts_for_callback = retryable_attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .retry_on(retry_error("retryable"))
        .build()
        .execute(with_fn(move |_| {
            let attempts = retryable_attempts_for_callback.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(retry_error("retryable"))
            })
        }))
        .await;

    assert!(result.is_err());
    assert_eq!(retryable_attempts.load(Ordering::Relaxed), 3);

    let rejected_attempts = Arc::new(AtomicU16::new(0));
    let rejected_attempts_for_callback = rejected_attempts.clone();

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .retry_on(retry_error("retryable"))
        .build()
        .execute(with_fn(move |_| {
            let attempts = rejected_attempts_for_callback.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(retry_error("other"))
            })
        }))
        .await;

    assert!(result.is_err());
    assert_eq!(rejected_attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn infinite_retry_keeps_retrying_until_success() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();

    let result = RetryTemplate::builder()
        .infinite_retry()
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt < 5 {
                    return Err(retry_error("not yet"));
                }
                Ok::<_, RetryError>(attempt)
            })
        }))
        .await;

    assert_eq!(result.unwrap(), 5);
    assert_eq!(attempts.load(Ordering::Relaxed), 5);
}

#[tokio::test]
async fn timeout_policy_stops_retrying_after_timeout() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();

    let result = RetryTemplate::builder()
        .with_timeout(1)
        .fixed_backoff(2)
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(retry_error("timeout"))
            })
        }))
        .await;

    assert!(result.is_err());
    assert!(attempts.load(Ordering::Relaxed) >= 1);
}

#[tokio::test]
async fn custom_backoff_runs_between_failed_attempts_only() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();
    let backoffs = Arc::new(AtomicU16::new(0));

    let result = RetryTemplate::builder()
        .max_attempts(4)
        .custom_backoff(CountingBackOffPolicy {
            calls: backoffs.clone(),
        })
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt < 4 {
                    return Err(retry_error("temporary"));
                }
                Ok::<_, RetryError>(attempt)
            })
        }))
        .await;

    assert_eq!(result.unwrap(), 4);
    assert_eq!(attempts.load(Ordering::Relaxed), 4);
    assert_eq!(backoffs.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn retry_listener_observes_lifecycle_events() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_for_callback = attempts.clone();
    let events = Arc::new(Mutex::new(Vec::new()));

    let result = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(RecordingListener {
            events: events.clone(),
        })
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_for_callback.clone();
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt == 1 {
                    return Err(retry_error("temporary"));
                }
                Ok::<_, RetryError>("done")
            })
        }))
        .await;

    assert_eq!(result.unwrap(), "done");
    assert_eq!(
        events.lock().unwrap().as_slice(),
        ["open", "error", "success", "close"]
    );
}

#[tokio::test]
async fn retry_context_attributes_survive_across_attempts() {
    let result = RetryTemplate::builder()
        .max_attempts(3)
        .build()
        .execute(with_fn(|context| {
            Box::pin(async move {
                let count = context
                    .get_attribute("attempts")
                    .and_then(|value| value.as_number())
                    .unwrap_or_default();
                context.set_attribute("attempts", AnyValue::Number(count + 1));

                if count < 2 {
                    return Err(retry_error("needs more attempts"));
                }

                Ok::<_, RetryError>(count + 1)
            })
        }))
        .await;

    assert_eq!(result.unwrap(), 3);
}

struct TestRecovery {
    retry_count_seen: Arc<AtomicU16>,
}

impl RecoveryCallback<u16> for TestRecovery {
    fn recover(&self, context: &dyn RetryContext) -> Result<u16, Box<dyn Error>> {
        self.retry_count_seen
            .store(context.get_retry_count(), Ordering::Relaxed);
        Ok(42)
    }
}

#[derive(Clone)]
struct CountingBackOffPolicy {
    calls: Arc<AtomicU16>,
}

#[async_trait]
impl BackOffPolicy for CountingBackOffPolicy {
    async fn start(&self, _context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        None
    }

    async fn backoff(&self, _context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

struct RecordingListener {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl RetryListener for RecordingListener {
    fn open(&self, _context: &dyn RetryContext) -> bool {
        self.events.lock().unwrap().push("open");
        true
    }

    fn close(&self, _context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        self.events.lock().unwrap().push("close");
    }

    fn on_success(&self, _context: &dyn RetryContext, _result: &dyn std::any::Any) {
        self.events.lock().unwrap().push("success");
    }

    fn on_error(&self, _context: &dyn RetryContext, _error: &dyn AnyError) {
        self.events.lock().unwrap().push("error");
    }
}
