//! The bulkhead itself.
//!
//! A bulkhead caps how many calls run at the same time. It is the semaphore
//! flavour of the pattern: every call takes one of a fixed number of permits
//! before it starts and returns it when it finishes, so a slow dependency can
//! never consume more than its share of the caller's capacity. A call that
//! finds every permit taken is either rejected immediately or, when a
//! [`max_wait_duration`] is configured, waits for a permit to come back.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::bulkhead::{Bulkhead, BulkheadConfig};
//!
//! # async fn example() {
//! // At most four renders may run at once; a fifth caller waits up to 50 ms.
//! let bulkhead = Bulkhead::new(
//!     "reports",
//!     BulkheadConfig::builder()
//!         .max_concurrent_calls(4)
//!         .max_wait_duration(Duration::from_millis(50))
//!         .build(),
//! );
//!
//! let result = bulkhead
//!     .call(|| async { Ok::<_, std::io::Error>("rendered") })
//!     .await;
//!
//! assert!(result.is_ok());
//! # }
//! ```
//!
//! [`max_wait_duration`]: crate::bulkhead::BulkheadConfigBuilder::max_wait_duration

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast};
use tracing::debug;

use super::{
    bulkhead_config::BulkheadConfig,
    bulkhead_error::BulkheadError,
    bulkhead_event::BulkheadEvent,
    bulkhead_snapshot::BulkheadSnapshot,
};
use crate::{error::BulkheadFullError, event::EventPublisher};

/// State a permit needs after the call it belongs to has ended.
///
/// A permit is dropped by whoever holds it, which is not necessarily the
/// bulkhead, so everything the drop has to report lives behind an [`Arc`] that
/// both of them share.
#[derive(Debug)]
struct BulkheadInner {
    /// Name of the guarded resource, used in logs and events.
    name: String,
    /// Fan-out of the events this bulkhead emits.
    events: EventPublisher<BulkheadEvent>,
    /// Total number of calls rejected since the bulkhead was created.
    number_of_rejected_calls: AtomicU64,
}

/// A bulkhead guarding one resource.
///
/// Wrap it in an [`Arc`] to share it: the bulkhead owns the permits that bound
/// concurrency, so cloning it would split that limit.
///
/// [`Bulkhead::call`] is the convenient entry point; when the guarded operation
/// cannot be expressed as one closure, drive the bulkhead by hand with
/// [`Bulkhead::acquire_permission`], which returns a [`BulkheadPermit`] that
/// releases its slot when it is dropped.
#[derive(Debug)]
pub struct Bulkhead {
    /// Settings of this bulkhead.
    config: BulkheadConfig,
    /// The slots themselves.
    semaphore: Arc<Semaphore>,
    /// State shared with the permits this bulkhead hands out.
    inner: Arc<BulkheadInner>,
}

