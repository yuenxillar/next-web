//! The rate limiter itself.
//!
//! A rate limiter caps how often a guarded resource is called. It hands out
//! `limit_for_period` permissions per `limit_refresh_period`, starting at every
//! multiple of that period on the clock, and refuses a call once the budget of
//! the current period is spent. Permissions that were not used are lost at the
//! end of the period; they are never carried over.
//!
//! A caller that is willing to wait can be served by the next period instead of
//! being refused. [`RateLimiter::acquire_permission_with_timeout`] sleeps until
//! the next refresh and retries, and [`RateLimiter::call`] does the same with
//! the configured `timeout_duration`.
//!
//! The limiter holds no budget of its own: it asks a [`RateLimiterStore`] for
//! each permission, so the budget can live in this process (the default) or in a
//! service every instance shares. See the
//! [`rate_limiter_store`](super::rate_limiter_store) module for that seam.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::ratelimiter::{RateLimiter, RateLimiterConfig};
//!
//! # async fn example() {
//! // At most 10 calls per second, waiting up to 2 seconds for a permission.
//! let limiter = RateLimiter::new(
//!     "payments",
//!     RateLimiterConfig::builder()
//!         .limit_for_period(10)
//!         .limit_refresh_period(Duration::from_secs(1))
//!         .timeout_duration(Duration::from_secs(2))
//!         .build(),
//! );
//!
//! match limiter.call(|| async { Ok::<_, std::io::Error>("charged") }).await {
//!     Ok(value) => println!("{value}"),
//!     Err(error) => eprintln!("not charged: {error}"),
//! }
//! # }
//! ```

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use tokio::sync::broadcast;
use tracing::warn;

use super::{
    rate_limiter_config::RateLimiterConfig, rate_limiter_error::RateLimiterError,
    rate_limiter_event::RateLimiterEvent, rate_limiter_snapshot::RateLimiterSnapshot,
    rate_limiter_store::{InMemoryRateLimiterStore, PermissionRequest, RateLimiterStore},
};
use crate::{error::RateLimitExceededError, event::EventPublisher};

/// A rate limiter guarding one resource.
///
/// Wrap it in an [`Arc`](std::sync::Arc) to share it: every caller has to go
/// through the same instance to share one budget, so cloning it would split that
/// state.
///
/// The limiter owns no budget itself. It turns each call into a
/// [`PermissionRequest`] and lets its [`RateLimiterStore`] decide, so the budget
/// can live in this process — the default, see [`InMemoryRateLimiterStore`] — or
/// somewhere every instance can see, which is what [`RateLimiter::with_store`]
/// is for.
///
/// The counters in [`RateLimiter::snapshot`] are the limiter's own: they count
/// what *this* instance took and refused, whatever the store does with them.
///
/// [`RateLimiter::call`] is the convenient entry point;
/// [`RateLimiter::acquire_permission`] and
/// [`RateLimiter::acquire_permission_with_timeout`] are the primitives for a
/// guarded operation that cannot be expressed as one closure.
#[derive(Debug)]
pub struct RateLimiter {
    /// Name of the guarded resource, used in logs, metrics and the store key.
    name: String,
    /// Settings of this limiter.
    config: RateLimiterConfig,
    /// Where the budget lives.
    store: Arc<dyn RateLimiterStore>,
    /// Permissions this instance was granted.
    acquired_permissions: AtomicU64,
    /// Permissions this instance was refused.
    rejected_permissions: AtomicU64,
    /// Fan-out of the events this limiter emits.
    events: EventPublisher<RateLimiterEvent>,
}

impl RateLimiter {
    /// Creates a limiter whose budget lives in this process.
    ///
    /// This is the right choice for a single instance. Several instances behind
    /// a load balancer each get their own budget this way, so a quota that has
    /// to cover them all needs [`RateLimiter::with_store`].
    pub fn new(name: impl Into<String>, config: RateLimiterConfig) -> Self {
        Self::with_store(name, config, Arc::new(InMemoryRateLimiterStore::new()))
    }

