//! Guards for the quorum lock.
//!
//! A quorum guard holds one lease per node that granted it, so releasing it
//! releases every one of those nodes: a partial release is reported, never
//! silently swallowed.

use std::{sync::Arc, time::Duration};

use tracing::warn;

use crate::lock::{
    async_lock::watchdog::RenewalTask,
    backend::{BlockingLockBackend, LockBackend},
    blocking::watchdog::RenewalThread,
    error::{RedisLockError, UnlockOutcome},
    guard::GuardCore,
    protocol::commands::unlock,
};

/// What one node said about a release.
#[derive(Debug, Default)]
struct ReleaseTally {
    released: usize,
    reentrant: usize,
    not_owner: usize,
    failures: usize,
}

impl ReleaseTally {
    fn record(&mut self, reply: Option<i64>) {
        match reply {
            Some(1) => self.released += 1,
            Some(0) => self.reentrant += 1,
            _ => self.not_owner += 1,
        }
    }

    /// Fold the per-node outcomes into one result, judged at the quorum.
    fn outcome(self, quorum: usize) -> Result<UnlockOutcome, RedisLockError> {
        if self.released >= quorum {
            return Ok(UnlockOutcome::Released);
        }
        if self.failures > 0 {
            return Err(RedisLockError::PartiallyReleased {
                failures: self.failures,
            });
        }
        if self.reentrant > 0 {
            return Ok(UnlockOutcome::Reentrant);
        }
        if self.not_owner > 0 {
            return Err(RedisLockError::NotOwner);
        }

        Err(RedisLockError::QuorumUnavailable {
            required: quorum,
            acquired: 0,
        })
    }
}

/// An acquired quorum lock.
pub struct RedLockGuard {
    core: GuardCore,
    nodes: Arc<[Arc<dyn LockBackend>]>,
    watchdogs: Vec<RenewalTask>,
    command_timeout: Duration,
    unlocked: bool,
}

impl RedLockGuard {
    pub(crate) fn new(
        core: GuardCore,
        nodes: Arc<[Arc<dyn LockBackend>]>,
        watchdogs: Vec<RenewalTask>,
        command_timeout: Duration,
    ) -> Self {
        Self {
            core,
            nodes,
            watchdogs,
            command_timeout,
            unlocked: false,
        }
    }

    /// The lock name.
    pub fn name(&self) -> &str {
        &self.core.name
    }

    /// The owner token every node was acquired under.
    pub fn owner_id(&self) -> &str {
        self.core.owner.as_str()
    }

    /// How many nodes granted this acquisition.
    pub fn node_count(&self) -> usize {
        self.core.leases.len()
    }

    /// How many nodes are required for it to be valid.
    pub fn quorum(&self) -> usize {
        self.core.quorum
    }

    /// Whether enough nodes lost their lease to invalidate the guard.
    pub fn is_lost(&self) -> bool {
        self.core.is_lost()
    }

    /// Release every node that granted this acquisition.
    ///
    /// # Errors
    ///
    /// [`RedisLockError::WatchdogLost`] when the lease was already lost (nothing
    /// is sent to Redis), [`RedisLockError::PartiallyReleased`] when some nodes
    /// failed, and [`RedisLockError::NotOwner`] when no node recognises the owner.
    pub async fn unlock(mut self) -> Result<UnlockOutcome, RedisLockError> {
        self.watchdogs.clear();
        self.unlocked = true;

        if self.core.is_lost() {
            return Err(RedisLockError::WatchdogLost);
        }

        let mut tally = ReleaseTally::default();
        for lease in &self.core.leases {
            let Some(node) = self.nodes.get(lease.index) else {
                tally.failures += 1;
                continue;
            };
            let request = unlock(
                &self.core.name,
                self.core.owner.as_str(),
                lease.lease.duration,
                node.publish_command(),
            )?;

            match node.eval(&request, self.command_timeout).await {
                Ok(reply) => tally.record(reply.int()),
                Err(error) => {
                    warn!(lock = %self.core.name, %error, "a quorum node refused the release");
                    tally.failures += 1;
                }
            }
        }

        tally.outcome(self.core.quorum)
    }
}

impl Drop for RedLockGuard {
    fn drop(&mut self) {
        if self.unlocked {
            return;
        }

        self.watchdogs.clear();

        warn!(
            lock = %self.core.name,
            owner = self.core.owner.as_str(),
            "quorum lock guard dropped without an explicit unlock; the leases will expire"
        );
    }
}

/// An acquired blocking quorum lock.
pub struct BlockingRedLockGuard {
    core: GuardCore,
    nodes: Arc<[Arc<dyn BlockingLockBackend>]>,
    renewals: Vec<RenewalThread>,
    command_timeout: Duration,
    unlocked: bool,
}

impl BlockingRedLockGuard {
    pub(crate) fn new(
        core: GuardCore,
        nodes: Arc<[Arc<dyn BlockingLockBackend>]>,
        renewals: Vec<RenewalThread>,
        command_timeout: Duration,
    ) -> Self {
        Self {
            core,
            nodes,
            renewals,
            command_timeout,
            unlocked: false,
        }
    }

    /// The lock name.
    pub fn name(&self) -> &str {
        &self.core.name
    }

    /// The owner token every node was acquired under.
    pub fn owner_id(&self) -> &str {
        self.core.owner.as_str()
    }

    /// How many nodes granted this acquisition.
    pub fn node_count(&self) -> usize {
        self.core.leases.len()
    }

    /// How many nodes are required for it to be valid.
    pub fn quorum(&self) -> usize {
        self.core.quorum
    }

    /// Whether enough nodes lost their lease to invalidate the guard.
    pub fn is_lost(&self) -> bool {
        self.core.is_lost()
    }

    /// Release every node that granted this acquisition.
    ///
    /// # Errors
    ///
    /// As [`RedLockGuard::unlock`].
    pub fn unlock(mut self) -> Result<UnlockOutcome, RedisLockError> {
        self.renewals.clear();
        self.unlocked = true;

        if self.core.is_lost() {
            return Err(RedisLockError::WatchdogLost);
        }

        let mut tally = ReleaseTally::default();
        for lease in &self.core.leases {
            let Some(node) = self.nodes.get(lease.index) else {
                tally.failures += 1;
                continue;
            };
            let request = unlock(
                &self.core.name,
                self.core.owner.as_str(),
                lease.lease.duration,
                node.publish_command(),
            )?;

            match node.eval(&request, self.command_timeout) {
                Ok(reply) => tally.record(reply.int()),
                Err(error) => {
                    warn!(lock = %self.core.name, %error, "a quorum node refused the release");
                    tally.failures += 1;
                }
            }
        }

        tally.outcome(self.core.quorum)
    }
}

impl Drop for BlockingRedLockGuard {
    fn drop(&mut self) {
        if self.unlocked {
            return;
        }

        self.renewals.clear();

        warn!(
            lock = %self.core.name,
            owner = self.core.owner.as_str(),
            "quorum lock guard dropped without an explicit unlock; the leases will expire"
        );
    }
}
