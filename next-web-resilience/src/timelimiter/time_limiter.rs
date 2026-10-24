//! The time limiter itself.
//!
//! A time limiter caps how long a guarded call may take. It does not make a slow
//! dependency faster; it makes sure the caller of a slow dependency gets an
//! answer on time, so one sluggish resource cannot hold a whole request path
//! hostage.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::timelimiter::{TimeLimiter, TimeLimiterConfig};
//!
//! # async fn example() {
//! let limiter = TimeLimiter::new(
//!     "payments",
//!     TimeLimiterConfig::builder()
//!         .timeout_duration(Duration::from_secs(2))
//!         .build(),
//! );
//!
//! let result = limiter
//!     .call(|| async { Ok::<_, std::io::Error>("charged") })
//!     .await;
//!
//! assert!(result.is_ok());
//! # }
//! ```
//!
//! # Cancellation
//!
//! [`TimeLimiter::call`] wraps the future of the guarded operation in
//! [`tokio::time::timeout`]. When the limit is reached, that future is **dropped
//! where it stands**: nothing polls it again, so any work it owns is abandoned.
//! For an async operation that is exactly how cancellation is expressed in Rust,
//! which is what `cancel_running_future = true` means.
//!
//! Dropping a future is not the same as stopping the work behind it. If the
//! future spawned its own task, or handed work to a thread pool or an operating
//! system call, that work keeps running; it simply never reports back. When the
//! result is still wanted — because the operation is expensive to repeat, or
//! because it must observe its own completion — use
//! [`TimeLimiter::call_non_cancelling`], which detaches the call instead of
//! cancelling it, and read its documentation for the price of doing so.

use std::{
    fmt,
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
};

use tokio::{
    sync::broadcast,
    time::{Instant, timeout},
};
use tracing::{debug, warn};

use super::{
    time_limiter_config::TimeLimiterConfig,
    time_limiter_error::TimeLimiterError,
    time_limiter_event::TimeLimiterEvent,
    time_limiter_snapshot::TimeLimiterSnapshot,
};
use crate::{
    error::{AbortedError, TimeoutError},
    event::EventPublisher,
};

/// A time limiter guarding one resource.
///
/// Wrap it in an [`Arc`](std::sync::Arc) to share it: the limiter owns the
/// counters its snapshot reports, so cloning it would split that state.
///
/// The limiter is stateless apart from those counters — every call is independent
/// of the calls before it — so unlike a circuit breaker it never needs a reset
/// and cannot be opened by past failures.
#[derive(Debug)]
pub struct TimeLimiter {
    /// Name of the guarded resource, used in logs and metrics.
    name: String,
    /// Settings of this limiter.
    config: TimeLimiterConfig,
    /// Calls that returned a value.
    number_of_successful_calls: AtomicU64,
    /// Calls that failed or ended without a result.
    number_of_failed_calls: AtomicU64,
    /// Calls that outlived the time limit.
    number_of_timeouts: AtomicU64,
    /// Fan-out of the events this limiter emits.
    events: EventPublisher<TimeLimiterEvent>,
}

impl TimeLimiter {
    /// Creates a limiter for the resource called `name`.
    pub fn new(name: impl Into<String>, config: TimeLimiterConfig) -> Self {
        Self {
            name: name.into(),
            config,
            number_of_successful_calls: AtomicU64::new(0),
            number_of_failed_calls: AtomicU64::new(0),
            number_of_timeouts: AtomicU64::new(0),
            events: EventPublisher::new(),
        }
    }

    /// Creates a limiter for `name` using the default settings.
    pub fn of(name: impl Into<String>) -> Self {
        Self::new(name, TimeLimiterConfig::default())
    }

    /// Returns the name of the guarded resource.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the settings of this limiter.
    pub fn config(&self) -> &TimeLimiterConfig {
        &self.config
    }

    /// Returns whether the guarded future is cancelled once the limit is
    /// reached.
    ///
    /// Shorthand for [`TimeLimiterConfig::cancel_running_future`].
    pub fn cancel_running_future(&self) -> bool {
        self.config.cancel_running_future()
    }

    /// Subscribes to the events this limiter emits from now on.
    pub fn event_stream(&self) -> broadcast::Receiver<TimeLimiterEvent> {
        self.events.subscribe()
    }

