//! The paths a production deployment actually leans on.
//!
//! The integration suite in `retry_template_tests.rs` covers the happy paths. This file covers
//! the failure paths and the invariants that decide whether a retry layer is safe to run in
//! production: what error reaches the caller, when recovery is skipped, whether a stateful
//! schedule survives between calls, what happens when the cache is full or loses an entry, and
//! whether a template can be shared by many tasks.

use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    },
    time::Duration,
};

use next_web_core::any::{AnyError, AnyValue};
use next_web_retry::{
    backoff::{back_off_context::BackOffContext, back_off_policy::BackOffPolicy},
    error::retry_error::{RetryError, RetryErrorKind},
    interceptor::{
        fixed_key_generator::FixedKeyGenerator,
        new_method_arguments_identifier::NewMethodArgumentsIdentifier,
        retry_interceptor_builder::RetryInterceptorBuilder,
    },
    policy::{
        circuit_breaker_retry_policy::CircuitBreakerRetryPolicy,
        map_retry_context_cache::MapRetryContextCache,
        max_attempts_retry_policy::MaxAttemptsRetryPolicy, retry_context_cache::RetryContextCache,
        simple_retry_policy::SimpleRetryPolicy,
    },
    recovery_callback::RecoveryCallback,
    retry_callback::{RetryCallback, with_fn},
    retry_context::RetryContext,
    retry_listener::RetryListener,
    retry_operations::RetryOperations,
    retry_state::DefaultRetryState,
    support::retry_template::RetryTemplate,
};

fn custom(message: &str) -> RetryError {
    RetryError::custom(message)
}

/// A callback that always fails, usable several times.
fn failing<T>() -> impl RetryCallback<T>
where
    T: Send + 'static,
{
    with_fn(|_| Box::pin(async { Err::<T, _>(custom("boom")) }))
}

/// A callback that always fails with `message`.
fn failing_with<T>(message: &'static str) -> impl RetryCallback<T>
where
    T: Send + 'static,
{
    with_fn(move |_| Box::pin(async move { Err::<T, _>(custom(message)) }))
}

/// A callback that succeeds with `value`.
fn succeeding<T>(value: T) -> impl RetryCallback<T>
where
    T: Send + Sync + Clone + 'static,
{
    with_fn(move |_| {
        let value = value.clone();
        Box::pin(async move { Ok::<T, RetryError>(value) })
    })
}

#[tokio::test]
async fn exhaustion_returns_the_callback_error_untouched() {
    let error = RetryError::exhausted_with_cause("gave up", Box::new(custom("inner")));
    let error_in_call = error.clone();

    let result = RetryTemplate::builder()
        .max_attempts(2)
        .build()
        .execute(with_fn(move |_| {
            let error = error_in_call.clone();
            Box::pin(async move { Err::<(), _>(error) })
        }))
        .await;

    let returned = result.expect_err("the retry should fail");
    assert_eq!(returned.kind(), RetryErrorKind::ExhaustedRetry);
    assert_eq!(returned.message(), "gave up");
    assert_eq!(returned.to_string(), error.to_string());
    assert!(returned.cause().is_some(), "the cause must survive");
}

#[tokio::test]
async fn an_interrupted_back_off_aborts_the_retry_and_still_closes() {
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_in_call = attempts.clone();
    let closes = Arc::new(AtomicU16::new(0));
    let closes_in_listener = closes.clone();

    let result = RetryTemplate::builder()
        .max_attempts(5)
        .custom_backoff(InterruptingBackOff)
        .with_listener(CloseCounter {
            closes: closes_in_listener,
        })
        .build()
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(custom("transient"))
            })
        }))
        .await;

    let error = result.expect_err("an interrupted wait must abort");
    assert!(error.is_back_off_interrupted(), "got {error:?}");
    assert_eq!(
        attempts.load(Ordering::Relaxed),
        1,
        "an interrupted wait means no further attempt"
    );
    assert_eq!(
        closes.load(Ordering::Relaxed),
        1,
        "close hooks must run even when the wait is interrupted"
    );
}

