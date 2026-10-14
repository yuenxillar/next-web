//! The permission store a rate limiter asks for its budget.
//!
//! A [`RateLimiter`](crate::ratelimiter::RateLimiter) owns no budget itself: it
//! turns a call into a [`PermissionRequest`] and lets a [`RateLimiterStore`]
//! decide. The default store, [`InMemoryRateLimiterStore`], keeps the budget in
//! one atomic word in this process. Implement the trait to put the budget
//! somewhere every instance can see, which is what a fleet behind a load
//! balancer needs: without a shared store each instance hands out the full quota
//! on its own, so the real rate is the quota multiplied by the number of
//! instances.
//!
//! # Writing a store
//!
//! The trait has a single operation, and it is deliberately not a
//! `get`/`set` pair. Two instances reading the same state and both writing back
//! "I took one" would each believe they were the only caller, so a store must
//! decide and decrement in one indivisible step. On Redis that is a server side
//! script — `INCR` with an expiry, or a Lua script holding the window counter —
//! never a read followed by a write.
//!
//! ```text
//! -- The shape a Redis store needs: decide and decrement in one step.
//! local current = redis.call('INCR', KEYS[1])
//! if current == 1 then
//!   redis.call('PEXPIRE', KEYS[1], ARGV[1])
//! end
//! if current > tonumber(ARGV[2]) then
//!   -- Refuse, and report how long the caller would have to wait.
//!   return redis.call('PTTL', KEYS[1])
//! end
//! return 0
//! ```
//!
//! Two further rules for a remote store:
//!
//! * **Never sleep inside `try_acquire`.** Waiting is the limiter's job, driven
//!   by the [`Refusal`] the store returns. A store that sleeps holds a
//!   connection for a decision that another period would answer anyway.
//! * **Prefer the store's own clock.** [`PermissionRequest::now`] carries the
//!   limiter's clock reading, which is what makes the in-memory store testable
//!   with a [`ManualClock`](crate::clock::ManualClock). A store in another
//!   process should ignore it and use its own clock — Redis `TIME`, or the
//!   script's expiry — so that instances do not have to keep their clocks in
//!   step with each other.

use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use next_web_core::async_trait;

/// One request for a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionRequest<'a> {
    /// Name of the guarded resource, used to key a shared budget.
    ///
    /// The limiter's name, so two limiters that were given the same name share
    /// one budget when they share a store.
    pub name: &'a str,
    /// Permissions granted per period.
    pub limit_for_period: u32,
    /// Length of one period.
    pub limit_refresh_period: Duration,
    /// The limiter's clock reading for this decision.
    ///
    /// Periods start at every multiple of `limit_refresh_period` counted from
    /// the clock's epoch, so this value alone identifies the period.
    pub now: Duration,
}

/// Why a store refused a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    /// How long until the period that could serve the request begins.
    ///
    /// This is the exact time remaining until the boundary, so a caller that
    /// sleeps for it is served by the very next period instead of being kept
    /// waiting for one it did not need.
    pub wait_until_next_period: Duration,
}