impl Bulkhead {
    /// Creates a bulkhead for the resource called `name`.
    ///
    /// # Panics
    ///
    /// Panics when the configuration reserves no slot; see
    /// [`BulkheadConfigBuilder::build`](crate::bulkhead::BulkheadConfigBuilder::build).
    pub fn new(name: impl Into<String>, config: BulkheadConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_calls() as usize)),
            config,
            inner: Arc::new(BulkheadInner {
                name: name.into(),
                events: EventPublisher::new(),
                number_of_rejected_calls: AtomicU64::new(0),
            }),
        }
    }

    /// Creates a bulkhead for `name` using the default settings.
    pub fn of(name: impl Into<String>) -> Self {
        Self::new(name, BulkheadConfig::default())
    }

    /// Returns the name of the guarded resource.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Returns the settings of this bulkhead.
    pub fn config(&self) -> &BulkheadConfig {
        &self.config
    }

    /// Subscribes to the events this bulkhead emits from now on.
    pub fn event_stream(&self) -> broadcast::Receiver<BulkheadEvent> {
        self.inner.events.subscribe()
    }

    /// Takes one of the bulkhead's slots for the caller.
    ///
    /// The returned [`BulkheadPermit`] holds the slot until it is dropped,
    /// which is what bounds the number of concurrent calls.
    ///
    /// # Errors
    ///
    /// Returns [`BulkheadFullError`] when every slot is taken and
    /// [`BulkheadConfig::max_wait_duration`] is zero, or when the wait for a
    /// free slot elapsed without one coming back. A rejection is counted and
    /// published as [`BulkheadEvent::CallRejected`]; an admitted call is
    /// published as [`BulkheadEvent::CallPermitted`].
    pub async fn acquire_permission(&self) -> Result<BulkheadPermit, BulkheadFullError> {
        let max_wait_duration = self.config.max_wait_duration();

        let permit = if max_wait_duration.is_zero() {
            // Without a wait duration a caller that finds the bulkhead full is
            // rejected outright, so there is nothing to await.
            match self.semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => return Err(self.reject()),
            }
        } else {
            match tokio::time::timeout(
                max_wait_duration,
                self.semaphore.clone().acquire_owned(),
            )
            .await
            {
                Ok(Ok(permit)) => permit,
                // Either the wait elapsed, or the semaphore was closed while
                // the caller waited. Both mean no slot was obtained.
                Ok(Err(_)) | Err(_) => return Err(self.reject()),
            }
        };

        let permit = BulkheadPermit::new(Arc::clone(&self.inner), permit);
        let available_concurrent_calls = self.available_concurrent_calls();

        debug!(
            bulkhead = %self.inner.name,
            available_concurrent_calls,
            "call permitted by bulkhead"
        );
        self.inner
            .events
            .publish(BulkheadEvent::CallPermitted {
                available_concurrent_calls,
            });

        Ok(permit)
    }

    /// Calls `guarded` under the protection of this bulkhead.
    ///
    /// The closure runs only when a slot is free, and its slot is held for as
    /// long as the future it returns takes, on both the success and the error
    /// path.
    ///
    /// # Errors
    ///
    /// Returns [`BulkheadError::BulkheadFull`] without running the closure when
    /// no slot was free in time, and [`BulkheadError::Inner`] when the closure
    /// fails.
    pub async fn call<T, E, F, Fut>(&self, guarded: F) -> Result<T, BulkheadError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        // The permit is bound to a named local on purpose: it has to live until
        // the end of this body so that the guarded call holds its slot while it
        // runs, and so that the slot is released on either outcome.
        let _permit = self.acquire_permission().await?;

        match guarded().await {
            Ok(value) => Ok(value),
            Err(error) => Err(BulkheadError::Inner(error)),
        }
    }

    /// Returns how many calls could start right now without waiting.
    pub fn available_concurrent_calls(&self) -> u32 {
        u32::try_from(self.semaphore.available_permits()).unwrap_or(u32::MAX)
    }

    /// Returns the counters of this bulkhead.
    pub fn snapshot(&self) -> BulkheadSnapshot {
        BulkheadSnapshot {
            available_concurrent_calls: self.available_concurrent_calls(),
            max_allowed_concurrent_calls: self.config.max_concurrent_calls(),
            number_of_rejected_calls: self
                .inner
                .number_of_rejected_calls
                .load(Ordering::Relaxed),
        }
    }

    /// Counts a rejection and announces it.
    fn reject(&self) -> BulkheadFullError {
        let available_concurrent_calls = self.available_concurrent_calls();
        self.inner
            .number_of_rejected_calls
            .fetch_add(1, Ordering::Relaxed);

        debug!(
            bulkhead = %self.inner.name,
            available_concurrent_calls,
            "call rejected by bulkhead"
        );
        self.inner
            .events
            .publish(BulkheadEvent::CallRejected {
                available_concurrent_calls,
            });

        BulkheadFullError
    }
}

/// One of a bulkhead's slots, held for as long as the call it was taken for.
///
/// Dropping the permit gives the slot back and publishes
/// [`BulkheadEvent::CallFinished`] with the time the permit was held. A permit
/// is deliberately neither [`Clone`] nor [`Copy`]: duplicating it would let one
/// call occupy two slots, or release a slot it never took.
#[derive(Debug)]
pub struct BulkheadPermit {
    /// State of the bulkhead that handed out this permit.
    inner: Arc<BulkheadInner>,
    /// The slot itself, released when this permit is dropped.
    permit: OwnedSemaphorePermit,
    /// When the slot was taken, used to time the guarded call.
    acquired_at: Instant,
}

impl BulkheadPermit {
    /// Wraps a freshly acquired slot in the permit the caller receives.
    fn new(inner: Arc<BulkheadInner>, permit: OwnedSemaphorePermit) -> Self {
        Self {
            inner,
            permit,
            acquired_at: Instant::now(),
        }
    }
}