#[tokio::test]
async fn a_not_recoverable_error_skips_the_recovery_callback() {
    let mut policy = SimpleRetryPolicy::new(1, HashMap::new(), false, true);
    policy.set_not_recoverable(vec![custom("fatal")]);
    let mut template = RetryTemplate::default();
    template.set_retry_policy(policy);

    let skipped = CountingRecovery::new(42);
    let result = template
        .execute_with_recovery(failing_with("fatal"), &skipped)
        .await;

    assert_eq!(result.expect_err("the error must surface"), custom("fatal"));
    assert_eq!(
        skipped.calls(),
        0,
        "a non-recoverable error must not reach the recoverer"
    );

    // The same failure through a policy that keeps recovery enabled is recovered.
    let recoverable_policy = SimpleRetryPolicy::new(1, HashMap::new(), false, true);
    let mut recoverable_template = RetryTemplate::default();
    recoverable_template.set_retry_policy(recoverable_policy);
    let recovered = CountingRecovery::new(42);
    let value = recoverable_template
        .execute_with_recovery(failing_with("fatal"), &recovered)
        .await
        .expect("recovery should produce a value");

    assert_eq!(value, 42);
    assert_eq!(recovered.calls(), 1);
}

#[tokio::test]
async fn a_stateful_recovery_runs_on_the_resumed_call_and_marks_the_context() {
    let flags = Arc::new(Mutex::new(None));
    let flags_in_listener = flags.clone();
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(FlagListener {
            seen: flags_in_listener,
        })
        .build();
    let state = DefaultRetryState::new("item".to_owned());
    let recovery = ValidatingRecovery { value: 7 };

    let first = template
        .execute_with_all(failing::<u16>(), &recovery, &state)
        .await;
    assert!(first.is_err(), "the first call must rethrow");

    let second = template
        .execute_with_all(succeeding(1_u16), &recovery, &state)
        .await;
    assert_eq!(
        second.expect("the resumed call should recover"),
        7,
        "the recovery value must be returned"
    );

    let (exhausted, recovered, closed) = flags
        .lock()
        .map(|flags| flags.expect("the listener must have run"))
        .expect("the flags must be readable");
    assert!(exhausted, "an exhausted context must be reported");
    assert!(recovered, "a recovered context must be reported");
    assert!(closed, "a recovered block is closed by the template");
}

#[tokio::test]
async fn a_stateful_retry_keeps_its_backoff_schedule_between_calls() {
    let recorder = Arc::new(next_web_retry::backoff::sleeper::RecordingSleeper::new());
    let mut policy =
        next_web_retry::backoff::exponential_back_off_policy::ExponentialBackOffPolicy::new();
    policy.set_sleeper(recorder.clone());
    policy.set_initial_interval(10);
    policy.set_multiplier(2.0);
    policy.set_max_interval(1_000);

    let template = RetryTemplate::builder()
        .max_attempts(4)
        .custom_backoff(policy)
        .build();
    let state = DefaultRetryState::new("item".to_owned());

    for _ in 0..3 {
        let _ = template.execute_with_state(failing::<u16>(), &state).await;
    }

    assert_eq!(
        recorder.periods(),
        vec![10, 20, 40],
        "the exponential schedule must continue across invocations"
    );
}

#[tokio::test]
async fn a_full_cache_surfaces_as_a_registration_failure() {
    let mut template = RetryTemplate::builder().max_attempts(1).build();
    template.set_retry_context_cache(MapRetryContextCache::with_capacity_and_eviction(1, false));

    let first = template
        .execute_with_state(
            failing::<u16>(),
            &DefaultRetryState::new("first".to_owned()),
        )
        .await;
    assert!(first.is_err());

    let second = template
        .execute_with_state(
            failing::<u16>(),
            &DefaultRetryState::new("second".to_owned()),
        )
        .await;

    let error = second.expect_err("a full cache must reject the context");
    assert_eq!(error.kind(), RetryErrorKind::TerminatedRetry);
    let cause = error
        .cause()
        .and_then(|cause| cause.as_any().downcast_ref::<RetryError>())
        .expect("the registration failure must be the cause");
    assert_eq!(cause.kind(), RetryErrorKind::RetryCacheCapacityExceeded);
}

#[tokio::test]
async fn a_cache_entry_that_expires_after_being_read_is_reported() {
    let mut template = RetryTemplate::builder().max_attempts(2).build();
    template.set_retry_context_cache(OneShotCache::default());
    let state = DefaultRetryState::new("item".to_owned());

    let first = template.execute_with_state(failing::<u16>(), &state).await;
    assert!(first.is_err());

    let second = template.execute_with_state(failing::<u16>(), &state).await;

    let error = second.expect_err("losing the entry must be reported");
    assert_eq!(error.kind(), RetryErrorKind::TerminatedRetry);
    let cause = error
        .cause()
        .and_then(|cause| cause.as_any().downcast_ref::<RetryError>())
        .expect("the inconsistency must be the cause");
    assert!(
        cause.message().contains("cache key has changed"),
        "unexpected message: {}",
        cause.message()
    );
}

