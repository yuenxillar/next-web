//! The asynchronous guard returned by an acquisition.

use std::{sync::Arc, time::Duration};

use tracing::warn;

use crate::lock::{
    async_lock::watchdog::RenewalTask,
    backend::LockBackend,
    error::{RedisLockError, UnlockOutcome},
    guard::GuardCore,
    protocol::commands::unlock,
};

/// An acquired lock level.
///
/// It must be released with [`RedisLockGuard::unlock`]. Releasing needs a Redis
/// round trip, which `Drop` cannot perform, so dropping a guard without an
/// unlock only stops the watchdog and warns: the lease expires on its own.
pub struct RedisLockGuard {
    core: GuardCore,
    backend: Arc<dyn LockBackend>,
    watchdogs: Vec<RenewalTask>,
    command_timeout: Duration,
    unlocked: bool,
}

impl RedisLockGuard {
    pub(crate) fn new(
        core: GuardCore,
        backend: Arc<dyn LockBackend>,
        watchdogs: Vec<RenewalTask>,
        command_timeout: Duration,
    ) -> Self {
        Self {
            core,
            backend,
            watchdogs,
            command_timeout,
            unlocked: false,
        }
    }

    /// The lock name.
    pub fn name(&self) -> &str {
        &self.core.name
    }

    /// The owner token this level was acquired under.
    pub fn owner_id(&self) -> &str {
        self.core.owner.as_str()
    }

    /// The lease this level was granted.
    pub fn lease(&self) -> Duration {
        self.core.lease().duration
    }

    /// Whether the watchdog has lost ownership of this level.
    pub fn is_lost(&self) -> bool {
        self.core.is_lost()
    }

    /// Release one reentrant level.
    ///
    /// # Errors
    ///
    /// - [`RedisLockError::WatchdogLost`] when the lease was already lost. Nothing
    ///   is sent to Redis in that case: the key may belong to another owner by
    ///   now, and deleting it would release their lock instead of this one.
    /// - [`RedisLockError::NotOwner`] when Redis says this owner holds no level.
    pub async fn unlock(mut self) -> Result<UnlockOutcome, RedisLockError> {
        self.stop_watchdogs();
        self.unlocked = true;

        if self.core.is_lost() {
            return Err(RedisLockError::WatchdogLost);
        }

        let request = unlock(
            &self.core.name,
            self.core.owner.as_str(),
            self.lease(),
            self.backend.publish_command(),
        )?;

        let reply = self.backend.eval(&request, self.command_timeout).await?;

        match reply.int() {
            Some(1) => Ok(UnlockOutcome::Released),
            Some(0) => Ok(UnlockOutcome::Reentrant),
            _ => Err(RedisLockError::NotOwner),
        }
    }

    /// Stop every renewal task without waiting for them to observe the signal.
    ///
    /// Dropping a [`RenewalTask`] sends the cancel signal, so clearing the list
    /// is the whole operation.
    fn stop_watchdogs(&mut self) {
        self.watchdogs.clear();
    }
}

impl Drop for RedisLockGuard {
    fn drop(&mut self) {
        if self.unlocked {
            return;
        }

        self.stop_watchdogs();

        warn!(
            lock = %self.core.name,
            owner = self.core.owner.as_str(),
            "lock guard dropped without an explicit unlock; the lease will expire"
        );
    }
}