impl Drop for BulkheadPermit {
    fn drop(&mut self) {
        let duration = self.acquired_at.elapsed();

        debug!(
            bulkhead = %self.inner.name,
            slots = self.permit.num_permits(),
            ?duration,
            "call finished in bulkhead"
        );
        self.inner
            .events
            .publish(BulkheadEvent::CallFinished { duration });
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

    use super::Bulkhead;
    use crate::{
        bulkhead::{bulkhead_config::BulkheadConfig, bulkhead_event::BulkheadEvent},
        error::BulkheadFullError,
    };

    /// A bulkhead that admits `max_concurrent_calls` calls and never waits.
    fn bulkhead_with(max_concurrent_calls: u32) -> Bulkhead {
        Bulkhead::new(
            "test",
            BulkheadConfig::builder()
                .max_concurrent_calls(max_concurrent_calls)
                .build(),
        )
    }

    fn failure() -> io::Error {
        io::Error::other("boom")
    }

    #[test]
    fn a_default_bulkhead_admits_twenty_five_calls_and_waits_for_none() {
        let bulkhead = Bulkhead::of("test");

        assert_eq!(bulkhead.name(), "test");
        assert_eq!(bulkhead.config().max_concurrent_calls(), 25);
        assert_eq!(bulkhead.config().max_wait_duration(), Duration::ZERO);
        assert_eq!(bulkhead.available_concurrent_calls(), 25);
    }

    #[tokio::test]
    async fn the_configured_number_of_slots_really_bounds_concurrency() {
        let bulkhead = bulkhead_with(3);

        let mut permits = Vec::new();
        for _ in 0..3 {
            permits.push(
                bulkhead
                    .acquire_permission()
                    .await
                    .expect("a call within the limit is admitted"),
            );
        }
        assert_eq!(bulkhead.available_concurrent_calls(), 0);

        let error = bulkhead
            .acquire_permission()
            .await
            .expect_err("the call beyond the limit must be rejected");
        assert_eq!(error, BulkheadFullError);
        assert_eq!(bulkhead.available_concurrent_calls(), 0);
    }

    #[tokio::test]
    async fn dropping_a_permit_frees_a_slot_again() {
        let bulkhead = bulkhead_with(1);

        let permit = bulkhead
            .acquire_permission()
            .await
            .expect("the only slot is free");
        assert!(bulkhead.acquire_permission().await.is_err());
        assert_eq!(bulkhead.available_concurrent_calls(), 0);

        drop(permit);
        assert_eq!(bulkhead.available_concurrent_calls(), 1);

        let permit = bulkhead
            .acquire_permission()
            .await
            .expect("the freed slot is reusable");
        drop(permit);
        assert_eq!(bulkhead.available_concurrent_calls(), 1);
    }

    /// Runs on paused time, so the holder's sleep and the waiter's wait are
    /// driven by the clock the runtime advances instead of real delays.
    #[tokio::test(start_paused = true)]
    async fn a_caller_that_waits_is_admitted_once_a_slot_is_released() {
        let bulkhead = Arc::new(Bulkhead::new(
            "test",
            BulkheadConfig::builder()
                .max_concurrent_calls(1)
                .max_wait_duration(Duration::from_secs(1))
                .build(),
        ));
        let permit = bulkhead
            .acquire_permission()
            .await
            .expect("the only slot is free");

        let holder = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            drop(permit);
        });

        // The outer timeout is a guard against a hang rather than part of the
        // scenario: the waiter gives up after one second on its own.
        let acquired = tokio::time::timeout(
            Duration::from_secs(10),
            bulkhead.acquire_permission(),
        )
        .await
        .expect("the waiter must not hang")
        .expect("the released slot must go to the waiter");
        holder.await.expect("the holder task must finish");