#[tokio::test]
async fn concurrent_stateful_retries_do_not_share_state() {
    let template = RetryTemplate::builder().max_attempts(3).build();
    let mut tasks = Vec::new();

    for id in 0..16_u16 {
        let template = template.clone();
        tasks.push(tokio::spawn(async move {
            let state = DefaultRetryState::new(format!("item-{id}"));
            let attempts = Arc::new(AtomicU16::new(0));
            let attempts_in_call = attempts.clone();

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

            let second = template.execute_with_state(succeeding(id), &state).await;

            (attempts.load(Ordering::Relaxed), second.ok())
        }));
    }

    for (index, task) in tasks.into_iter().enumerate() {
        let (attempts, result) = task.await.expect("the task should finish");
        assert_eq!(attempts, 1, "task {index} retried more than once");
        assert_eq!(
            result,
            Some(index as u16),
            "task {index} did not resume its own state"
        );
    }
}

#[tokio::test]
async fn a_listener_opening_sees_the_name_and_the_attempt_budget() {
    let seen = Arc::new(Mutex::new(None));
    let seen_in_listener = seen.clone();

    let template = RetryTemplate::builder()
        .max_attempts(5)
        .with_listener(OpenObserver {
            seen: seen_in_listener,
        })
        .build();

    let _ = template.execute(LabeledCallback::new("orders")).await;

    let (name, max_attempts) = seen
        .lock()
        .ok()
        .and_then(|seen| seen.clone())
        .expect("the listener must have run");
    assert_eq!(name, Some("orders".to_owned()));
    assert_eq!(
        max_attempts,
        Some(5),
        "the attempt budget must be published before the open hook"
    );
}

