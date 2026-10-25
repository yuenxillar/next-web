//! The blocking guard returned by an acquisition.

use std::{sync::Arc, time::Duration};

use tracing::warn;

use crate::lock::{
    backend::BlockingLockBackend,
    blocking::watchdog::RenewalThread,
    error::{RedisLockError, UnlockOutcome},
    guard::GuardCore,
    protocol::commands::unlock,
};

/// An acquired lock level from a blocking handle.
///
/// It must be released with [`BlockingRedisLockGuard::unlock`]. Dropping it only
/// stops the renewal thread and warns, because a release needs a Redis round
/// trip: the lease expires on its own.
pub struct BlockingRedisLockGuard {
    core: GuardCore,
    backend: Arc<dyn BlockingLockBackend>,
    renewals: Vec<RenewalThread>,
    command_timeout: Duration,
    unlocked: bool,
}

impl BlockingRedisLockGuard {
    pub(crate) fn new(
        core: GuardCore,
        backend: Arc<dyn BlockingLockBackend>,
        renewals: Vec<RenewalThread>,
        command_timeout: Duration,
    ) -> Self {
        Self {
            core,
            backend,
            renewals,
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
    /// - [`RedisLockError::WatchdogLost`] when the lease was already lost; nothing
    ///   is sent to Redis in that case, so the next owner's lock is left alone.
    /// - [`RedisLockError::NotOwner`] when Redis says this owner holds no level.
    pub fn unlock(mut self) -> Result<UnlockOutcome, RedisLockError> {
        self.stop_renewals();
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

        let reply = self.backend.eval(&request, self.command_timeout)?;

        match reply.int() {
            Some(1) => Ok(UnlockOutcome::Released),
            Some(0) => Ok(UnlockOutcome::Reentrant),
            _ => Err(RedisLockError::NotOwner),
        }
    }

    fn stop_renewals(&mut self) {
        self.renewals.clear();
    }
}

impl Drop for BlockingRedisLockGuard {
    fn drop(&mut self) {
        if self.unlocked {
            return;
        }

        self.stop_renewals();

        warn!(
            lock = %self.core.name,
            owner = self.core.owner.as_str(),
            "lock guard dropped without an explicit unlock; the lease will expire"
        );
    }
}