        assert_eq!(bulkhead.available_concurrent_calls(), 0);
        drop(acquired);
        assert_eq!(bulkhead.available_concurrent_calls(), 1);
    }

    /// Runs on paused time, so the caller waits out its whole budget and is
    /// rejected without any real delay.
    #[tokio::test(start_paused = true)]
    async fn a_caller_that_waits_past_the_max_wait_duration_is_rejected() {
        let bulkhead = Bulkhead::new(
            "test",
            BulkheadConfig::builder()
                .max_concurrent_calls(1)
                .max_wait_duration(Duration::from_millis(100))
                .build(),
        );
        let mut events = bulkhead.event_stream();

        let _permit = bulkhead
            .acquire_permission()
            .await
            .expect("the only slot is free");
        let error = bulkhead
            .acquire_permission()
            .await
            .expect_err("the waiter must give up after the max wait duration");

        assert_eq!(error, BulkheadFullError);
        assert_eq!(bulkhead.snapshot().number_of_rejected_calls, 1);
        assert!(matches!(
            events.recv().await,
            Ok(BulkheadEvent::CallPermitted {
                available_concurrent_calls: 0
            })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(BulkheadEvent::CallRejected {
                available_concurrent_calls: 0
            })
        ));
    }

    #[tokio::test]
    async fn call_returns_the_value_of_a_successful_call() {
        let bulkhead = bulkhead_with(4);

        let value = bulkhead
            .call(|| async { Ok::<_, io::Error>(7) })
            .await
            .expect("the guarded call succeeds");

        assert_eq!(value, 7);
    }

    #[tokio::test]
    async fn call_releases_its_permit_on_the_success_path() {
        let bulkhead = bulkhead_with(4);

        let _ = bulkhead
            .call(|| async { Ok::<_, io::Error>(7) })
            .await
            .expect("the guarded call succeeds");

        assert_eq!(bulkhead.available_concurrent_calls(), 4);
    }

    #[tokio::test]
    async fn call_releases_its_permit_on_the_error_path() {
        let bulkhead = bulkhead_with(4);

        let error = bulkhead
            .call(|| async { Err::<(), _>(failure()) })
            .await
            .expect_err("the guarded call fails");

        assert!(!error.is_bulkhead_full());
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
        assert_eq!(bulkhead.available_concurrent_calls(), 4);
    }

    #[tokio::test]
    async fn the_permit_is_held_while_the_guarded_call_runs() {
        let bulkhead = bulkhead_with(1);

        let rejected = bulkhead
            .call(|| async {
                // The single slot is still taken by the call we are inside of.
                let rejected = bulkhead.acquire_permission().await.is_err();
                Ok::<_, io::Error>(rejected)
            })
            .await
            .expect("the guarded call succeeds");

        assert!(rejected);
        assert_eq!(bulkhead.available_concurrent_calls(), 1);
    }

    #[tokio::test]
    async fn call_does_not_run_the_closure_when_the_bulkhead_is_full() {
        let bulkhead = bulkhead_with(1);
        let _permit = bulkhead
            .acquire_permission()
            .await
            .expect("the only slot is free");

        let executed = AtomicBool::new(false);
        let error = bulkhead
            .call(|| async {
                executed.store(true, Ordering::SeqCst);
                Ok::<_, io::Error>(())
            })
            .await
            .expect_err("the call must be rejected");

        assert!(error.is_bulkhead_full());
        assert!(!executed.load(Ordering::SeqCst));
        assert_eq!(bulkhead.snapshot().number_of_rejected_calls, 1);
    }

    #[tokio::test]
    async fn the_snapshot_reports_capacity_and_a_monotonic_rejection_total() {
        let bulkhead = bulkhead_with(2);

        let empty = bulkhead.snapshot();
        assert_eq!(empty.max_allowed_concurrent_calls, 2);
        assert_eq!(empty.available_concurrent_calls, 2);
        assert_eq!(empty.number_of_rejected_calls, 0);
        assert!(!empty.is_full());

        let first = bulkhead
            .acquire_permission()
            .await
            .expect("first call admitted");
        let second = bulkhead
            .acquire_permission()
            .await
            .expect("second call admitted");

        let full = bulkhead.snapshot();
        assert_eq!(full.available_concurrent_calls, 0);
        assert_eq!(full.max_allowed_concurrent_calls, 2);
        assert!(full.is_full());

        assert!(bulkhead.acquire_permission().await.is_err());
        assert!(bulkhead.acquire_permission().await.is_err());

        let rejected = bulkhead.snapshot();
        assert_eq!(rejected.number_of_rejected_calls, 2);
        assert_eq!(rejected.available_concurrent_calls, 0);

        drop(first);
        drop(second);

        let released = bulkhead.snapshot();
        assert_eq!(released.available_concurrent_calls, 2);
        assert_eq!(released.number_of_rejected_calls, 2);
        assert!(!released.is_full());
    }

    #[tokio::test]
    async fn the_event_stream_reports_permitted_and_rejected_calls() {
        let bulkhead = bulkhead_with(1);
        let mut events = bulkhead.event_stream();

        let permit = bulkhead
            .acquire_permission()
            .await
            .expect("the only slot is free");
        assert!(bulkhead.acquire_permission().await.is_err());
        drop(permit);

        assert_eq!(
            events.recv().await.expect("the admitted call is announced"),
            BulkheadEvent::CallPermitted {
                available_concurrent_calls: 0
            }
        );
        assert_eq!(
            events.recv().await.expect("the rejected call is announced"),
            BulkheadEvent::CallRejected {
                available_concurrent_calls: 0
            }
        );
        assert!(matches!(
            events.recv().await,
            Ok(BulkheadEvent::CallFinished { .. })
        ));
    }

    /// Uses a short real sleep on purpose: the reported duration is wall clock
    /// time, and the assertion checks half of the slept span, so the test
    /// cannot flake while still being meaningful.
    #[tokio::test]
    async fn dropping_a_permit_reports_how_long_it_was_held() {
        let bulkhead = bulkhead_with(1);
        let mut events = bulkhead.event_stream();

        let permit = bulkhead
            .acquire_permission()
            .await
            .expect("the only slot is free");
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(permit);

        assert!(matches!(
            events.recv().await,
            Ok(BulkheadEvent::CallPermitted { .. })
        ));

        match events.recv().await {
            Ok(BulkheadEvent::CallFinished { duration }) => {
                assert!(
                    duration >= Duration::from_millis(10),
                    "reported duration {duration:?} is too short"
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