#[tokio::test]
async fn a_circuit_breaker_attempts_again_after_the_reset_timeout() {
    let mut policy = CircuitBreakerRetryPolicy::new(Arc::new(MaxAttemptsRetryPolicy::new(1)));
    policy.set_open_timeout(15);
    policy.set_reset_timeout(15);
    let mut template = RetryTemplate::default();
    template.set_retry_policy(policy);
    let state = DefaultRetryState::new("breaker".to_owned());
    let attempts = Arc::new(AtomicU16::new(0));
    let attempts_in_call = attempts.clone();

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

    // While the circuit is open the callback is not reached.
    let short_circuited = template.execute_with_state(succeeding(1_u16), &state).await;
    assert!(short_circuited.is_err());
    assert_eq!(attempts.load(Ordering::Relaxed), 1);

    tokio::time::sleep(Duration::from_millis(30)).await;

    // After the reset timeout the circuit closes and the call is attempted again.
    let recovered = template.execute_with_state(succeeding(9_u16), &state).await;
    assert_eq!(recovered.expect("the circuit should close"), 9);
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn a_raw_key_shares_the_state_between_different_calls() {
    let mut shared = RetryInterceptorBuilder::stateful::<u16>()
        .max_attempts(1)
        .build();
    shared.set_key_generator(Arc::new(FixedKeyGenerator::new("shared")));
    shared.set_use_raw_key(true);

    let first = shared
        .invoke(
            next_web_retry::interceptor::method_invocation::MethodInvocation::without_arguments(
                "A",
            ),
            || async { Err::<u16, _>(custom("boom")) },
        )
        .await;
    assert!(first.is_err());

    let calls = Arc::new(AtomicU16::new(0));
    let calls_in_call = calls.clone();
    let second = shared
        .invoke(
            next_web_retry::interceptor::method_invocation::MethodInvocation::without_arguments(
                "B",
            ),
            move || {
                let calls = calls_in_call.clone();
                async move {
                    calls.fetch_add(1, Ordering::Relaxed);
                    Ok::<u16, RetryError>(1)
                }
            },
        )
        .await;

    assert!(
        matches!(second, Err(RetryError::ExhaustedRetryError(_))),
        "a shared raw key must resume the same state"
    );
    assert_eq!(
        calls.load(Ordering::Relaxed),
        0,
        "a resumed exhausted state must not call again"
    );

    // Without the raw key the call name participates in the key, so the state is separate.
    let mut qualified = RetryInterceptorBuilder::stateful::<u16>()
        .max_attempts(1)
        .build();
    qualified.set_key_generator(Arc::new(FixedKeyGenerator::new("shared")));

    let first = qualified
        .invoke(
            next_web_retry::interceptor::method_invocation::MethodInvocation::without_arguments(
                "A",
            ),
            || async { Err::<u16, _>(custom("boom")) },
        )
        .await;
    assert!(first.is_err());

    let calls = Arc::new(AtomicU16::new(0));
    let calls_in_call = calls.clone();
    let second = qualified
        .invoke(
            next_web_retry::interceptor::method_invocation::MethodInvocation::without_arguments(
                "B",
            ),
            move || {
                let calls = calls_in_call.clone();
                async move {
                    calls.fetch_add(1, Ordering::Relaxed);
                    Ok::<u16, RetryError>(1)
                }
            },
        )
        .await;

    assert_eq!(second.expect("a separate state must attempt"), 1);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn a_new_arguments_identifier_forces_a_fresh_context() {
    let mut interceptor = RetryInterceptorBuilder::stateful::<u16>()
        .max_attempts(2)
        .build();
    interceptor.set_new_arguments_identifier(Arc::new(AlwaysNew));

    let invocation = || {
        next_web_retry::interceptor::method_invocation::MethodInvocation::new(
            "Service::call",
            vec![AnyValue::from("item")],
        )
    };

    let first = interceptor
        .invoke(invocation(), || async { Err::<u16, _>(custom("boom")) })
        .await;
    assert!(first.is_err());

    let calls = Arc::new(AtomicU16::new(0));
    let calls_in_call = calls.clone();
    let second = interceptor
        .invoke(invocation(), move || {
            let calls = calls_in_call.clone();
            async move {
                calls.fetch_add(1, Ordering::Relaxed);
                Ok::<u16, RetryError>(5)
            }
        })
        .await;

    assert_eq!(
        second.expect("a fresh context must attempt"),
        5,
        "a fresh context must not resume the exhausted one"
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn the_exponential_schedule_is_monotonic_and_capped() {
    use next_web_retry::backoff::exponential_back_off_policy::ExponentialBackOffContext;

    let context = ExponentialBackOffContext::new(100, 1.5, 1_000, false, None, None, None);
    let mut previous = 0_u64;

    for _ in 0..100 {
        let sleep = context.get_sleep_and_increment();
        assert!(sleep >= previous, "the schedule must not go backwards");
        assert!(sleep <= 1_000, "the schedule must be capped at the maximum");
        previous = sleep;
    }

    assert_eq!(previous, 1_000, "the schedule must reach the maximum");
}

#[tokio::test]
async fn a_back_off_context_is_reused_within_a_stateful_block() {
    let recorder = Arc::new(next_web_retry::backoff::sleeper::RecordingSleeper::new());
    let mut policy =
        next_web_retry::backoff::exponential_back_off_policy::ExponentialBackOffPolicy::new();
    policy.set_sleeper(recorder.clone());
    policy.set_initial_interval(5);
    policy.set_multiplier(2.0);
    policy.set_max_interval(1_000);

    let template = RetryTemplate::builder()
        .max_attempts(4)
        .custom_backoff(policy)
        .build();
    let state = DefaultRetryState::new("item".to_owned());

    for _ in 0..2 {
        let _ = template.execute_with_state(failing::<u16>(), &state).await;
    }

    assert_eq!(
        recorder.periods(),
        vec![5, 10],
        "the same back-off context must be reused across invocations"
    );
}

/// A back-off policy that reports an interruption on every wait.
struct InterruptingBackOff;
#[next_web_core::async_trait]
impl BackOffPolicy for InterruptingBackOff {
    async fn start(&self, _context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        None
    }

    async fn backoff(&self, _context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        Err(RetryError::back_off_interrupted("waiting was cancelled"))
    }
}

struct CloseCounter {
    closes: Arc<AtomicU16>,
}

impl RetryListener for CloseCounter {
    fn close(&self, _context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        self.closes.fetch_add(1, Ordering::Relaxed);
    }
}

struct CountingRecovery {
    value: u16,
    calls: Arc<AtomicU16>,
}

impl CountingRecovery {
    fn new(value: u16) -> Self {
        Self {
            value,
            calls: Arc::new(AtomicU16::new(0)),
        }
    }

    fn calls(&self) -> u16 {
        self.calls.load(Ordering::Relaxed)
    }
}

#[next_web_core::async_trait]
impl RecoveryCallback<u16> for CountingRecovery {
    async fn recover(&self, _context: &dyn RetryContext) -> Result<u16, RetryError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.value)
    }
}

struct ValidatingRecovery {
    value: u16,
}

#[next_web_core::async_trait]
impl RecoveryCallback<u16> for ValidatingRecovery {
    async fn recover(&self, context: &dyn RetryContext) -> Result<u16, RetryError> {
        assert!(
            context.is_exhausted(),
            "recovery runs on an exhausted block"
        );
        assert!(
            !context.is_recovered(),
            "the recovered flag is set after the recoverer returns"
        );
        assert!(context.get_last_error().is_some());
        Ok(self.value)
    }
}

struct FlagListener {
    seen: Arc<Mutex<Option<(bool, bool, bool)>>>,
}

impl RetryListener for FlagListener {
    fn close(&self, context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some((
                context.is_exhausted(),
                context.is_recovered(),
                context.is_closed(),
            ));
        }
    }
}

/// A cache that drops an entry as soon as it is read, like one that expires under pressure.
#[derive(Default)]
struct OneShotCache {
    inner: MapRetryContextCache,
}

impl RetryContextCache for OneShotCache {
    fn get(&self, key: &str) -> Option<Arc<dyn RetryContext>> {
        let context = self.inner.get(key);
        self.inner.remove(key);
        context
    }

    fn put(&self, key: &str, context: Arc<dyn RetryContext>) -> Result<(), RetryError> {
        self.inner.put(key, context)
    }

    fn remove(&self, key: &str) {
        self.inner.remove(key);
    }

    fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }
}