/// Decides whether a call may take one of a resource's permissions.
///
/// See the [module documentation](self) for what an implementation has to
/// guarantee, and for the shape a Redis-backed one takes.
#[async_trait]
pub trait RateLimiterStore: Send + Sync + fmt::Debug {
    /// Takes one permission for the period `request.now` falls into.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal`] when the budget of that period is already spent. The
    /// store must not sleep: waiting is the limiter's job.
    async fn try_acquire(&self, request: PermissionRequest<'_>) -> Result<(), Refusal>;

    /// Returns the permissions left in the period `request.now` falls into, as
    /// far as this store can tell **without a round trip**.
    ///
    /// This backs [`RateLimiter::snapshot`](crate::ratelimiter::RateLimiter::snapshot),
    /// which is synchronous and must not await, so the default implementation
    /// answers `None` — "not knowable locally" — and a store in another process
    /// should keep that default rather than pretend. A local store overrides it
    /// and answers exactly.
    fn available_permissions(&self, _request: &PermissionRequest<'_>) -> Option<u32> {
        None
    }
}

/// Number of bits the permission count occupies in the packed state.
const PERMISSIONS_BITS: u32 = 32;
/// Mask selecting the permission count in the packed state.
const PERMISSIONS_MASK: u64 = u32::MAX as u64;
/// Cycle number reserved to mean "no period has been served yet".
const UNSTARTED_CYCLE: u32 = u32::MAX;
/// Highest cycle number a real period may report, so that the reserved value
/// above can never be produced by the clock.
const MAX_CYCLE: u32 = u32::MAX - 1;

/// A [`RateLimiterStore`] that keeps the budget in this process.
///
/// The whole budget lives in one atomic word: a caller either swaps in a fresh
/// budget, when it is the first to notice that the clock crossed a refresh
/// boundary, or takes one permission away from the current budget. Calls
/// therefore never wait for each other and no lock is taken on the hot path.
///
/// This is the store [`RateLimiter::new`](crate::ratelimiter::RateLimiter::new)
/// installs, and it is the right one for a single instance. It is **not**
/// shared between processes: see the [module documentation](self).
#[derive(Debug)]
pub struct InMemoryRateLimiterStore {
    /// Packed `(cycle, permissions)`; see [`pack`] and [`unpack_cycle`].
    state: AtomicU64,
}

impl InMemoryRateLimiterStore {
    /// Creates a store holding the full budget of its first period.
    pub fn new() -> Self {
        // The reserved cycle means the first request always looks like the start
        // of a period, so it installs a fresh budget instead of finding a spent
        // one. Starting from a zeroed word would encode
        // `(cycle = 0, permissions = 0)` — exactly what a spent period zero looks
        // like — and a clock that starts at zero would then refuse every call of
        // that period.
        Self {
            state: AtomicU64::new(pack(UNSTARTED_CYCLE, 0)),
        }
    }
}

impl Default for InMemoryRateLimiterStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RateLimiterStore for InMemoryRateLimiterStore {
    async fn try_acquire(&self, request: PermissionRequest<'_>) -> Result<(), Refusal> {
        loop {
            let cycle = cycle_of(&request);
            let observed = self.state.load(Ordering::SeqCst);

            // A fresh budget starts one permission down, because this request is
            // taking one of them. The configuration guarantees a limit of at
            // least one, so a fresh budget always grants.
            let (next, permitted) = if unpack_cycle(observed) != cycle {
                (pack(cycle, request.limit_for_period.saturating_sub(1)), true)
            } else {
                match unpack_permissions(observed) {
                    // The state already describes a spent period: writing the
                    // limit back would make the budget look unspent to the next
                    // caller for as long as this period lasts.
                    0 => (observed, false),
                    available => (pack(cycle, available - 1), true),
                }
            };

            if !permitted {
                return Err(Refusal {
                    wait_until_next_period: wait_until_next_cycle(&request, cycle),
                });
            }

            if self
                .state
                .compare_exchange(observed, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(());
            }
            // Another caller won the swap; read the state again and retry.
        }
    }

    fn available_permissions(&self, request: &PermissionRequest<'_>) -> Option<u32> {
        let observed = self.state.load(Ordering::SeqCst);

        if unpack_cycle(observed) == cycle_of(request) {
            Some(unpack_permissions(observed))
        } else {
            // The state belongs to an older period, so the whole budget is
            // available again.
            Some(request.limit_for_period)
        }
    }
}

/// Packs a cycle and its remaining permissions into one word.
fn pack(cycle: u32, permissions: u32) -> u64 {
    (u64::from(cycle) << PERMISSIONS_BITS) | u64::from(permissions)
}

/// Returns the cycle of a packed state.
fn unpack_cycle(state: u64) -> u32 {
    (state >> PERMISSIONS_BITS) as u32
}

/// Returns the permissions left in a packed state.
fn unpack_permissions(state: u64) -> u32 {
    (state & PERMISSIONS_MASK) as u32
}

/// Returns the period `request.now` falls into.
fn cycle_of(request: &PermissionRequest<'_>) -> u32 {
    let period = request.limit_refresh_period.as_nanos();
    if period == 0 {
        // Validation rejects a zero period; without one every call would be its
        // own period.
        return MAX_CYCLE;
    }

    u32::try_from(request.now.as_nanos() / period)
        .unwrap_or(MAX_CYCLE)
        .min(MAX_CYCLE)
}

/// Returns how long is left of the period numbered `cycle`.
///
/// The result is the exact time to the next boundary. Waking *at* the boundary
/// is enough, because a period is identified by
/// `floor(now / limit_refresh_period)` and that quotient changes exactly at the
/// boundary. The value is never zero: `cycle` comes from the request that was
/// just refused, so `now` lies strictly inside that period.
fn wait_until_next_cycle(request: &PermissionRequest<'_>, cycle: u32) -> Duration {
    let period = request.limit_refresh_period.as_nanos();
    if period == 0 {
        // Validation rejects a zero period; there is no boundary to wait for.
        return Duration::ZERO;
    }

    let boundary = (u128::from(cycle) + 1).saturating_mul(period);
    let wait = boundary.saturating_sub(request.now.as_nanos());

    Duration::from_nanos(u64::try_from(wait).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{
        InMemoryRateLimiterStore, PermissionRequest, RateLimiterStore, cycle_of,
        wait_until_next_cycle,
    };
    use crate::clock::{Clock, ManualClock};

    /// A store shared behind the trait object a limiter actually holds.
    fn store() -> Arc<dyn RateLimiterStore> {
        Arc::new(InMemoryRateLimiterStore::new())
    }

    fn request<'a>(name: &'a str, limit: u32, period: Duration, now: Duration) -> PermissionRequest<'a> {
        PermissionRequest {
            name,
            limit_for_period: limit,
            limit_refresh_period: period,
            now,
        }
    }