    /// Creates a limiter whose permissions come from `store`.
    ///
    /// Pass a store that talks to a shared service — Redis, a database, another
    /// process — to make one quota cover every instance of the application. The
    /// store sees this limiter's [`PermissionRequest::name`], so limiters given
    /// the same name and the same store share one budget.
    ///
    /// See the [`rate_limiter_store`](super::rate_limiter_store) module for what
    /// an implementation has to guarantee.
    pub fn with_store(
        name: impl Into<String>,
        config: RateLimiterConfig,
        store: Arc<dyn RateLimiterStore>,
    ) -> Self {
        Self {
            name: name.into(),
            config,
            store,
            acquired_permissions: AtomicU64::new(0),
            rejected_permissions: AtomicU64::new(0),
            events: EventPublisher::new(),
        }
    }

    /// Creates a limiter for `name` using the default settings.
    pub fn of(name: impl Into<String>) -> Self {
        Self::new(name, RateLimiterConfig::default())
    }

    /// Returns the store that decides this limiter's permissions.
    pub fn store(&self) -> &Arc<dyn RateLimiterStore> {
        &self.store
    }

    /// Builds the request the store decides on, from a single clock reading.
    fn permission_request(&self) -> PermissionRequest<'_> {
        PermissionRequest {
            name: &self.name,
            limit_for_period: self.config.limit_for_period(),
            limit_refresh_period: self.config.limit_refresh_period(),
            now: self.config.clock().monotonic_time(),
        }
    }

    /// Returns the name of the guarded resource.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the settings of this limiter.
    pub fn config(&self) -> &RateLimiterConfig {
        &self.config
    }

    /// Subscribes to the events this limiter emits from now on.
    pub fn event_stream(&self) -> broadcast::Receiver<RateLimiterEvent> {
        self.events.subscribe()
    }

    /// Takes a permission for one call of the guarded resource.
    ///
    /// This never waits: when the budget of the current period is spent, the
    /// call is refused at once. Use [`Self::acquire_permission_with_timeout`] to
    /// let the caller be served by the next period instead.
    ///
    /// It awaits because the store behind the limiter may live in another
    /// process.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimitExceededError`] when no permission is left in the
    /// current period. Every rejection is counted and published as
    /// [`RateLimiterEvent::PermissionRejected`]; every grant is counted and
    /// published as [`RateLimiterEvent::PermissionAcquired`] with a wait of
    /// [`Duration::ZERO`].
    pub async fn acquire_permission(&self) -> Result<(), RateLimitExceededError> {
        if self
            .store
            .try_acquire(self.permission_request())
            .await
            .is_ok()
        {
            self.acquired_permissions.fetch_add(1, Ordering::SeqCst);
            self.events
                .publish(RateLimiterEvent::PermissionAcquired {
                    waited: Duration::ZERO,
                });
            Ok(())
        } else {
            self.rejected_permissions.fetch_add(1, Ordering::SeqCst);
            warn!(
                rate_limiter = %self.name,
                "call not permitted by rate limiter"
            );
            self.events.publish(RateLimiterEvent::PermissionRejected);
            Err(RateLimitExceededError)
        }
    }

    /// Takes a permission, waiting up to `timeout` for the next period to start
    /// one.
    ///
    /// When the current period is spent, this method sleeps until the next
    /// refresh boundary and tries again, spending the timeout as it goes. It
    /// awaits, so it must be called from an async context. A caller that cannot
    /// be served inside `timeout` is refused at once, without sleeping at all.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimitExceededError`] when the next period does not start
    /// within `timeout`. Every rejection is counted and published as
    /// [`RateLimiterEvent::PermissionRejected`]; every grant is counted and
    /// published as [`RateLimiterEvent::PermissionAcquired`], whose `waited`
    /// field is the time this call spent between its start and the grant.
    pub async fn acquire_permission_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Duration, RateLimitExceededError> {
        // The clock the limiter already reads measures the waiting, so a test
        // that drives a `ManualClock` observes the same timeline as the periods
        // the store decides on.
        let started = self.config.clock().monotonic_time();
        let mut remaining = timeout;

        loop {
            match self.store.try_acquire(self.permission_request()).await {
                Ok(()) => {
                    let waited = self
                        .config
                        .clock()
                        .monotonic_time()
                        .saturating_sub(started);
                    self.acquired_permissions.fetch_add(1, Ordering::SeqCst);
                    self.events
                        .publish(RateLimiterEvent::PermissionAcquired { waited });
                    return Ok(waited);
                }
                Err(refusal) => {
                    let wait = refusal.wait_until_next_period;
                    // The period that would serve this call starts later than the
                    // budget allows. Sleeping would only postpone the refusal, so
                    // this returns at once and never waits for a period the
                    // caller cannot be served by.
                    if wait > remaining {
                        return Err(self.reject(RateLimitExceededError, timeout, remaining));
                    }

                    tokio::time::sleep(wait).await;
                    remaining = remaining.saturating_sub(wait);
                }
            }
        }
    }

    /// Calls `guarded` under the protection of this limiter.
    ///
    /// A permission is acquired first, waiting up to the configured
    /// `timeout_duration` for it; the closure then runs and its value is
    /// returned.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimiterError::RateLimitExceeded`] without running the
    /// closure when no permission could be acquired in time, and
    /// [`RateLimiterError::Inner`] when the closure fails.
    pub async fn call<T, E, F, Fut>(&self, guarded: F) -> Result<T, RateLimiterError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        self.acquire_permission_with_timeout(self.config.timeout_duration())
            .await?;

        guarded().await.map_err(RateLimiterError::Inner)
    }

    /// Returns the counters of this limiter.
    ///
    /// The acquired and rejected totals describe this instance only. The
    /// available permissions come from the store: a local store answers exactly,
    /// while a store in another process reports `None`, because answering would
    /// cost a round trip and this method is synchronous.
    pub fn snapshot(&self) -> RateLimiterSnapshot {
        RateLimiterSnapshot {
            available_permissions: self
                .store
                .available_permissions(&self.permission_request()),
            limit_for_period: self.config.limit_for_period(),
            limit_refresh_period: self.config.limit_refresh_period(),
            number_of_acquired_permissions: self.acquired_permissions.load(Ordering::SeqCst),
            number_of_rejected_permissions: self.rejected_permissions.load(Ordering::SeqCst),
        }
    }

    /// Counts and publishes a refusal, and hands the error back.
    fn reject(
        &self,
        error: RateLimitExceededError,
        timeout: Duration,
        remaining: Duration,
    ) -> RateLimitExceededError {
        self.rejected_permissions.fetch_add(1, Ordering::SeqCst);
        warn!(
            rate_limiter = %self.name,
            timeout = ?timeout,
            remaining = ?remaining,
            "call not permitted by rate limiter"
        );
        self.events.publish(RateLimiterEvent::PermissionRejected);
        error
    }

}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use super::RateLimiter;
    use crate::{
        clock::{Clock, ManualClock},
        error::RateLimitExceededError,
        ratelimiter::{
            rate_limiter_config::RateLimiterConfig, rate_limiter_error::RateLimiterError,
            rate_limiter_event::RateLimiterEvent,
        },
    };

    /// Number of clock reads after which [`AdvancingClock`] moves the underlying
    /// manual clock on by one of its periods.
    const READS_BEFORE_EACH_ADVANCE: usize = 8;

    /// A clock that stands still for the first few reads and then jumps forward
    /// once per [`READS_BEFORE_EACH_ADVANCE`] reads.
    ///
    /// The rate limiter sleeps for the whole time until the next refresh, so on
    /// a clock that never moves the sleep would not change the cycle and the
    /// timeout would be spent without ever acquiring. This clock models the one
    /// thing a test cannot wait for — time passing while the caller sleeps —
    /// without introducing a real delay, which keeps the test deterministic.
    #[derive(Debug)]
    struct AdvancingClock {
        clock: ManualClock,
        period: Duration,
        reads: AtomicUsize,
        advances: AtomicUsize,
    }

    impl AdvancingClock {
        fn new(period: Duration) -> Self {
            Self {
                clock: ManualClock::new(),
                period,
                reads: AtomicUsize::new(0),
                advances: AtomicUsize::new(0),
            }
        }
    }

    impl Clock for AdvancingClock {
        fn monotonic_time(&self) -> Duration {
            let read = self.reads.fetch_add(1, Ordering::SeqCst) + 1;

            if read.is_multiple_of(READS_BEFORE_EACH_ADVANCE) {
                self.clock.advance(self.period);
                self.advances.fetch_add(1, Ordering::SeqCst);
            }

            self.clock.monotonic_time()
        }
    }

    /// Builds a limiter granting `limit` permissions per `period`, driven by
    /// `clock`.
    fn limiter_with(clock: &ManualClock, limit: u32, period: Duration) -> RateLimiter {
        // The budget of a waiting call is deliberately smaller than the period,
        // so a call that cannot be served straight away is refused at once
        // instead of sleeping for the whole period.
        limiter_with_timeout(clock, limit, period, Duration::from_millis(250))
    }

    /// Builds a limiter granting `limit` permissions per `period`, driven by
    /// `clock`, with an explicit budget for a waiting call.
    fn limiter_with_timeout(
        clock: &ManualClock,
        limit: u32,
        period: Duration,
        timeout: Duration,
    ) -> RateLimiter {
        RateLimiter::new(
            "test",
            RateLimiterConfig::builder()
                .limit_for_period(limit)
                .limit_refresh_period(period)
                .timeout_duration(timeout)
                .clock(Arc::new(clock.clone()))
                .build(),
        )
    }

    #[tokio::test]
    async fn a_fresh_limiter_grants_its_whole_budget_before_refusing() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 3, Duration::from_secs(1));

        assert_eq!(limiter.name(), "test");
        assert_eq!(limiter.snapshot().available_permissions, Some(3));

        for call in 1..=3 {
            assert!(
                limiter.acquire_permission().await.is_ok(),
                "call {call} of a brand new limiter should be permitted"
            );
        }

        assert!(limiter.acquire_permission().await.is_err());
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 3);
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 1);
    }

    #[tokio::test]
    async fn a_fresh_limiter_on_a_clock_that_starts_late_grants_its_whole_budget() {
        let clock = ManualClock::new();
        // A clock whose epoch is not the limiter's: the first call lands in the
        // cycle numbered 2, in the middle of a period.
        clock.set(Duration::from_millis(1500));
        let limiter = limiter_with(&clock, 3, Duration::from_secs(1));

        for call in 1..=3 {
            assert!(
                limiter.acquire_permission().await.is_ok(),
                "call {call} of a brand new limiter should be permitted"
            );
        }

        assert!(limiter.acquire_permission().await.is_err());
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 3);
    }

    #[tokio::test]
    async fn a_single_permission_budget_is_granted_once() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 1, Duration::from_secs(600));

        // This is the shape the integration layer uses: one permission per ten
        // minutes and a caller that cannot wait at all.
        assert!(
            limiter.acquire_permission().await.is_ok(),
            "the first call of a new limiter must be permitted"
        );
        assert!(limiter.acquire_permission().await.is_err());
    }

    #[tokio::test]
    async fn the_default_limiter_is_configured_like_resilience4j() {
        let limiter = RateLimiter::of("payments");

        assert_eq!(limiter.name(), "payments");
        assert_eq!(limiter.config().limit_for_period(), 50);
        assert_eq!(
            limiter.config().limit_refresh_period(),
            Duration::from_millis(500)
        );
        assert_eq!(limiter.config().timeout_duration(), Duration::from_secs(5));
        assert_eq!(limiter.snapshot().available_permissions, Some(50));
    }

    #[tokio::test]
    async fn one_period_grants_exactly_the_limit_and_then_refuses() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 3, Duration::from_secs(1));

        for _ in 0..3 {
            limiter.acquire_permission().await.expect("permission granted");
        }

        assert!(limiter.acquire_permission().await.is_err());
        assert_eq!(limiter.snapshot().is_permission_available(), Some(false));

        let snapshot = limiter.snapshot();
        assert_eq!(snapshot.number_of_acquired_permissions, 3);
        assert_eq!(snapshot.number_of_rejected_permissions, 1);
        assert_eq!(snapshot.limit_for_period, 3);
        assert_eq!(snapshot.limit_refresh_period, Duration::from_secs(1));
    }

    #[tokio::test]
    async fn time_within_one_period_does_not_refill_the_budget() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 2, Duration::from_secs(1));

        limiter.acquire_permission().await.expect("first permission");

        // The cycle is the elapsed time divided by the period, so any time short
        // of a full period stays inside cycle 0.
        clock.advance(Duration::from_millis(999));

        limiter.acquire_permission().await.expect("second permission");
        assert!(limiter.acquire_permission().await.is_err());
    }

    #[tokio::test]
    async fn advancing_a_period_refreshes_the_budget() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 2, Duration::from_secs(1));

        limiter.acquire_permission().await.expect("first permission");
        limiter.acquire_permission().await.expect("second permission");
        assert!(limiter.acquire_permission().await.is_err());
        assert_eq!(limiter.snapshot().available_permissions, Some(0));

        clock.advance(Duration::from_secs(1));

        assert_eq!(limiter.snapshot().available_permissions, Some(2));
        assert!(limiter.acquire_permission().await.is_ok());
        assert!(limiter.acquire_permission().await.is_ok());
        assert!(limiter.acquire_permission().await.is_err());
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 4);
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 2);
    }

    #[tokio::test]
    async fn unused_permissions_do_not_carry_over_into_the_next_cycle() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 2, Duration::from_secs(1));

        // Only one of the two permissions is spent before the period ends.
        limiter.acquire_permission().await.expect("first permission");

        clock.advance(Duration::from_secs(1));

        // The new period starts from the full limit again rather than from the
        // permission that was never used.
        assert_eq!(limiter.snapshot().available_permissions, Some(2));
        limiter.acquire_permission().await.expect("first of the new period");
        limiter.acquire_permission().await.expect("second of the new period");
        assert!(limiter.acquire_permission().await.is_err());
    }

    #[tokio::test]
    async fn several_periods_each_grant_their_own_budget() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 2, Duration::from_millis(100));

        for period in 0..5 {
            assert!(
                limiter.acquire_permission().await.is_ok(),
                "period {period} should grant its first permission"
            );
            assert!(
                limiter.acquire_permission().await.is_ok(),
                "period {period} should grant its second permission"
            );
            assert!(
                limiter.acquire_permission().await.is_err(),
                "period {period} should not grant a third permission"
            );

            clock.advance(Duration::from_millis(100));
        }

        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 10);
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 5);
    }

    #[tokio::test]
    async fn the_snapshot_never_reports_more_than_the_configured_limit() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 4, Duration::from_secs(1));

        for _ in 0..10 {
            clock.advance(Duration::from_millis(500));
            let snapshot = limiter.snapshot();
            assert!(snapshot.available_permissions.is_some_and(|available| available <= 4), "{snapshot:?}");
            assert_eq!(snapshot.limit_for_period, 4);
        }
    }

    #[tokio::test]
    async fn a_waiting_caller_is_served_by_the_next_period() {
        let clock = Arc::new(AdvancingClock::new(Duration::from_millis(50)));
        let limiter = RateLimiter::new(
            "test",
            RateLimiterConfig::builder()
                .limit_for_period(1)
                .limit_refresh_period(Duration::from_millis(50))
                .clock(clock.clone())
                .build(),
        );

        limiter.acquire_permission().await.expect("permission granted");
        assert!(limiter.acquire_permission().await.is_err());

        let waited = limiter
            .acquire_permission_with_timeout(Duration::from_secs(1))
            .await
            .expect("the next period should serve the caller");

        assert_eq!(waited, Duration::from_millis(50));
        assert_eq!(clock.advances.load(Ordering::SeqCst), 1);
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 2);
    }

    #[tokio::test]
    async fn acquiring_with_a_timeout_waits_for_the_next_cycle() {
        // This test uses a real, very short refresh period with the real clock
        // rather than a paused tokio clock. `tokio::time::sleep` is measured
        // against tokio's clock while the cycle comes from the configured clock,
        // so a `ManualClock` would never move and the loop would time out. Twenty
        // milliseconds per period with a two second budget leaves two orders of
        // magnitude of slack, which keeps the test deterministic in practice.
        let limiter = RateLimiter::new(
            "test",
            RateLimiterConfig::builder()
                .limit_for_period(1)
                .limit_refresh_period(Duration::from_millis(20))
                .build(),
        );

        limiter.acquire_permission().await.expect("permission granted");
        assert!(limiter.acquire_permission().await.is_err());

        let waited = limiter
            .acquire_permission_with_timeout(Duration::from_secs(2))
            .await
            .expect("the next cycle should serve the caller");

        assert!(waited > Duration::ZERO, "the caller had to wait: {waited:?}");
        assert!(
            waited < Duration::from_secs(2),
            "the wait was bounded: {waited:?}"
        );
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 2);
    }

    #[tokio::test]
    async fn a_zero_timeout_acquires_from_the_current_cycle_without_waiting() {
        let clock = ManualClock::new();
        let limiter = limiter_with_timeout(&clock, 2, Duration::from_secs(3600), Duration::ZERO);

        // A permission of the current cycle is free, so no waiting is involved.
        let waited = tokio::time::timeout(
            Duration::from_secs(5),
            limiter.acquire_permission_with_timeout(Duration::ZERO),
        )
        .await
        .expect("a reachable permission never sleeps")
        .expect("the fresh budget has a permission left");

        assert_eq!(waited, Duration::ZERO);

        // The second one too.
        assert!(
            limiter
                .acquire_permission_with_timeout(Duration::ZERO)
                .await
                .is_ok()
        );

        // The third has to wait for a cycle an hour away, which a zero budget
        // cannot do, so it must be refused rather than slept on.
        let refused = tokio::time::timeout(
            Duration::from_secs(5),
            limiter.acquire_permission_with_timeout(Duration::ZERO),
        )
        .await
        .expect("a zero timeout never sleeps")
        .expect_err("an hour does not fit in a zero budget");

        assert_eq!(refused.to_string(), RateLimitExceededError.to_string());
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 2);
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 1);
    }

    #[tokio::test]
    async fn a_timeout_without_a_reachable_cycle_is_refused() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 1, Duration::from_secs(3600));

        limiter.acquire_permission().await.expect("permission granted");

        // The next cycle is an hour away, so a zero budget cannot reach it.
        let error = limiter
            .acquire_permission_with_timeout(Duration::ZERO)
            .await
            .expect_err("the next cycle is out of reach");

        assert_eq!(error.to_string(), RateLimitExceededError.to_string());
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 1);
        assert_eq!(limiter.snapshot().available_permissions, Some(0));
    }

    #[tokio::test]
    async fn a_timeout_shorter_than_the_next_cycle_is_refused() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 1, Duration::from_secs(3600));

        limiter.acquire_permission().await.expect("permission granted");

        let error = limiter
            .acquire_permission_with_timeout(Duration::from_millis(250))
            .await
            .expect_err("an hour does not fit in 250 milliseconds");

        assert_eq!(error.to_string(), RateLimitExceededError.to_string());
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 1);
        // Nothing was acquired, so the budget of this period is still spent.
        assert_eq!(limiter.snapshot().is_permission_available(), Some(false));
    }

    #[tokio::test]
    async fn a_permitted_call_runs_the_closure() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 1, Duration::from_secs(3600));

        let executed = AtomicBool::new(false);
        let value = limiter
            .call(|| async {
                executed.store(true, Ordering::SeqCst);
                Ok::<_, io::Error>(7)
            })
            .await
            .expect("the first call is permitted");

        assert_eq!(value, 7);
        assert!(executed.load(Ordering::SeqCst));
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 1);
    }

    #[tokio::test]
    async fn a_refused_call_does_not_run_the_closure() {
        let clock = ManualClock::new();
        // One permission per ten minutes with a caller that cannot wait: this is
        // the shape the integration layer uses for a strict public endpoint.
        let limiter = limiter_with_timeout(&clock, 1, Duration::from_secs(600), Duration::ZERO);

        let executed = AtomicBool::new(false);
        limiter
            .call(|| async { Ok::<_, io::Error>(()) })
            .await
            .expect("the first call is permitted");

        // The budget is spent and the next cycle is ten minutes away, so the
        // zero budget cannot reach it and the closure must not run.
        let error = tokio::time::timeout(
            Duration::from_secs(5),
            limiter.call(|| async {
                executed.store(true, Ordering::SeqCst);
                Ok::<_, io::Error>(8)
            }),
        )
        .await
        .expect("a refusal is immediate, it never sleeps")
        .expect_err("the second call is refused");

        assert!(error.is_rate_limit_exceeded());
        assert!(matches!(error, RateLimiterError::RateLimitExceeded(_)));
        assert!(!executed.load(Ordering::SeqCst));
        assert_eq!(limiter.snapshot().number_of_acquired_permissions, 1);
        assert_eq!(limiter.snapshot().number_of_rejected_permissions, 1);
    }

    #[tokio::test]
    async fn call_wraps_the_failure_of_the_guarded_closure() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 3, Duration::from_secs(1));

        let error = limiter
            .call(|| async { Err::<(), _>(io::Error::other("boom")) })
            .await
            .expect_err("the closure should fail");

        assert!(!error.is_rate_limit_exceeded());
        assert_eq!(
            error.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );

        let composed: crate::error::ResilienceError<io::Error> = error.into();
        assert_eq!(
            composed.inner_error().map(ToString::to_string),
            Some("boom".to_owned())
        );
    }

    #[tokio::test]
    async fn events_report_every_acquire_and_reject() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 2, Duration::from_secs(3600));
        let mut events = limiter.event_stream();

        limiter.acquire_permission().await.expect("permission granted");
        limiter.acquire_permission().await.expect("permission granted");
        assert!(limiter.acquire_permission().await.is_err());

        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionAcquired {
                waited: Duration::ZERO
            })
        );
        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionAcquired {
                waited: Duration::ZERO
            })
        );
        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionRejected)
        );
    }

    #[tokio::test]
    async fn an_acquire_served_by_a_later_cycle_reports_no_wait() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 1, Duration::from_secs(1));
        let mut events = limiter.event_stream();

        limiter.acquire_permission().await.expect("permission granted");
        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionAcquired {
                waited: Duration::ZERO
            })
        );

        // The next cycle has already begun, so the caller is served straight
        // away and waited for none of it.
        clock.advance(Duration::from_secs(2));

        let waited = limiter
            .acquire_permission_with_timeout(Duration::from_secs(1))
            .await
            .expect("the next cycle serves the caller");

        assert_eq!(waited, Duration::ZERO);
        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionAcquired { waited })
        );
    }

    #[tokio::test]
    async fn rejected_async_acquires_are_announced_on_the_event_stream() {
        let clock = ManualClock::new();
        let limiter = limiter_with(&clock, 1, Duration::from_secs(3600));
        let mut events = limiter.event_stream();

        limiter.acquire_permission().await.expect("permission granted");
        assert!(
            limiter
                .acquire_permission_with_timeout(Duration::ZERO)
                .await
                .is_err()
        );

        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionAcquired {
                waited: Duration::ZERO
            })
        );
        assert_eq!(
            events.recv().await.ok(),
            Some(RateLimiterEvent::PermissionRejected)
        );
    }
}