/// What the open hook observed: the reporting name and the attempt budget.
type OpenObservation = (Option<String>, Option<u16>);

struct OpenObserver {
    seen: Arc<Mutex<Option<OpenObservation>>>,
}

impl RetryListener for OpenObserver {
    fn open(&self, context: &dyn RetryContext) -> bool {
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some((context.get_name(), context.get_max_attempts()));
        }
        true
    }
}

struct LabeledCallback {
    label: &'static str,
}

impl LabeledCallback {
    fn new(label: &'static str) -> Self {
        Self { label }
    }
}

#[next_web_core::async_trait]
impl RetryCallback<u16> for LabeledCallback {
    async fn do_with_retry(&self, _context: &dyn RetryContext) -> Result<u16, RetryError> {
        Err(custom("boom"))
    }

    fn label(&self) -> Option<&str> {
        Some(self.label)
    }
}

struct AlwaysNew;

impl NewMethodArgumentsIdentifier for AlwaysNew {
    fn is_new(&self, _arguments: &[AnyValue]) -> bool {
        true
    }
}

#[tokio::test]
async fn a_field_rule_decides_which_errors_are_retried() {
    let attempts_5xx = Arc::new(AtomicU16::new(0));
    let attempts_in_call = attempts_5xx.clone();
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .retry_on_error_if::<HttpError, _>(|error| error.status >= 500)
        .build();

    let result = template
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(http_error(503))
            })
        }))
        .await;
    assert!(result.is_err());
    assert_eq!(
        attempts_5xx.load(Ordering::Relaxed),
        3,
        "a 5xx must be retried up to the limit"
    );

    let attempts_4xx = Arc::new(AtomicU16::new(0));
    let attempts_in_call = attempts_4xx.clone();
    let result = template
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(http_error(404))
            })
        }))
        .await;
    assert!(result.is_err());
    assert_eq!(
        attempts_4xx.load(Ordering::Relaxed),
        1,
        "a 4xx must not be retried by a white list of 5xx"
    );
}

