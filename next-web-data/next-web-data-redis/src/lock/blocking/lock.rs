//! The blocking reentrant lock.
//!
//! Same semantics as [`crate::lock::async_lock::RedisLock`] through the
//! synchronous client: no `block_on`, no runtime requirement, and a caller that
//! blocks the thread it runs on. Wake-ups come from polling at the retry
//! interval rather than from Pub/Sub, because a synchronous subscriber would need
//! a second connection and a receive thread of its own.

use std::{sync::Arc, time::Duration};

use crate::lock::{
    backend::BlockingLockBackend,
    blocking::{guard::BlockingRedisLockGuard, watchdog::RenewalThread},
    config::RedisLockConfig,
    error::RedisLockError,
    guard::GuardCore,
    lease::{Lease, NodeLease},
    ownership::OwnerToken,
    protocol::{
        ScriptReply,
        commands::{acquire, force_unlock, status},
        keys::channel_name,
    },
    retry::{Deadline, next_delay},
};

/// A reentrant distributed lock with a synchronous API.
///
/// Every method blocks the calling thread. Call it from a blocking context (a
/// worker thread, `tokio::task::spawn_blocking`, a sync `main`), never from
/// inside an async task: the async flavour exists for that.
#[derive(Clone)]
pub struct BlockingRedisLock {
    backend: Arc<dyn BlockingLockBackend>,
    name: Arc<str>,
    owner: OwnerToken,
    config: Arc<RedisLockConfig>,
}

impl BlockingRedisLock {
    /// Build a handle. One handle means one owner token.
    pub fn new(
        backend: Arc<dyn BlockingLockBackend>,
        name: impl Into<String>,
        owner: OwnerToken,
        config: Arc<RedisLockConfig>,
    ) -> Self {
        Self {
            backend,
            name: Arc::from(name.into()),
            owner,
            config,
        }
    }

    /// The lock name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The owner token this handle acquires under.
    pub fn owner_id(&self) -> &str {
        self.owner.as_str()
    }

    /// Acquire and wait for as long as it takes, keeping the lease alive.
    ///
    /// # Errors
    ///
    /// Returns whatever the backend returned.
    pub fn lock(&self) -> Result<BlockingRedisLockGuard, RedisLockError> {
        self.acquire_with(
            Deadline::Forever,
            Lease::watchdog(self.config.watchdog_timeout),
        )?
        .ok_or_else(|| {
            RedisLockError::InvalidConfiguration(
                "a lock with no deadline must eventually acquire".to_string(),
            )
        })
    }

    /// Acquire with a fixed lease and no watchdog, waiting as long as it takes.
    ///
    /// # Errors
    ///
    /// As [`BlockingRedisLock::lock`], plus [`RedisLockError::InvalidDuration`].
    pub fn lock_with_lease(
        &self,
        lease: Duration,
    ) -> Result<BlockingRedisLockGuard, RedisLockError> {
        self.acquire_with(Deadline::Forever, Lease::fixed(lease))?
            .ok_or_else(|| {
                RedisLockError::InvalidConfiguration(
                    "a lock with no deadline must eventually acquire".to_string(),
                )
            })
    }