    #[tokio::test]
    async fn a_fresh_store_grants_the_whole_budget_then_refuses() {
        let store = store();
        let period = Duration::from_secs(1);

        for _ in 0..4 {
            store
                .try_acquire(request("api", 4, period, Duration::ZERO))
                .await
                .expect("the budget lasts four calls");
        }

        assert!(
            store
                .try_acquire(request("api", 4, period, Duration::ZERO))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn the_next_period_refreshes_the_budget() {
        let store = store();
        let period = Duration::from_secs(1);

        store
            .try_acquire(request("api", 1, period, Duration::ZERO))
            .await
            .expect("the first period grants one");

        let refusal = store
            .try_acquire(request("api", 1, period, Duration::from_millis(400)))
            .await
            .expect_err("the first period is spent");
        assert_eq!(refusal.wait_until_next_period, Duration::from_millis(600));

        store
            .try_acquire(request("api", 1, period, Duration::from_secs(1)))
            .await
            .expect("the second period grants one");
    }

    #[tokio::test]
    async fn unused_permissions_are_not_carried_over() {
        let store = store();
        let period = Duration::from_secs(1);

        // Four granted, none of the remaining budget may survive the boundary.
        store
            .try_acquire(request("api", 8, period, Duration::ZERO))
            .await
            .expect("granted");

        for _ in 0..4 {
            store
                .try_acquire(request("api", 4, period, Duration::from_secs(1)))
                .await
                .expect("the second period grants four of its own");
        }

        assert!(
            store
                .try_acquire(request("api", 4, period, Duration::from_secs(1)))
                .await
                .is_err(),
            "the second period must grant four, not twelve"
        );
    }

    #[tokio::test]
    async fn a_local_store_can_report_what_is_left() {
        let store = InMemoryRateLimiterStore::new();
        let period = Duration::from_secs(1);

        assert_eq!(
            store.available_permissions(&request("api", 3, period, Duration::ZERO)),
            Some(3)
        );

        store
            .try_acquire(request("api", 3, period, Duration::ZERO))
            .await
            .expect("granted");
        assert_eq!(
            store.available_permissions(&request("api", 3, period, Duration::ZERO)),
            Some(2)
        );

        // Once the period has turned the whole budget is back.
        assert_eq!(
            store.available_permissions(&request("api", 3, period, Duration::from_secs(1))),
            Some(3)
        );
    }

    #[tokio::test]
    async fn a_store_whose_clock_starts_late_still_grants_the_whole_budget() {
        let clock = ManualClock::new();
        clock.advance(Duration::from_secs(3600));

        let store = store();
        let period = Duration::from_secs(1);
        let now = clock.monotonic_time();

        for _ in 0..2 {
            store
                .try_acquire(request("api", 2, period, now))
                .await
                .expect("the budget of a period that started long ago still lasts");
        }
        assert!(
            store
                .try_acquire(request("api", 2, period, now))
                .await
                .is_err()
        );
    }

    #[test]
    fn the_refusal_points_exactly_at_the_boundary() {
        let period = Duration::from_millis(100);
        let request = request("api", 1, period, Duration::from_millis(1));

        // The caller must not be made to wait a whole extra period.
        assert_eq!(wait_until_next_cycle(&request, 0), Duration::from_millis(99));
    }

    #[test]
    fn a_zero_period_cannot_be_turned_into_a_cycle() {
        let request = request("api", 1, Duration::ZERO, Duration::from_secs(5));

        assert_eq!(cycle_of(&request), u32::MAX - 1);
        assert_eq!(wait_until_next_cycle(&request, 0), Duration::ZERO);
    }

    #[test]
    fn the_reserved_cycle_can_never_be_produced_by_a_clock() {
        let request = request(
            "api",
            1,
            Duration::from_nanos(1),
            Duration::from_secs(86_400),
        );

        // A saturated cycle must not collide with the value that means "no
        // period has been served yet", or a fresh store would look spent.
        assert!(cycle_of(&request) < u32::MAX);
    }
}