#[tokio::test]
async fn a_type_rule_can_reject_an_owned_error_type() {
    let rejected_attempts = Arc::new(AtomicU16::new(0));
    let attempts_in_call = rejected_attempts.clone();
    let template = RetryTemplate::builder()
        .max_attempts(2)
        .not_retry_on_error_type::<HttpError>()
        .build();

    let result = template
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(http_error(500))
            })
        }))
        .await;
    assert!(result.is_err());
    assert_eq!(rejected_attempts.load(Ordering::Relaxed), 1);

    // Everything else keeps the black-list default, which is to retry.
    let other_attempts = Arc::new(AtomicU16::new(0));
    let attempts_in_call = other_attempts.clone();
    let result = template
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(custom("some other failure"))
            })
        }))
        .await;
    assert!(result.is_err());
    assert_eq!(other_attempts.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn cause_traversal_lets_a_wrapped_error_match_a_type_rule() {
    let wrapped_attempts = Arc::new(AtomicU16::new(0));
    let attempts_in_call = wrapped_attempts.clone();
    let template = RetryTemplate::builder()
        .max_attempts(2)
        .retry_on_error_type::<HttpError>()
        .traversing_error_causes()
        .build();

    let _ = template
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(RetryError::retry_with_cause(
                    "wrapper",
                    Box::new(http_error(503)),
                ))
            })
        }))
        .await;
    assert_eq!(
        wrapped_attempts.load(Ordering::Relaxed),
        2,
        "the wrapped type must be found through the cause chain"
    );

    // Without traversal the wrapper does not match, so the white list rejects it.
    let shallow_attempts = Arc::new(AtomicU16::new(0));
    let attempts_in_call = shallow_attempts.clone();
    let shallow = RetryTemplate::builder()
        .max_attempts(2)
        .retry_on_error_type::<HttpError>()
        .build();

    let _ = shallow
        .execute(with_fn(move |_| {
            let attempts = attempts_in_call.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::Relaxed);
                Err::<u16, _>(RetryError::retry_with_cause(
                    "wrapper",
                    Box::new(http_error(503)),
                ))
            })
        }))
        .await;
    assert_eq!(shallow_attempts.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn a_panicking_callback_still_runs_the_close_hooks() {
    let closes = Arc::new(AtomicU16::new(0));
    let closes_in_listener = closes.clone();
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(CloseCounter {
            closes: closes_in_listener,
        })
        .build();

    let handle = tokio::spawn(async move {
        let _ = template
            .execute(with_fn(|_| {
                Box::pin(async move {
                    panic!("the callback exploded");
                    #[allow(unreachable_code)]
                    Ok::<u16, RetryError>(0)
                })
            }))
            .await;
    });

    let join = handle.await;
    let error = join.expect_err("the task must report the panic");
    assert!(error.is_panic(), "the join error must be a panic");
    assert_eq!(
        closes.load(Ordering::Relaxed),
        1,
        "close hooks must run while the stack unwinds"
    );
}

#[tokio::test]
async fn a_cancelled_retry_still_runs_the_close_hooks() {
    let closes = Arc::new(AtomicU16::new(0));
    let closes_in_listener = closes.clone();
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(CloseCounter {
            closes: closes_in_listener,
        })
        .build();

    let started = Arc::new(tokio::sync::Notify::new());
    let started_in_call = started.clone();
    let handle = tokio::spawn(async move {
        let _ = template
            .execute(with_fn(move |_| {
                let started = started_in_call.clone();
                Box::pin(async move {
                    started.notify_one();
                    std::future::pending::<()>().await;
                    Ok::<u16, RetryError>(0)
                })
            }))
            .await;
    });

    started.notified().await;
    handle.abort();
    let _ = handle.await;

    assert_eq!(
        closes.load(Ordering::Relaxed),
        1,
        "a dropped retry must still close its policies and listeners"
    );
}

/// An owned error type, the way a service layer defines one.
#[derive(Clone, Debug, PartialEq, Eq)]
struct HttpError {
    status: u16,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "http {}", self.status)
    }
}

impl std::error::Error for HttpError {}

impl Hash for HttpError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.status.hash(state);
    }
}

fn http_error(status: u16) -> RetryError {
    RetryError::Any(Box::new(HttpError { status }))
}

/// Captures whether the close hook saw a block that did not finish.
struct AbortedProbe {
    seen: Arc<Mutex<Option<bool>>>,
}

impl RetryListener for AbortedProbe {
    fn close(&self, context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        if let Ok(mut seen) = self.seen.lock() {
            *seen =
                Some(context.flag(next_web_retry::retry_context::retry_context_constants::ABORTED));
        }
    }
}

#[tokio::test]
async fn a_completed_retry_is_not_marked_aborted() {
    let seen = Arc::new(Mutex::new(None));
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(AbortedProbe { seen: seen.clone() })
        .build();

    let _ = template.execute(succeeding(1_u16)).await;

    assert_eq!(seen.lock().ok().and_then(|seen| *seen), Some(false));
}

#[tokio::test]
async fn a_panicking_retry_is_marked_aborted() {
    let seen = Arc::new(Mutex::new(None));
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(AbortedProbe { seen: seen.clone() })
        .build();

    let handle = tokio::spawn(async move {
        let _ = template
            .execute(with_fn(|_| {
                Box::pin(async move {
                    panic!("the callback exploded");
                    #[allow(unreachable_code)]
                    Ok::<u16, RetryError>(0)
                })
            }))
            .await;
    });
    let _ = handle.await;

    assert_eq!(seen.lock().ok().and_then(|seen| *seen), Some(true));
}

#[tokio::test]
async fn a_cancelled_retry_is_marked_aborted() {
    let seen = Arc::new(Mutex::new(None));
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(AbortedProbe { seen: seen.clone() })
        .build();
    let started = Arc::new(tokio::sync::Notify::new());
    let started_in_call = started.clone();

    let handle = tokio::spawn(async move {
        let _ = template
            .execute(with_fn(move |_| {
                let started = started_in_call.clone();
                Box::pin(async move {
                    started.notify_one();
                    std::future::pending::<()>().await;
                    Ok::<u16, RetryError>(0)
                })
            }))
            .await;
    });
    started.notified().await;
    handle.abort();
    let _ = handle.await;

    assert_eq!(seen.lock().ok().and_then(|seen| *seen), Some(true));
}