    /// Try once, without waiting.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn try_lock(&self) -> Result<Option<BlockingRedisLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::Once,
            Lease::watchdog(self.config.watchdog_timeout),
        )
    }

    /// Retry until `wait` elapses.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn try_lock_for(
        &self,
        wait: Duration,
    ) -> Result<Option<BlockingRedisLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::after(wait),
            Lease::watchdog(self.config.watchdog_timeout),
        )
    }

    /// Retry until `wait` elapses, with a fixed lease and no watchdog.
    ///
    /// # Errors
    ///
    /// As [`BlockingRedisLock::try_lock_for`], plus [`RedisLockError::InvalidDuration`].
    pub fn try_lock_for_with_lease(
        &self,
        wait: Duration,
        lease: Duration,
    ) -> Result<Option<BlockingRedisLockGuard>, RedisLockError> {
        self.acquire_with(Deadline::after(wait), Lease::fixed(lease))
    }

    /// Whether the lock is held by any owner.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn is_locked(&self) -> Result<bool, RedisLockError> {
        let reply = self.status()?;

        Ok(reply
            .ints()
            .and_then(|values| values.first())
            .is_some_and(|exists| *exists == 1))
    }

    /// Whether this handle's owner holds at least one level.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn is_held_by_owner(&self) -> Result<bool, RedisLockError> {
        Ok(self.hold_count()? > 0)
    }

    /// How many levels this handle's owner holds.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn hold_count(&self) -> Result<u32, RedisLockError> {
        let reply = self.status()?;

        Ok(reply
            .ints()
            .and_then(|values| values.get(1).copied())
            .unwrap_or(0)
            .max(0) as u32)
    }

    /// The remaining lease, or `None` when the lock has no expiry or is gone.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn remaining_lease(&self) -> Result<Option<Duration>, RedisLockError> {
        let reply = self.status()?;

        Ok(reply
            .ints()
            .and_then(|values| values.get(2).copied())
            .filter(|millis| *millis > 0)
            .map(|millis| Duration::from_millis(millis as u64)))
    }

    /// Delete the lock regardless of owner.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub fn force_unlock(&self) -> Result<bool, RedisLockError> {
        let request = force_unlock(&self.name, self.backend.publish_command());
        let reply = self.backend.eval(&request, self.config.command_timeout)?;

        Ok(reply.int() == Some(1))
    }

    /// Run `operation` while holding the lock, and always release afterwards.
    ///
    /// The operation's own error wins over a release error.
    ///
    /// # Errors
    ///
    /// Returns the acquisition or release error.
    pub fn with_lock<T, E>(
        &self,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<T, E>, RedisLockError> {
        let guard = self.lock()?;
        let result = operation();
        let released = guard.unlock();

        match (result, released) {
            (Ok(value), Ok(_)) => Ok(Ok(value)),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Ok(Err(error)),
        }
    }

    /// Shared acquisition loop.
    fn acquire_with(
        &self,
        deadline: Deadline,
        lease: Lease,
    ) -> Result<Option<BlockingRedisLockGuard>, RedisLockError> {
        self.config.validate()?;
        if lease.duration.is_zero() {
            return Err(RedisLockError::InvalidDuration { field: "lease" });
        }

        loop {
            let request = acquire(&self.name, self.owner.as_str(), lease.duration)?;

            // Never spend more than the caller has left: an attempt that starts
            // with 100ms of budget left must not be handed a 3s command timeout.
            let attempt_timeout = match deadline.remaining() {
                Some(Some(remaining)) => self.config.command_timeout.min(remaining),
                _ => self.config.command_timeout,
            };

            let reply = match self.backend.eval(&request, attempt_timeout) {
                Ok(reply) => reply,
                // A call that ran out because the caller's own budget ran out is
                // a failed acquisition, not a backend failure.
                Err(RedisLockError::CommandTimeout { .. }) if deadline.expired() => {
                    return Ok(None);
                }
                Err(error) => return Err(error),
            };

            match reply {
                ScriptReply::Nil => return Ok(Some(self.grant(lease))),
                ScriptReply::Int(ttl) => {
                    let Some(remaining) = deadline.remaining() else {
                        return Ok(None);
                    };
                    let delay = next_delay(
                        remaining,
                        self.config.retry_interval,
                        Some(Duration::from_millis(ttl.max(0) as u64)),
                    );
                    let Some(delay) = delay else {
                        return Ok(None);
                    };

                    self.backend
                        .wait_for_unlock(&self.name, &channel_name(&self.name), delay)?;
                }
                other => {
                    return Err(RedisLockError::UnexpectedReply(format!(
                        "the acquire script returned {other:?}"
                    )));
                }
            }
        }
    }

    /// Build the guard for an acquisition that just succeeded.
    fn grant(&self, lease: Lease) -> BlockingRedisLockGuard {
        let node = NodeLease::new(0, lease);

        let renewals = if lease.watchdog {
            vec![RenewalThread::spawn(
                self.backend.clone(),
                self.name.clone(),
                Arc::new(self.owner.clone()),
                lease,
                node.lost.clone(),
                self.config.command_timeout,
            )]
        } else {
            Vec::new()
        };

        BlockingRedisLockGuard::new(
            GuardCore::new(self.name.clone(), self.owner.clone(), 1, vec![node]),
            self.backend.clone(),
            renewals,
            self.config.command_timeout,
        )
    }

    /// Read the lock's state in one round trip.
    fn status(&self) -> Result<ScriptReply, RedisLockError> {
        let request = status(&self.name, self.owner.as_str());

        self.backend.eval(&request, self.config.command_timeout)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::lock::{
        backend::memory::MemoryLockBackend,
        config::RedisLockConfig,
        error::{RedisLockError, UnlockOutcome},
        test_support::{test_blocking_lock, test_blocking_lock_with},
    };

    #[test]
    fn blocking_two_owners_are_mutually_exclusive() {
        let backend = MemoryLockBackend::new_shared();

        let guard = test_blocking_lock(backend.clone(), "orders")
            .try_lock()
            .unwrap()
            .unwrap();
        assert!(
            test_blocking_lock(backend.clone(), "orders")
                .try_lock()
                .unwrap()
                .is_none()
        );
        assert_eq!(guard.unlock().unwrap(), UnlockOutcome::Released);
    }

    #[test]
    fn blocking_reentrancy_follows_the_handle() {
        let backend = MemoryLockBackend::new_shared();
        let lock = test_blocking_lock(backend.clone(), "orders");

        let first = lock.try_lock().unwrap().unwrap();
        let second = lock.clone().try_lock().unwrap().unwrap();

        assert_eq!(lock.hold_count().unwrap(), 2);
        assert_eq!(second.unlock().unwrap(), UnlockOutcome::Reentrant);
        assert_eq!(first.unlock().unwrap(), UnlockOutcome::Released);
        assert_eq!(backend.unlock_notifications().len(), 1);
    }

    #[test]
    fn blocking_watchdog_renews_until_unlock() {
        let backend = MemoryLockBackend::new_shared();
        let config = RedisLockConfig {
            watchdog_timeout: Duration::from_millis(90),
            command_timeout: Duration::from_millis(20),
            ..RedisLockConfig::default()
        };
        let lock = test_blocking_lock_with(backend.clone(), "orders", config);

        let guard = lock.lock().unwrap();
        std::thread::sleep(Duration::from_millis(200));

        assert!(!guard.is_lost());
        assert!(lock.is_held_by_owner().unwrap());
        assert!(backend.renew_count("orders") > 2);

        guard.unlock().unwrap();
    }

    #[test]
    fn a_dropped_blocking_guard_stops_renewing() {
        let backend = MemoryLockBackend::new_shared();
        let config = RedisLockConfig {
            watchdog_timeout: Duration::from_millis(90),
            command_timeout: Duration::from_millis(20),
            ..RedisLockConfig::default()
        };
        let lock = test_blocking_lock_with(backend.clone(), "orders", config);

        {
            let _guard = lock.lock().unwrap();
            std::thread::sleep(Duration::from_millis(120));
        }

        let renewals_after_drop = backend.renew_count("orders");
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(backend.renew_count("orders"), renewals_after_drop);
    }

    #[test]
    fn blocking_try_lock_for_waits_and_succeeds_after_release() {
        let backend = MemoryLockBackend::new_shared();
        let holder = test_blocking_lock(backend.clone(), "orders");
        let waiter = test_blocking_lock(backend.clone(), "orders");

        let guard = holder.try_lock().unwrap().unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            guard.unlock().unwrap();
        });

        let acquired = waiter.try_lock_for(Duration::from_secs(2)).unwrap();
        assert!(acquired.is_some());
        release.join().unwrap();
    }

    #[test]
    fn blocking_try_lock_for_respects_the_wait_budget() {
        let backend = MemoryLockBackend::new_shared();
        let _guard = test_blocking_lock(backend.clone(), "orders")
            .try_lock()
            .unwrap()
            .unwrap();

        let started = Instant::now();
        let contended = test_blocking_lock(backend.clone(), "orders")
            .try_lock_for(Duration::from_millis(150))
            .unwrap();

        assert!(contended.is_none());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn blocking_eval_timeout_surfaces_as_error() {
        let backend = MemoryLockBackend::new_shared();
        backend.set_eval_delay(Duration::from_millis(300));
        let config = RedisLockConfig {
            command_timeout: Duration::from_millis(50),
            ..RedisLockConfig::default()
        };
        let lock = test_blocking_lock_with(backend.clone(), "orders", config);

        assert!(matches!(
            lock.try_lock(),
            Err(RedisLockError::CommandTimeout { .. })
        ));
    }

    #[test]
    fn blocking_with_lock_releases_even_when_the_operation_fails() {
        let backend = MemoryLockBackend::new_shared();
        let lock = test_blocking_lock(backend.clone(), "orders");

        let result: Result<Result<(), &'static str>, RedisLockError> =
            lock.with_lock(|| Err("boom"));

        assert_eq!(result.unwrap(), Err("boom"));
        assert!(!lock.is_locked().unwrap());
    }

    #[test]
    fn a_lost_blocking_lease_is_reported_on_unlock() {
        let backend = MemoryLockBackend::new_shared();
        let config = RedisLockConfig {
            watchdog_timeout: Duration::from_millis(90),
            command_timeout: Duration::from_millis(20),
            ..RedisLockConfig::default()
        };
        let lock = test_blocking_lock_with(backend.clone(), "orders", config);

        let guard = lock.lock().unwrap();
        backend.expire_now("orders");
        std::thread::sleep(Duration::from_millis(150));

        assert!(guard.is_lost());
        assert!(matches!(guard.unlock(), Err(RedisLockError::WatchdogLost)));
    }
}

#[cfg(test)]
mod budget_tests {
    use std::time::Instant;

    use super::*;
    use crate::lock::{
        backend::memory::MemoryLockBackend, config::RedisLockConfig,
        test_support::test_blocking_lock_with,
    };

    #[test]
    fn blocking_try_lock_for_never_exceeds_its_budget() {
        let backend = MemoryLockBackend::new_shared();
        backend.set_eval_delay(Duration::from_millis(300));
        let config = RedisLockConfig {
            command_timeout: Duration::from_millis(300),
            ..RedisLockConfig::default()
        };
        let lock = test_blocking_lock_with(backend, "orders", config);

        let started = Instant::now();
        let acquired = lock.try_lock_for(Duration::from_millis(100)).unwrap();

        assert!(acquired.is_none());
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "try_lock_for spent {:?} on a 100ms budget",
            started.elapsed()
        );
    }
}