    /// Returns the counters of this limiter.
    pub fn snapshot(&self) -> TimeLimiterSnapshot {
        TimeLimiterSnapshot {
            number_of_successful_calls: self.number_of_successful_calls.load(Ordering::Relaxed),
            number_of_failed_calls: self.number_of_failed_calls.load(Ordering::Relaxed),
            number_of_timeouts: self.number_of_timeouts.load(Ordering::Relaxed),
        }
    }

    /// Calls `guarded`, cancelling it when it outlives the time limit.
    ///
    /// The future is created by `guarded` and handed to
    /// [`tokio::time::timeout`], so this method needs a Tokio runtime with the
    /// time driver running.
    ///
    /// # Cancellation
    ///
    /// When the limit is reached the future is **dropped** and never polled
    /// again, which is how an async operation is cancelled in Rust; that is the
    /// behaviour `cancel_running_future = true` describes. Work the future
    /// handed to another task, thread pool or operating system call is not
    /// stopped by the drop — it merely stops being observed. Use
    /// [`Self::call_non_cancelling`] when the dropped call must nevertheless run
    /// to completion.
    ///
    /// # Errors
    ///
    /// Returns [`TimeLimiterError::Timeout`] when the call outlived the
    /// configured limit, and [`TimeLimiterError::Inner`] when the guarded call
    /// failed on its own. Every outcome is counted and published on
    /// [`Self::event_stream`].
    pub async fn call<T, E, F, Fut>(&self, guarded: F) -> Result<T, TimeLimiterError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: fmt::Display,
    {
        let timeout_duration = self.config.timeout_duration();
        let started = Instant::now();

        match timeout(timeout_duration, guarded()).await {
            Err(_elapsed) => {
                debug!(
                    time_limiter = %self.name,
                    timeout = ?timeout_duration,
                    "guarded call timed out and was cancelled"
                );
                self.number_of_timeouts.fetch_add(1, Ordering::Relaxed);
                self.events.publish(TimeLimiterEvent::Timeout {
                    timeout: timeout_duration,
                });
                Err(TimeLimiterError::Timeout(TimeoutError::new(
                    timeout_duration,
                )))
            }
            Ok(Ok(value)) => {
                let duration = started.elapsed();
                self.number_of_successful_calls
                    .fetch_add(1, Ordering::Relaxed);
                self.events.publish(TimeLimiterEvent::Success { duration });
                Ok(value)
            }
            Ok(Err(error)) => {
                let duration = started.elapsed();
                let message = error.to_string();
                self.number_of_failed_calls.fetch_add(1, Ordering::Relaxed);
                self.events
                    .publish(TimeLimiterEvent::Error { duration, message });
                Err(TimeLimiterError::Inner(error))
            }
        }
    }

