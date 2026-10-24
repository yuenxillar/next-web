//! The retry itself.
//!
//! A retry repeats a call that failed, waiting between attempts. It is the
//! right tool for a transient fault — a dropped connection, a momentary
//! timeout, a leader election — and the wrong tool for a call that already
//! changed something. **Only retry operations that are safe to run twice.**
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::retry::{Retry, RetryConfig};
//!
//! # async fn fetch() -> Result<String, std::io::Error> {
//! #     Ok("payload".to_owned())
//! # }
//! # async fn example() {
//! let retry = Retry::new(
//!     "inventory",
//!     RetryConfig::builder()
//!         .max_attempts(3)
//!         .wait_duration(Duration::from_millis(200))
//!         .build(),
//! );
//!
//! let payload = retry.call(fetch).await.expect("inventory should answer");
//! # let _ = payload;
//! # }
//! ```

use std::{error::Error, future::Future, sync::atomic::AtomicU64, time::Duration};

use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use super::{
    retry_config::{RetryConfig, RetryConfigBuilder},
    retry_error::RetryError,
    retry_event::RetryEvent,
    retry_snapshot::RetrySnapshot,
};
use crate::{error::MaxRetriesExceededError, event::EventPublisher};

/// The message reported when a result based retry runs out of attempts.
const UNACCEPTABLE_RESULT: &str = "the guarded call returned a result the retry predicate rejects";

/// Counters that accumulate since the retry was created.
#[derive(Debug, Default)]
struct Counters {
    successful_without_retry: AtomicU64,
    successful_with_retry: AtomicU64,
    failed_without_retry: AtomicU64,
    failed_with_retry: AtomicU64,
    retries: AtomicU64,
}

/// A retry policy guarding one operation.
///
/// Wrap it in an [`Arc`](std::sync::Arc) to share it: the retry owns the
/// counters that describe its behaviour, so cloning it would split that state.
///
/// [`Retry::call`] retries a call that returns an error.
/// [`Retry::call_on_result`] additionally retries a call that *succeeded* but
/// returned a value the caller does not accept.
#[derive(Debug)]
pub struct Retry {
    /// Name of the guarded operation, used in logs and metrics.
    name: String,
    /// Settings of this retry.
    config: RetryConfig,
    /// Totals of the calls this retry drove.
    counters: Counters,
    /// Fan-out of the events this retry emits.
    events: EventPublisher<RetryEvent>,
}

impl Retry {
    /// Creates a retry for the operation called `name`.
    pub fn new(name: impl Into<String>, config: RetryConfig) -> Self {
        Self {
            name: name.into(),
            config,
            counters: Counters::default(),
            events: EventPublisher::new(),
        }
    }

    /// Creates a retry for `name` using the default settings.
    pub fn of(name: impl Into<String>) -> Self {
        Self::new(name, RetryConfig::default())
    }

    /// Starts building a retry for the operation called `name`.
    pub fn builder(name: impl Into<String>) -> RetryBuilder {
        RetryBuilder {
            name: name.into(),
            config: RetryConfigBuilder::default(),
        }
    }

    /// Returns the name of the guarded operation.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the settings of this retry.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Subscribes to the events this retry emits from now on.
    pub fn event_stream(&self) -> broadcast::Receiver<RetryEvent> {
        self.events.subscribe()
    }