#[tokio::test]
async fn a_stateful_rethrow_is_recorded_as_failed() {
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(2)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .build();
    let state = DefaultRetryState::new("item".to_owned());

    let result = template
        .execute_with_state(
            with_fn(|context| {
                context.set_attribute(
                    next_web_retry::retry_context::retry_context_constants::NAME,
                    "orders".into(),
                );
                Box::pin(async { Err::<u16, _>(custom("boom")) })
            }),
            &state,
        )
        .await;
    assert!(result.is_err());

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"failed\"} 1"),
        "a rethrown stateful failure must be reported as failed, got:\n{text}"
    );
}

#[tokio::test]
async fn an_interrupted_back_off_is_recorded_as_failed() {
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .custom_backoff(InterruptingBackOff)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .build();

    let result = template
        .execute(with_fn(|context| {
            context.set_attribute(
                next_web_retry::retry_context::retry_context_constants::NAME,
                "orders".into(),
            );
            Box::pin(async { Err::<u16, _>(custom("transient")) })
        }))
        .await;
    assert!(result.is_err());

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"failed\"} 1"),
        "an interrupted block must not be reported as success, got:\n{text}"
    );
}

#[tokio::test]
async fn a_vetoed_retry_is_recorded_as_failed() {
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .with_listener(VetoingListener)
        .build();

    let result = template.execute(LabeledCallback::new("orders")).await;
    assert!(result.is_err(), "the listener must veto the retry");

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"failed\"} 1"),
        "a vetoed block must not be reported as success, got:\n{text}"
    );
}
/// A listener that vetoes every retry before the first attempt.
struct VetoingListener;

impl RetryListener for VetoingListener {
    fn open(&self, _context: &dyn RetryContext) -> bool {
        false
    }
}

#[tokio::test]
async fn an_aborted_stateful_block_does_not_poison_the_next_invocation() {
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .build();

    // Invocation 1: a normal failure, which caches the context under the item key.
    let first = template
        .execute_with_state(
            LabeledCallback::new("orders"),
            &DefaultRetryState::new("orders-1".to_owned()),
        )
        .await;
    assert!(first.is_err());

    // Invocation 2: the callback panics, so the block is aborted on the cached context.
    let panicking = template.clone();
    let handle = tokio::spawn(async move {
        let _ = panicking
            .execute_with_state(
                with_fn(|context| {
                    context.set_attribute(
                        next_web_retry::retry_context::retry_context_constants::NAME,
                        "orders".into(),
                    );
                    Box::pin(async move {
                        panic!("the callback exploded");
                        #[allow(unreachable_code)]
                        Ok::<u16, RetryError>(0)
                    })
                }),
                &DefaultRetryState::new("orders-1".to_owned()),
            )
            .await;
    });
    let _ = handle.await;

    // Invocation 3: a success on the same item must not inherit the aborted flag.
    let third = template
        .execute_with_state(
            with_fn(|context| {
                context.set_attribute(
                    next_web_retry::retry_context::retry_context_constants::NAME,
                    "orders".into(),
                );
                Box::pin(async { Ok::<u16, RetryError>(7) })
            }),
            &DefaultRetryState::new("orders-1".to_owned()),
        )
        .await;
    assert_eq!(third.expect("the resumed invocation should succeed"), 7);

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"aborted\"} 1"),
        "the panicking invocation must be the only aborted block, got:\n{text}"
    );
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"success\"} 1"),
        "the successful invocation must not inherit the aborted flag, got:\n{text}"
    );
}

#[tokio::test]
async fn an_aborted_circuit_breaker_block_does_not_poison_the_next_invocation() {
    use next_web_retry::policy::circuit_breaker_retry_policy::CircuitBreakerRetryPolicy;
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let mut template = RetryTemplate::default();
    template.set_retry_policy(CircuitBreakerRetryPolicy::new(Arc::new(
        MaxAttemptsRetryPolicy::new(1),
    )));
    template.set_listeners(vec![Arc::new(MetricsRetryListener::new(metrics.clone()))]);

    // A circuit-breaker context is global: it stays in the cache after the block closes.
    let panicking = template.clone();
    let handle = tokio::spawn(async move {
        let _ = panicking
            .execute_with_state(
                with_fn(|context| {
                    context.set_attribute(
                        next_web_retry::retry_context::retry_context_constants::NAME,
                        "breaker".into(),
                    );
                    Box::pin(async move {
                        panic!("the callback exploded");
                        #[allow(unreachable_code)]
                        Ok::<u16, RetryError>(0)
                    })
                }),
                &DefaultRetryState::new("breaker".to_owned()),
            )
            .await;
    });
    let _ = handle.await;

    // The next invocation of the same key succeeds; it must not inherit the aborted flag.
    let next = template
        .execute_with_state(
            with_fn(|context| {
                context.set_attribute(
                    next_web_retry::retry_context::retry_context_constants::NAME,
                    "breaker".into(),
                );
                Box::pin(async { Ok::<u16, RetryError>(9) })
            }),
            &DefaultRetryState::new("breaker".to_owned()),
        )
        .await;
    assert_eq!(next.expect("the circuit should allow the call"), 9);

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"breaker\",outcome=\"success\"} 1"),
        "the successful invocation must not inherit the aborted flag, got:\n{text}"
    );
}