    /// Calls `guarded` on its own task and stops waiting for it when it outlives
    /// the time limit.
    ///
    /// The difference from [`Self::call`] is what happens at the deadline. The
    /// guarded call runs inside a spawned task, so the timeout can only stop the
    /// *waiting*: the [`JoinHandle`](tokio::task::JoinHandle) is dropped, which
    /// detaches the task rather than aborting it. The call therefore keeps
    /// running to completion, keeps consuming whatever resources it holds — a
    /// connection, a thread, memory — and its result is discarded, because
    /// nobody is left to receive it. Only the counters and the
    /// [`TimeLimiterEvent::Timeout`] published here record that the caller gave
    /// up.
    ///
    /// Use this when abandoning the call would corrupt shared state, or when the
    /// operation must observe its own completion; otherwise prefer
    /// [`Self::call`], which lets the runtime reclaim the work. The
    /// `cancel_running_future` setting describes what [`Self::call`] does; this
    /// method exists precisely to do the opposite, so it ignores that setting.
    ///
    /// # Errors
    ///
    /// Returns [`TimeLimiterError::Timeout`] when the call outlived the
    /// configured limit, [`TimeLimiterError::Inner`] when the guarded call
    /// failed on its own, and [`TimeLimiterError::Aborted`] when the spawned
    /// task ended without producing either — because it panicked, or because the
    /// runtime cancelled it. An abandoned call that finishes later publishes
    /// nothing and changes no counter: it is no longer the caller's call.
    pub async fn call_non_cancelling<T, E, F, Fut>(
        &self,
        guarded: F,
    ) -> Result<T, TimeLimiterError<E>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static + fmt::Display,
    {
        let timeout_duration = self.config.timeout_duration();
        let started = Instant::now();
        let handle = tokio::spawn(guarded());

        match timeout(timeout_duration, handle).await {
            Err(_elapsed) => {
                debug!(
                    time_limiter = %self.name,
                    timeout = ?timeout_duration,
                    "guarded call timed out; its task keeps running detached"
                );
                self.number_of_timeouts.fetch_add(1, Ordering::Relaxed);
                self.events.publish(TimeLimiterEvent::Timeout {
                    timeout: timeout_duration,
                });
                Err(TimeLimiterError::Timeout(TimeoutError::new(
                    timeout_duration,
                )))
            }
            Ok(Ok(Ok(value))) => {
                let duration = started.elapsed();
                self.number_of_successful_calls
                    .fetch_add(1, Ordering::Relaxed);
                self.events.publish(TimeLimiterEvent::Success { duration });
                Ok(value)
            }
            Ok(Ok(Err(error))) => {
                let duration = started.elapsed();
                let message = error.to_string();
                self.number_of_failed_calls.fetch_add(1, Ordering::Relaxed);
                self.events
                    .publish(TimeLimiterEvent::Error { duration, message });
                Err(TimeLimiterError::Inner(error))
            }
            Ok(Err(join_error)) => {
                // The task produced neither a value nor an error, so there is no
                // `E` to hand back; the runtime's report is all that is left.
                let duration = started.elapsed();
                let message = join_error.to_string();
                warn!(
                    time_limiter = %self.name,
                    %message,
                    "guarded call ended without a result"
                );
                self.number_of_failed_calls.fetch_add(1, Ordering::Relaxed);
                self.events.publish(TimeLimiterEvent::Error {
                    duration,
                    message: message.clone(),
                });
                Err(TimeLimiterError::Aborted(AbortedError::new(message)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use super::TimeLimiter;
    use crate::timelimiter::{
        time_limiter_config::TimeLimiterConfig, time_limiter_error::TimeLimiterError,
        time_limiter_event::TimeLimiterEvent, time_limiter_snapshot::TimeLimiterSnapshot,
    };

    /// The limit every test in this module configures.
    const TIMEOUT: Duration = Duration::from_millis(100);

    /// Builds a limiter that gives a guarded call 100 ms.
    fn limiter() -> TimeLimiter {
        TimeLimiter::new(
            "test",
            TimeLimiterConfig::builder()
                .timeout_duration(TIMEOUT)
                .build(),
        )
    }

    /// The failure a guarded call reports in these tests.
    fn failure() -> io::Error {
        io::Error::other("boom")
    }

    #[test]
    fn a_limiter_reports_its_name_and_settings() {
        let limiter = TimeLimiter::of("payments");
        let defaults = TimeLimiterConfig::default();

        assert_eq!(limiter.name(), "payments");
        assert_eq!(limiter.config().timeout_duration(), defaults.timeout_duration());
        assert!(limiter.cancel_running_future());
        assert_eq!(
            limiter.snapshot(),
            TimeLimiterSnapshot {
                number_of_successful_calls: 0,
                number_of_failed_calls: 0,
                number_of_timeouts: 0,
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_fast_call_returns_its_value_and_is_counted() {
        let limiter = limiter();

        let value = limiter
            .call(|| async { Ok::<_, io::Error>(41) })
            .await
            .expect("call should succeed");

        assert_eq!(value, 41);

        let snapshot = limiter.snapshot();
        assert_eq!(snapshot.number_of_successful_calls, 1);
        assert_eq!(snapshot.number_of_failed_calls, 0);
        assert_eq!(snapshot.number_of_timeouts, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn a_call_slower_than_the_timeout_reports_the_configured_limit() {
        let limiter = limiter();

        let error = limiter
            .call(|| async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok::<_, io::Error>(1)
            })
            .await
            .expect_err("call should time out");

        match error {
            TimeLimiterError::Timeout(timeout) => assert_eq!(timeout.timeout, TIMEOUT),
            other => panic!("unexpected error: {other:?}"),
        }

        let snapshot = limiter.snapshot();
        assert_eq!(snapshot.number_of_timeouts, 1);
        assert_eq!(snapshot.number_of_successful_calls, 0);
        assert_eq!(snapshot.number_of_failed_calls, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn the_failure_of_the_guarded_call_is_wrapped_in_inner() {
        let limiter = limiter();

        let error = limiter
            .call(|| async { Err::<(), _>(failure()) })
            .await
            .expect_err("call should fail");

        assert!(!error.is_timeout());
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
        assert_eq!(limiter.snapshot().number_of_failed_calls, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn the_success_event_reports_how_long_the_call_took() {
        let limiter = limiter();
        let mut events = limiter.event_stream();

        let value = limiter
            .call(|| async {
                tokio::time::sleep(Duration::from_millis(40)).await;
                Ok::<_, io::Error>(5)
            })
            .await
            .expect("call should succeed");

        assert_eq!(value, 5);
        match events.recv().await {
            Ok(TimeLimiterEvent::Success { duration }) => {
                assert_eq!(duration, Duration::from_millis(40));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn events_describe_the_outcome_of_every_call() {
        let limiter = limiter();
        let mut events = limiter.event_stream();

        let _ = limiter.call(|| async { Ok::<_, io::Error>(1) }).await;
        let _ = limiter.call(|| async { Err::<(), _>(failure()) }).await;
        let _ = limiter
            .call(|| async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok::<_, io::Error>(1)
            })
            .await;

        assert!(matches!(
            events.recv().await,
            Ok(TimeLimiterEvent::Success { .. })
        ));
        match events.recv().await {
            Ok(TimeLimiterEvent::Error { message, .. }) => assert_eq!(message, "boom"),
            other => panic!("unexpected event: {other:?}"),
        }
        assert!(matches!(
            events.recv().await,
            Ok(TimeLimiterEvent::Timeout { timeout }) if timeout == TIMEOUT
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn the_snapshot_totals_every_outcome() {
        let limiter = limiter();

        for _ in 0..2 {
            let _ = limiter.call(|| async { Ok::<_, io::Error>(1) }).await;
        }
        for _ in 0..3 {
            let _ = limiter.call(|| async { Err::<(), _>(failure()) }).await;
        }
        for _ in 0..4 {
            let _ = limiter
                .call(|| async {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    Ok::<_, io::Error>(1)
                })
                .await;
        }

        let snapshot = limiter.snapshot();
        assert_eq!(snapshot.number_of_successful_calls, 2);
        assert_eq!(snapshot.number_of_failed_calls, 3);
        assert_eq!(snapshot.number_of_timeouts, 4);
    }

    #[tokio::test(start_paused = true)]
    async fn a_non_cancelling_call_reports_the_timeout_and_leaves_its_task_running() {
        let limiter = limiter();
        let finished = Arc::new(AtomicBool::new(false));
        let completed = Arc::clone(&finished);

        let error = limiter
            .call_non_cancelling(move || async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                completed.store(true, Ordering::SeqCst);
                Ok::<u32, io::Error>(7)
            })
            .await
            .expect_err("call should time out");

        assert!(error.is_timeout());
        assert_eq!(limiter.snapshot().number_of_timeouts, 1);

        // The detached task is still asleep when the caller gave up. Letting the
        // paused clock run past its deadline proves that it finishes on its own,
        // without panicking on the state the caller left behind.
        assert!(!finished.load(Ordering::SeqCst));
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(finished.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn a_non_cancelling_call_that_finishes_in_time_behaves_like_call() {
        let limiter = limiter();
        let mut events = limiter.event_stream();

        let value = limiter
            .call_non_cancelling(|| async { Ok::<_, io::Error>(11) })
            .await
            .expect("call should succeed");

        assert_eq!(value, 11);
        assert_eq!(limiter.snapshot().number_of_successful_calls, 1);
        assert!(matches!(
            events.recv().await,
            Ok(TimeLimiterEvent::Success { .. })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn a_task_that_ends_without_a_result_is_reported_as_aborted() {
        let limiter = limiter();

        let result: Result<u32, TimeLimiterError<io::Error>> = limiter
            .call_non_cancelling(|| async {
                panic!("guarded call panicked");
            })
            .await;

        let error = result.expect_err("call should be aborted");
        match error {
            TimeLimiterError::Aborted(error) => {
                assert!(
                    error.message.contains("panicked"),
                    "unexpected message: {}",
                    error.message
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(limiter.snapshot().number_of_failed_calls, 1);
    }
}