    /// Returns the counters of this retry.
    pub fn snapshot(&self) -> RetrySnapshot {
        RetrySnapshot {
            number_of_successful_calls_without_retry: self
                .counters
                .successful_without_retry
                .load(std::sync::atomic::Ordering::Relaxed),
            number_of_successful_calls_with_retry: self
                .counters
                .successful_with_retry
                .load(std::sync::atomic::Ordering::Relaxed),
            number_of_failed_calls_without_retry: self
                .counters
                .failed_without_retry
                .load(std::sync::atomic::Ordering::Relaxed),
            number_of_failed_calls_with_retry: self
                .counters
                .failed_with_retry
                .load(std::sync::atomic::Ordering::Relaxed),
            number_of_retries: self
                .counters
                .retries
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Calls `guarded`, retrying it while it fails.
    ///
    /// The closure is called again for every further attempt, so it must be
    /// safe to run more than once.
    ///
    /// # Errors
    ///
    /// Returns [`RetryError::Inner`] with the failure of the last attempt, or
    /// [`RetryError::MaxRetriesExceeded`] when the configuration asks for a
    /// spent attempt budget to be reported.
    pub async fn call<T, E, F, Fut>(&self, guarded: F) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Error + 'static,
    {
        self.execute(guarded, None).await
    }

    /// Calls `guarded`, retrying it while it fails **or** while `retry_on_result`
    /// asks for the returned value to be retried.
    ///
    /// `retry_on_result` follows Resilience4j's `retryOnResult`: it returns
    /// `true` when the value warrants another attempt, not when the value is
    /// acceptable. Use it to retry a call that succeeded but answered something
    /// useless — an empty page, a `null` payload, a "try again later" status.
    ///
    /// A value the predicate asks to retry is treated exactly like a failure.
    /// When the attempt budget is spent:
    ///
    /// * with `fail_after_max_attempts(false)`, the default, the last value is
    ///   returned, because the final attempt is the best answer available;
    /// * with `fail_after_max_attempts(true)`, the call fails with
    ///   [`RetryError::MaxRetriesExceeded`].
    ///
    /// # Errors
    ///
    /// As [`Retry::call`], plus [`RetryError::MaxRetriesExceeded`] for a
    /// retryable value when `fail_after_max_attempts` is enabled.
    pub async fn call_on_result<T, E, F, Fut, P>(
        &self,
        guarded: F,
        retry_on_result: P,
    ) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        P: Fn(&T) -> bool + Send + Sync,
        E: Error + 'static,
    {
        self.execute(guarded, Some(&retry_on_result)).await
    }

    /// Drives the attempt loop that both public entry points share.
    async fn execute<T, E, F, Fut>(
        &self,
        mut guarded: F,
        retry_on_result: Option<&(dyn Fn(&T) -> bool + Send + Sync)>,
    ) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Error + 'static,
    {
        let max_attempts = self.config.max_attempts();
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;

            let started = self.config.clock().monotonic_time();
            let outcome = guarded().await;
            let duration = self
                .config
                .clock()
                .monotonic_time()
                .saturating_sub(started);

            match outcome {
                Ok(value) => {
                    // The predicate answers whether the value should be retried,
                    // matching Resilience4j's `retryOnResult`; it is not an
                    // acceptance test.
                    let must_retry =
                        retry_on_result.is_some_and(|should_retry| should_retry(&value));
                    if !must_retry {
                        self.record_success(attempt, duration);
                        return Ok(value);
                    }

                    if attempt >= max_attempts {
                        return self.finish_without_failure(attempt, value);
                    }

                    let wait = self.prepare_next_attempt(attempt, UNACCEPTABLE_RESULT.to_owned());
                    self.wait(wait).await;
                }
                Err(error) => {
                    if !self.should_retry(&error) {
                        self.record_ignored_error(attempt, error.to_string());
                        return Err(RetryError::Inner(error));
                    }

                    if attempt >= max_attempts {
                        return self.finish_with_failure(attempt, error);
                    }

                    let message = error.to_string();
                    let wait = self.prepare_next_attempt(attempt, message);
                    self.wait(wait).await;
                }
            }
        }
    }

    /// Returns whether this policy retries the failure of an attempt.
    ///
    /// This is the decision [`Retry::call`] takes after every failed attempt,
    /// exposed so that a composed decorator driving its own attempt loop
    /// applies the same rules.
    pub fn should_retry(&self, error: &(dyn Error + 'static)) -> bool {
        let ignored = self
            .config
            .ignore_exception_predicate()
            .is_some_and(|ignore| ignore(error));
        if ignored {
            return false;
        }

        (self.config.retry_exception_predicate())(error)
    }

    /// Announces the next attempt and returns how long to wait for it.
    ///
    /// This counts the retry and publishes [`RetryEvent::Retry`], so it is the
    /// counterpart of [`Retry::should_retry`] for a decorator that drives its
    /// own attempt loop and wants this retry's snapshot to stay accurate.
    pub fn prepare_next_attempt(&self, attempt: u32, message: String) -> Duration {
        let wait = self.config.interval_function().apply(attempt);
        self.counters
            .retries
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        self.events.publish(RetryEvent::Retry {
            attempt,
            wait,
            message,
        });
        debug!(retry = %self.name, attempt, ?wait, "retrying the guarded call");

        wait
    }

    /// Sleeps for `wait`, unless there is nothing to wait for.
    async fn wait(&self, wait: Duration) {
        if wait.is_zero() {
            return;
        }
        tokio::time::sleep(wait).await;
    }

    /// Ends a result based retry whose value stayed unacceptable.
    fn finish_without_failure<T, E>(&self, attempt: u32, value: T) -> Result<T, RetryError<E>> {
        self.record_failure(attempt, None);

        if self.config.fail_after_max_attempts() {
            return Err(RetryError::MaxRetriesExceeded(
                MaxRetriesExceededError::new(attempt, None),
            ));
        }

        Ok(value)
    }

    /// Ends a retry whose last attempt failed.
    fn finish_with_failure<T, E>(&self, attempt: u32, error: E) -> Result<T, RetryError<E>>
    where
        E: std::fmt::Display,
    {
        self.record_failure(attempt, Some(error.to_string()));

        if self.config.fail_after_max_attempts() {
            return Err(RetryError::MaxRetriesExceeded(
                MaxRetriesExceededError::new(attempt, Some(error)),
            ));
        }

        Err(RetryError::Inner(error))
    }

    /// Records that a call eventually succeeded after `attempts` attempts, and
    /// announces it.
    ///
    /// This is the counterpart of [`Retry::should_retry`] for a decorator that
    /// drives its own attempt loop: it keeps this retry's snapshot and event
    /// stream accurate without going through [`Retry::call`].
    pub fn record_success(&self, attempts: u32, duration: Duration) {
        self.count_success(attempts);
        self.events.publish(RetryEvent::Success {
            attempts,
            duration,
        });
        info!(retry = %self.name, attempts, "guarded call succeeded");
    }

    /// Records that a call failed after `attempts` attempts, and announces it.
    ///
    /// The attempt budget is treated as spent, so this publishes
    /// [`RetryEvent::MaxAttemptsReached`].
    pub fn record_failure(&self, attempts: u32, message: Option<String>) {
        self.count_failure(attempts);
        self.events.publish(RetryEvent::MaxAttemptsReached {
            attempts,
            message,
        });
        warn!(retry = %self.name, attempts, "retry attempts exhausted");
    }

    /// Records that a call failed with an error the policy refuses to retry, and
    /// announces it.
    ///
    /// This is the counterpart of [`Retry::should_retry`] for a decorator that
    /// drives its own attempt loop: a failure the policy will not retry is a
    /// failed call, but it is not a spent attempt budget, so it must not be
    /// reported as [`RetryEvent::MaxAttemptsReached`].
    pub fn record_ignored_error(&self, attempts: u32, message: String) {
        self.count_failure(attempts);
        self.events.publish(RetryEvent::IgnoredError { message });
        debug!(
            retry = %self.name,
            attempts,
            "failure is not retryable, giving up"
        );
    }

    fn count_success(&self, attempts: u32) {
        let counter = if attempts <= 1 {
            &self.counters.successful_without_retry
        } else {
            &self.counters.successful_with_retry
        };
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn count_failure(&self, attempts: u32) {
        let counter = if attempts <= 1 {
            &self.counters.failed_without_retry
        } else {
            &self.counters.failed_with_retry
        };
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Builds a [`Retry`] together with its configuration.
pub struct RetryBuilder {
    name: String,
    config: RetryConfigBuilder,
}

impl RetryBuilder {
    /// Finishes the configuration and creates the retry.
    ///
    /// # Panics
    ///
    /// Panics when a setting is outside its documented range.
    pub fn build(self) -> Retry {
        Retry::new(self.name, self.config.build())
    }

    /// Adjusts the configuration before building.
    ///
    /// Every setter of [`RetryConfigBuilder`] is available through this method.
    pub fn config(
        mut self,
        configure: impl FnOnce(RetryConfigBuilder) -> RetryConfigBuilder,
    ) -> Self {
        self.config = configure(self.config);
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{
            Arc, Mutex,
            atomic::{AtomicU32, Ordering},
        },
        time::Duration,
    };

    use super::Retry;
    use crate::retry::{
        interval_function::{FixedIntervalFunction, IntervalFunction},
        retry_config::RetryConfig,
        retry_error::RetryError,
    };

    /// An error a policy is expected to leave alone.
    #[derive(Debug)]
    struct BusinessRejection;

    impl std::fmt::Display for BusinessRejection {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("rejected by the business rules")
        }
    }

    impl std::error::Error for BusinessRejection {}

    /// Records the attempt numbers the retry asks about, so a test can prove
    /// the interval function is consulted once per failed attempt.
    #[derive(Debug, Clone)]
    struct RecordingIntervalFunction {
        attempts: Arc<Mutex<Vec<u32>>>,
    }

    impl IntervalFunction for RecordingIntervalFunction {
        fn apply(&self, attempt: u32) -> Duration {
            if let Ok(mut recorded) = self.attempts.lock() {
                recorded.push(attempt);
            }
            Duration::ZERO
        }
    }

    fn failure() -> io::Error {
        io::Error::other("boom")
    }

    /// A configuration that retries immediately, so tests do not spend time
    /// waiting.
    fn immediate_config(max_attempts: u32) -> RetryConfig {
        RetryConfig::builder()
            .max_attempts(max_attempts)
            .interval_function(FixedIntervalFunction::new(Duration::ZERO))
            .build()
    }

    #[tokio::test]
    async fn a_call_that_succeeds_immediately_is_not_retried() {
        let retry = Retry::new("test", immediate_config(3));
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let value = retry
            .call(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, io::Error>(7)
                }
            })
            .await
            .expect("the first attempt succeeds");

        assert_eq!(value, 7);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(retry.snapshot().number_of_successful_calls_without_retry, 1);
        assert_eq!(retry.snapshot().number_of_retries, 0);
    }

    #[tokio::test]
    async fn a_call_that_fails_then_succeeds_returns_its_value() {
        let retry = Retry::new("test", immediate_config(4));
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let value = retry
            .call(move || {
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
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(retry.snapshot().number_of_successful_calls_with_retry, 1);
        assert_eq!(retry.snapshot().number_of_retries, 2);
    }

    #[tokio::test]
    async fn a_persistent_failure_gives_up_after_max_attempts() {
        let retry = Retry::new("test", immediate_config(3));
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let error = retry
            .call(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(failure())
                }
            })
            .await
            .expect_err("every attempt fails");

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(!error.is_max_retries_exceeded());
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
        assert_eq!(retry.snapshot().number_of_failed_calls_with_retry, 1);
    }

    #[tokio::test]
    async fn fail_after_max_attempts_reports_the_spent_budget() {
        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(2)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .fail_after_max_attempts(true)
                .build(),
        );

        let error = retry
            .call(|| async { Err::<(), _>(failure()) })
            .await
            .expect_err("every attempt fails");

        assert!(error.is_max_retries_exceeded());
        assert_eq!(error.attempts(), Some(2));
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
    }

    #[tokio::test]
    async fn a_non_retryable_failure_is_not_retried() {
        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(5)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .retry_on_exception(Arc::new(|_| false))
                .build(),
        );
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let error = retry
            .call(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(failure())
                }
            })
            .await
            .expect_err("the failure is not retryable");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(retry.snapshot().number_of_failed_calls_without_retry, 1);
        assert!(!error.is_max_retries_exceeded());
    }

    #[tokio::test]
    async fn an_ignored_failure_is_not_retried() {
        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(5)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .ignore_exception(Arc::new(
                    |error: &(dyn std::error::Error + 'static)| {
                        error.downcast_ref::<BusinessRejection>().is_some()
                    },
                ))
                .build(),
        );
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let error = retry
            .call(move || {
                let probe = probe.clone();
                async move {
                    probe.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(BusinessRejection)
                }
            })
            .await
            .expect_err("a business rejection is final");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(matches!(error, RetryError::Inner(BusinessRejection)));
    }

    #[tokio::test]
    async fn an_ignored_failure_is_announced_on_the_event_stream() {
        use crate::retry::retry_event::RetryEvent;

        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(5)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .retry_on_exception(Arc::new(|_| false))
                .build(),
        );
        let mut events = retry.event_stream();

        let _ = retry.call(|| async { Err::<(), _>(failure()) }).await;

        assert!(matches!(
            events.recv().await,
            Ok(RetryEvent::IgnoredError { .. })
        ));
    }

    #[tokio::test]
    async fn the_interval_function_is_asked_once_per_failed_attempt() {
        let recorder = RecordingIntervalFunction {
            attempts: Arc::new(Mutex::new(Vec::new())),
        };
        let observed = recorder.attempts.clone();
        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(4)
                .interval_function(recorder)
                .build(),
        );

        let _ = retry.call(|| async { Err::<(), _>(failure()) }).await;

        // Four attempts, so three waits; the fourth failure ends the call.
        let recorded = observed.lock().expect("the recorder is not poisoned");
        assert_eq!(*recorded, vec![1, 2, 3]);
    }

    #[tokio::test(start_paused = true)]
    async fn the_configured_wait_is_observed_before_retrying() {
        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(2)
                .wait_duration(Duration::from_secs(5))
                .build(),
        );

        let started = tokio::time::Instant::now();
        let _ = retry.call(|| async { Err::<(), _>(failure()) }).await;

        assert_eq!(started.elapsed(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn a_result_based_retry_repeats_until_the_value_is_acceptable() {
        let retry = Retry::new("test", immediate_config(4));
        let values = Arc::new(AtomicU32::new(0));
        let probe = values.clone();

        let value = retry
            .call_on_result(
                move || {
                    let probe = probe.clone();
                    async move { Ok::<_, io::Error>(probe.fetch_add(1, Ordering::SeqCst)) }
                },
                |value: &u32| *value < 2,
            )
            .await
            .expect("the third value is acceptable");

        assert_eq!(value, 2);
        assert_eq!(retry.snapshot().number_of_successful_calls_with_retry, 1);
    }

    #[tokio::test]
    async fn a_result_based_retry_returns_the_last_value_by_default() {
        let retry = Retry::new("test", immediate_config(3));
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let value = retry
            .call_on_result(
                move || {
                    let probe = probe.clone();
                    async move {
                        probe.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, io::Error>(0u32)
                    }
                },
                |_: &u32| true,
            )
            .await
            .expect("the last value is returned when the budget is spent");

        assert_eq!(value, 0);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(retry.snapshot().number_of_failed_calls_with_retry, 1);
    }

    #[tokio::test]
    async fn a_result_based_retry_can_fail_when_the_budget_is_spent() {
        let retry = Retry::new(
            "test",
            RetryConfig::builder()
                .max_attempts(2)
                .interval_function(FixedIntervalFunction::new(Duration::ZERO))
                .fail_after_max_attempts(true)
                .build(),
        );

        let error = retry
            .call_on_result(|| async { Ok::<_, io::Error>(0u32) }, |_: &u32| true)
            .await
            .expect_err("the budget is reported as spent");

        assert!(error.is_max_retries_exceeded());
        assert_eq!(error.attempts(), Some(2));
        assert!(error.inner_error().is_none());
    }

    #[tokio::test]
    async fn a_result_based_retry_still_retries_failures() {
        let retry = Retry::new("test", immediate_config(3));
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let value = retry
            .call_on_result(
                move || {
                    let probe = probe.clone();
                    async move {
                        let attempt = probe.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt < 2 {
                            Err(failure())
                        } else {
                            Ok(attempt)
                        }
                    }
                },
                // The predicate never asks for a value to be retried, so only
                // the failure on the first attempt can drive the retry.
                |_: &u32| false,
            )
            .await
            .expect("the second attempt succeeds");

        assert_eq!(value, 2);
    }

    #[tokio::test]
    async fn events_report_the_retry_and_the_eventual_success() {
        use crate::retry::retry_event::RetryEvent;

        let retry = Retry::new("test", immediate_config(3));
        let mut events = retry.event_stream();
        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();

        let _ = retry
            .call(move || {
                let probe = probe.clone();
                async move {
                    let attempt = probe.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 2 {
                        Err(failure())
                    } else {
                        Ok(attempt)
                    }
                }
            })
            .await;

        assert!(matches!(
            events.recv().await,
            Ok(RetryEvent::Retry { attempt: 1, .. })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(RetryEvent::Success { attempts: 2, .. })
        ));
    }

    #[tokio::test]
    async fn exhaustion_is_announced_on_the_event_stream() {
        use crate::retry::retry_event::RetryEvent;

        let retry = Retry::new("test", immediate_config(2));
        let mut events = retry.event_stream();

        let _ = retry.call(|| async { Err::<(), _>(failure()) }).await;

        assert!(matches!(
            events.recv().await,
            Ok(RetryEvent::Retry { attempt: 1, .. })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(RetryEvent::MaxAttemptsReached { attempts: 2, .. })
        ));
    }

    #[tokio::test]
    async fn the_snapshot_separates_calls_that_needed_a_retry() {
        let retry = Retry::new("test", immediate_config(3));

        let _ = retry.call(|| async { Ok::<_, io::Error>(1) }).await;

        let calls = Arc::new(AtomicU32::new(0));
        let probe = calls.clone();
        let _ = retry
            .call(move || {
                let probe = probe.clone();
                async move {
                    if probe.fetch_add(1, Ordering::SeqCst) == 0 {
                        Err(failure())
                    } else {
                        Ok(2)
                    }
                }
            })
            .await;

        let _ = retry.call(|| async { Err::<i32, _>(failure()) }).await;

        let snapshot = retry.snapshot();
        assert_eq!(snapshot.number_of_successful_calls_without_retry, 1);
        assert_eq!(snapshot.number_of_successful_calls_with_retry, 1);
        assert_eq!(snapshot.number_of_failed_calls_with_retry, 1);
        assert_eq!(snapshot.number_of_failed_calls_without_retry, 0);
        assert_eq!(snapshot.number_of_retries, 3);
        assert_eq!(snapshot.number_of_calls(), 3);
    }
}
