//! The asynchronous reentrant lock.

use std::{future::Future, sync::Arc, time::Duration};

use crate::lock::{
    async_lock::{guard::RedisLockGuard, watchdog::RenewalTask},
    backend::LockBackend,
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

/// A reentrant distributed lock on one Redis deployment.
///
/// A handle owns one owner token, which its clones share: acquiring twice
/// through the same handle (or a clone) is reentrant, while a second
/// [`crate::lock::service::RedisLockService::get_lock`] call contends. The token
/// stands in for the thread identity Redisson uses, because a lock can be held
/// across `.await` points where no thread identity survives.
#[derive(Clone)]
pub struct RedisLock {
    backend: Arc<dyn LockBackend>,
    name: Arc<str>,
    owner: OwnerToken,
    config: Arc<RedisLockConfig>,
}

impl RedisLock {
    /// Build a handle. One handle means one owner token.
    pub fn new(
        backend: Arc<dyn LockBackend>,
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
    /// Returns whatever the backend returned, plus
    /// [`RedisLockError::InvalidDuration`] for an unusable configuration.
    pub async fn lock(&self) -> Result<RedisLockGuard, RedisLockError> {
        self.acquire_with(
            Deadline::Forever,
            Lease::watchdog(self.config.watchdog_timeout),
        )
        .await?
        .ok_or_else(|| {
            RedisLockError::InvalidConfiguration(
                "a lock with no deadline must eventually acquire".to_string(),
            )
        })
    }

    /// Acquire with a fixed lease and no watchdog, waiting for as long as it takes.
    ///
    /// # Errors
    ///
    /// As [`RedisLock::lock`], plus [`RedisLockError::InvalidDuration`] when
    /// `lease` is zero.
    pub async fn lock_with_lease(&self, lease: Duration) -> Result<RedisLockGuard, RedisLockError> {
        self.acquire_with(Deadline::Forever, Lease::fixed(lease))
            .await?
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
    pub async fn try_lock(&self) -> Result<Option<RedisLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::Once,
            Lease::watchdog(self.config.watchdog_timeout),
        )
        .await
    }

    /// Retry until `wait` elapses.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub async fn try_lock_for(
        &self,
        wait: Duration,
    ) -> Result<Option<RedisLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::after(wait),
            Lease::watchdog(self.config.watchdog_timeout),
        )
        .await
    }

    /// Retry until `wait` elapses, with a fixed lease and no watchdog.
    ///
    /// # Errors
    ///
    /// As [`RedisLock::try_lock_for`], plus [`RedisLockError::InvalidDuration`].
    pub async fn try_lock_for_with_lease(
        &self,
        wait: Duration,
        lease: Duration,
    ) -> Result<Option<RedisLockGuard>, RedisLockError> {
        self.acquire_with(Deadline::after(wait), Lease::fixed(lease))
            .await
    }

    /// Whether the lock is held by any owner.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub async fn is_locked(&self) -> Result<bool, RedisLockError> {
        let reply = self.status().await?;

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
    pub async fn is_held_by_owner(&self) -> Result<bool, RedisLockError> {
        Ok(self.hold_count().await? > 0)
    }

    /// How many levels this handle's owner holds.
    ///
    /// # Errors
    ///
    /// Returns the backend error.
    pub async fn hold_count(&self) -> Result<u32, RedisLockError> {
        let reply = self.status().await?;

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
    pub async fn remaining_lease(&self) -> Result<Option<Duration>, RedisLockError> {
        let reply = self.status().await?;

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
    pub async fn force_unlock(&self) -> Result<bool, RedisLockError> {
        let request = force_unlock(&self.name, self.backend.publish_command());
        let reply = self
            .backend
            .eval(&request, self.config.command_timeout)
            .await?;

        Ok(reply.int() == Some(1))
    }

    /// Run `operation` while holding the lock, and always release afterwards.
    ///
    /// The operation's own error wins over a release error: a caller who has an
    /// error to report should not have it replaced by a cleanup failure.
    ///
    /// # Errors
    ///
    /// Returns the acquisition or release error.
    pub async fn with_lock<T, E, F, Fut>(
        &self,
        operation: F,
    ) -> Result<Result<T, E>, RedisLockError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let guard = self.lock().await?;
        let result = operation().await;
        let released = guard.unlock().await;

        match (result, released) {
            (Ok(value), Ok(_)) => Ok(Ok(value)),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Ok(Err(error)),
        }
    }

    /// Shared acquisition loop.
    async fn acquire_with(
        &self,
        deadline: Deadline,
        lease: Lease,
    ) -> Result<Option<RedisLockGuard>, RedisLockError> {
        self.config.validate()?;
        if lease.duration.is_zero() {
            return Err(RedisLockError::InvalidDuration { field: "lease" });
        }

        loop {
            let request = acquire(&self.name, self.owner.as_str(), lease.duration)?;
            let reply = self
                .backend
                .eval(&request, self.config.command_timeout)
                .await?;

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
                        .wait_for_unlock(&self.name, &channel_name(&self.name), delay)
                        .await?;
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
    fn grant(&self, lease: Lease) -> RedisLockGuard {
        let node = NodeLease::new(0, lease);

        let watchdogs = if lease.watchdog {
            vec![RenewalTask::spawn(
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

        RedisLockGuard::new(
            GuardCore::new(self.name.clone(), self.owner.clone(), 1, vec![node]),
            self.backend.clone(),
            watchdogs,
            self.config.command_timeout,
        )
    }

    /// Read the lock's state in one round trip.
    async fn status(&self) -> Result<ScriptReply, RedisLockError> {
        let request = status(&self.name, self.owner.as_str());

        self.backend
            .eval(&request, self.config.command_timeout)
            .await
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
        test_support::{test_lock, test_lock_with},
    };

    #[tokio::test]
    async fn two_owners_are_mutually_exclusive() {
        let backend = MemoryLockBackend::new_shared();
        let first = test_lock(backend.clone(), "orders");
        let second = test_lock(backend.clone(), "orders");

        let guard = first.try_lock().await.unwrap().unwrap();
        assert!(second.try_lock().await.unwrap().is_none());
        assert!(second.is_locked().await.unwrap());
        assert!(!second.is_held_by_owner().await.unwrap());

        guard.unlock().await.unwrap();
        let acquired = second.try_lock().await.unwrap().unwrap();
        assert!(second.is_held_by_owner().await.unwrap());
        acquired.unlock().await.unwrap();
    }

    #[tokio::test]
    async fn second_lock_on_the_same_handle_is_reentrant() {
        let backend = MemoryLockBackend::new_shared();
        let lock = test_lock(backend.clone(), "orders");

        let first = lock.try_lock().await.unwrap().unwrap();
        let second = lock.clone().try_lock().await.unwrap().unwrap();

        assert_eq!(lock.hold_count().await.unwrap(), 2);
        assert_eq!(first.owner_id(), second.owner_id());
        assert_eq!(second.unlock().await.unwrap(), UnlockOutcome::Reentrant);
        assert_eq!(first.unlock().await.unwrap(), UnlockOutcome::Released);
        assert_eq!(backend.unlock_notifications().len(), 1);
    }

    #[tokio::test]
    async fn force_unlock_releases_a_lock_the_caller_does_not_own() {
        let backend = MemoryLockBackend::new_shared();
        let owner = test_lock(backend.clone(), "orders");
        let intruder = test_lock(backend.clone(), "orders");

        let guard = owner.try_lock().await.unwrap().unwrap();
        assert!(!intruder.is_held_by_owner().await.unwrap());
        assert!(intruder.force_unlock().await.unwrap());
        assert!(!owner.is_locked().await.unwrap());

        // The owner's hash field is gone, so releasing that level is refused.
        assert!(matches!(
            guard.unlock().await,
            Err(RedisLockError::NotOwner)
        ));
    }

    #[tokio::test]
    async fn try_lock_for_waits_and_succeeds_after_release() {
        let backend = MemoryLockBackend::new_shared();
        let holder = test_lock(backend.clone(), "orders");
        let waiter = test_lock(backend.clone(), "orders");

        let guard = holder.try_lock().await.unwrap().unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            guard.unlock().await.unwrap();
        });

        let acquired = waiter.try_lock_for(Duration::from_secs(2)).await.unwrap();
        assert!(acquired.is_some());
        release.await.unwrap();
    }

    #[tokio::test]
    async fn try_lock_for_respects_the_wait_budget() {
        let backend = MemoryLockBackend::new_shared();
        let holder = test_lock(backend.clone(), "orders");
        let _guard = holder.try_lock().await.unwrap().unwrap();

        let started = Instant::now();
        let contended = test_lock(backend.clone(), "orders")
            .try_lock_for(Duration::from_millis(150))
            .await
            .unwrap();

        assert!(contended.is_none());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn watchdog_renews_until_unlock() {
        let backend = MemoryLockBackend::new_shared();
        let config = RedisLockConfig {
            watchdog_timeout: Duration::from_millis(90),
            ..RedisLockConfig::default()
        };
        let lock = test_lock_with(backend.clone(), "orders", config);

        let guard = lock.lock().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(!guard.is_lost());
        assert!(lock.is_held_by_owner().await.unwrap());
        assert!(backend.renew_count("orders") > 3);

        guard.unlock().await.unwrap();
    }

    #[tokio::test]
    async fn lost_lease_is_reported_on_unlock() {
        let backend = MemoryLockBackend::new_shared();
        let config = RedisLockConfig {
            watchdog_timeout: Duration::from_millis(90),
            ..RedisLockConfig::default()
        };
        let lock = test_lock_with(backend.clone(), "orders", config);

        let guard = lock.lock().await.unwrap();
        backend.expire_now("orders");
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(guard.is_lost());
        assert!(matches!(
            guard.unlock().await,
            Err(RedisLockError::WatchdogLost)
        ));
    }

    #[tokio::test]
    async fn eval_timeout_surfaces_as_error() {
        let backend = MemoryLockBackend::new_shared();
        backend.set_eval_delay(Duration::from_millis(300));
        let config = RedisLockConfig {
            command_timeout: Duration::from_millis(50),
            ..RedisLockConfig::default()
        };
        let lock = test_lock_with(backend.clone(), "orders", config);

        assert!(matches!(
            lock.try_lock().await,
            Err(RedisLockError::CommandTimeout { .. })
        ));
    }

    #[tokio::test]
    async fn with_lock_releases_even_when_the_operation_fails() {
        let backend = MemoryLockBackend::new_shared();
        let lock = test_lock(backend.clone(), "orders");

        let result: Result<Result<(), &'static str>, RedisLockError> =
            lock.with_lock(|| async { Err("boom") }).await;

        assert_eq!(result.unwrap(), Err("boom"));
        assert!(!lock.is_locked().await.unwrap());
    }
}