#[tokio::test]
async fn a_vetoed_circuit_breaker_block_does_not_poison_the_next_invocation() {
    use std::sync::atomic::{AtomicBool, Ordering};

    use next_web_retry::policy::circuit_breaker_retry_policy::CircuitBreakerRetryPolicy;
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    /// Vetoes the first block only, so the next invocation can succeed.
    struct VetoOnce {
        vetoed: Arc<AtomicBool>,
    }

    impl RetryListener for VetoOnce {
        fn open(&self, _context: &dyn RetryContext) -> bool {
            self.vetoed.swap(true, Ordering::AcqRel)
        }
    }

    let metrics = Arc::new(RetryMetrics::new());
    let mut template = RetryTemplate::default();
    template.set_retry_policy(CircuitBreakerRetryPolicy::new(Arc::new(
        MaxAttemptsRetryPolicy::new(1),
    )));
    template.set_listeners(vec![
        Arc::new(MetricsRetryListener::new(metrics.clone())),
        Arc::new(VetoOnce {
            vetoed: Arc::new(AtomicBool::new(false)),
        }),
    ]);

    let vetoed = template
        .execute_with_state(
            with_fn(|context| {
                context.set_attribute(
                    next_web_retry::retry_context::retry_context_constants::NAME,
                    "breaker".into(),
                );
                Box::pin(async { Ok::<u16, RetryError>(1) })
            }),
            &DefaultRetryState::new("breaker".to_owned()),
        )
        .await;
    assert!(vetoed.is_err(), "the first block must be vetoed");

    let next = template
        .execute_with_state(
            with_fn(|context| {
                context.set_attribute(
                    next_web_retry::retry_context::retry_context_constants::NAME,
                    "breaker".into(),
                );
                Box::pin(async { Ok::<u16, RetryError>(9) })
            }),
            &DefaultRetryState::new("breaker".to_owned()),
        )
        .await;
    assert_eq!(next.expect("the second block should succeed"), 9);

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"breaker\",outcome=\"success\"} 1"),
        "the successful invocation must not inherit the terminated flag, got:\n{text}"
    );
}

#[tokio::test]
async fn a_first_attempt_success_records_one_block_with_a_non_zero_duration() {
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .build();

    let result = template
        .execute(with_fn(|context| {
            context.set_attribute(
                next_web_retry::retry_context::retry_context_constants::NAME,
                "orders".into(),
            );
            Box::pin(async {
                tokio::task::yield_now().await;
                Ok::<u16, RetryError>(1)
            })
        }))
        .await;
    assert_eq!(result.expect("the first attempt should succeed"), 1);

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"success\"} 1"),
        "got:\n{text}"
    );
    assert!(text.contains("next_web_retry_attempt_errors_total{name=\"orders\"} 0"));
    assert!(text.contains("next_web_retry_block_duration_seconds_count{name=\"orders\"} 1"));
    assert!(
        !text.contains("next_web_retry_block_duration_seconds_sum{name=\"orders\"} 0.000000"),
        "a fast block must still report a non-zero duration, got:\n{text}"
    );
}

#[tokio::test]
async fn each_stateful_invocation_records_one_block() {
    use next_web_retry::support::metrics_retry_listener::MetricsRetryListener;
    use next_web_retry::support::retry_metrics::RetryMetrics;

    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(5)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .build();
    let state = DefaultRetryState::new("item".to_owned());

    for _ in 0..3 {
        let result = template
            .execute_with_state(LabeledCallback::new("orders"), &state)
            .await;
        assert!(result.is_err());
    }

    let text = metrics.render_prometheus();
    assert!(
        text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"failed\"} 3"),
        "three invocations must be three blocks, got:\n{text}"
    );
    assert!(text.contains("next_web_retry_attempt_errors_total{name=\"orders\"} 3"));
}
