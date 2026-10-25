//! The quorum lock: at least three independent deployments, a majority of votes.
//!
//! This is the Redlock construction. It earns its extra failure modes only when
//! the deployments are genuinely independent; with one node it degenerates into
//! a slower [`crate::lock::async_lock::RedisLock`], so it refuses that
//! configuration outright.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::future::join_all;

use crate::lock::{
    async_lock::watchdog::RenewalTask,
    backend::{BlockingLockBackend, LockBackend},
    blocking::watchdog::RenewalThread,
    config::RedisLockConfig,
    error::RedisLockError,
    guard::GuardCore,
    lease::{Lease, NodeLease},
    ownership::OwnerToken,
    protocol::{ScriptReply, commands::acquire},
    redlock::guard::{BlockingRedLockGuard, RedLockGuard},
    retry::Deadline,
};

/// The number of nodes that must agree: a strict majority.
pub const fn quorum_for(node_count: usize) -> usize {
    node_count / 2 + 1
}

/// How many nodes a quorum lock needs to be worth using.
const MIN_NODES: usize = 3;

/// How long a failed attempt may take before its result is no longer trusted.
///
/// A lock acquired over several nodes can lose its validity while the attempts
/// are still running, so the elapsed time plus this drift has to stay under the
/// lease. The drift covers clock rate between the client and the servers.
fn validity_drift(lease: Duration) -> Duration {
    Duration::from_millis(2).saturating_add(lease / 100)
}

/// Wait before the next attempt, or `None` when the caller should stop.
fn retry_delay(deadline: Deadline, retry_interval: Duration) -> Option<Duration> {
    crate::lock::retry::next_delay(deadline.remaining()?, retry_interval, None)
}

/// A quorum lock across independent Redis deployments.
#[derive(Clone)]
pub struct RedLock {
    nodes: Arc<[Arc<dyn LockBackend>]>,
    quorum: usize,
    name: Arc<str>,
    owner: OwnerToken,
    config: Arc<RedisLockConfig>,
}

impl RedLock {
    /// Build a quorum lock over `nodes`.
    ///
    /// # Errors
    ///
    /// [`RedisLockError::InvalidConfiguration`] when fewer than three nodes are
    /// given: a majority of two is not a quorum worth trusting.
    pub fn new(
        nodes: Vec<Arc<dyn LockBackend>>,
        name: impl Into<String>,
        owner: OwnerToken,
        config: Arc<RedisLockConfig>,
    ) -> Result<Self, RedisLockError> {
        if nodes.len() < MIN_NODES {
            return Err(RedisLockError::InvalidConfiguration(format!(
                "a quorum lock needs at least {MIN_NODES} independent Redis URLs, got {}",
                nodes.len()
            )));
        }

        let quorum = quorum_for(nodes.len());

        Ok(Self {
            nodes: nodes.into(),
            quorum,
            name: Arc::from(name.into()),
            owner,
            config,
        })
    }

    /// The lock name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The owner token every node is acquired under.
    pub fn owner_id(&self) -> &str {
        self.owner.as_str()
    }

    /// How many nodes must agree.
    pub fn quorum(&self) -> usize {
        self.quorum
    }

    /// How many nodes are configured.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Acquire a majority, waiting for as long as it takes.
    ///
    /// # Errors
    ///
    /// Returns the backend error when no attempt could be evaluated at all.
    pub async fn lock(&self) -> Result<RedLockGuard, RedisLockError> {
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

    /// Try once, without waiting.
    ///
    /// # Errors
    ///
    /// Returns the backend error when no attempt could be evaluated at all.
    pub async fn try_lock(&self) -> Result<Option<RedLockGuard>, RedisLockError> {
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
    /// Returns the backend error when no attempt could be evaluated at all.
    pub async fn try_lock_for(
        &self,
        wait: Duration,
    ) -> Result<Option<RedLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::after(wait),
            Lease::watchdog(self.config.watchdog_timeout),
        )
        .await
    }

    /// Delete the lock on every node, regardless of owner.
    ///
    /// # Errors
    ///
    /// Returns [`RedisLockError::QuorumUnavailable`] when fewer than a quorum of
    /// nodes confirmed the deletion.
    pub async fn force_unlock(&self) -> Result<bool, RedisLockError> {
        let publishes: Vec<&'static str> = self
            .nodes
            .iter()
            .map(|node| node.publish_command())
            .collect();
        let futures = self.nodes.iter().zip(publishes).map(|(node, publish)| {
            let request = crate::lock::protocol::commands::force_unlock(&self.name, publish);
            async move { node.eval(&request, self.config.command_timeout).await }
        });
        let results = join_all(futures).await;

        self.count_quorum(results.into_iter().filter_map(Result::ok))
    }

    /// One acquisition attempt across every node.
    async fn acquire_with(
        &self,
        deadline: Deadline,
        lease: Lease,
    ) -> Result<Option<RedLockGuard>, RedisLockError> {
        self.config.validate()?;
        if lease.duration.is_zero() {
            return Err(RedisLockError::InvalidDuration { field: "lease" });
        }

        loop {
            let started = Instant::now();
            let request = acquire(&self.name, self.owner.as_str(), lease.duration)?;

            let results = join_all(
                self.nodes
                    .iter()
                    .map(|node| node.eval(&request, self.config.command_timeout)),
            )
            .await;

            let mut acquired = Vec::new();
            for (index, result) in results.into_iter().enumerate() {
                match result {
                    Ok(ScriptReply::Nil) => acquired.push(index),
                    Ok(_) => {}
                    Err(_) => {}
                }
            }

            let valid = started
                .elapsed()
                .saturating_add(validity_drift(lease.duration))
                < lease.duration;

            if acquired.len() >= self.quorum && valid {
                return Ok(Some(self.grant(lease, acquired)));
            }

            // A partial acquisition must never linger: another contender would
            // see those nodes as locked by an owner that has already given up.
            self.roll_back(&acquired, lease.duration).await;

            let Some(delay) = retry_delay(deadline, self.config.retry_interval) else {
                return Ok(None);
            };

            tokio::time::sleep(delay).await;
        }
    }

    /// Release every node acquired by an attempt that did not reach a quorum.
    async fn roll_back(&self, acquired: &[usize], lease: Duration) {
        let timeout = self.config.command_timeout;
        let mut futures = Vec::with_capacity(acquired.len());

        for index in acquired {
            let Some(node) = self.nodes.get(*index).cloned() else {
                continue;
            };
            let Ok(request) = crate::lock::protocol::commands::unlock(
                &self.name,
                self.owner.as_str(),
                lease,
                node.publish_command(),
            ) else {
                continue;
            };

            futures.push(async move {
                let _ = node.eval(&request, timeout).await;
            });
        }

        join_all(futures).await;
    }

    /// Build the guard for an attempt that reached the quorum.
    fn grant(&self, lease: Lease, acquired: Vec<usize>) -> RedLockGuard {
        let mut watchdogs = Vec::new();
        let mut leases = Vec::new();

        for index in &acquired {
            let node = NodeLease::new(*index, lease);
            if lease.watchdog {
                if let Some(backend) = self.nodes.get(*index) {
                    watchdogs.push(RenewalTask::spawn(
                        backend.clone(),
                        self.name.clone(),
                        Arc::new(self.owner.clone()),
                        lease,
                        node.lost.clone(),
                        self.config.command_timeout,
                    ));
                }
            }
            leases.push(node);
        }

        RedLockGuard::new(
            GuardCore::new(self.name.clone(), self.owner.clone(), self.quorum, leases),
            self.nodes.clone(),
            watchdogs,
            self.config.command_timeout,
        )
    }

    /// Count a quorum of successful deletions.
    fn count_quorum(
        &self,
        replies: impl Iterator<Item = ScriptReply>,
    ) -> Result<bool, RedisLockError> {
        let deleted = replies.filter(|reply| reply.int() == Some(1)).count();

        if deleted >= self.quorum {
            Ok(true)
        } else if deleted == 0 {
            Ok(false)
        } else {
            Err(RedisLockError::QuorumUnavailable {
                required: self.quorum,
                acquired: deleted,
            })
        }
    }
}

/// A blocking quorum lock.
#[derive(Clone)]
pub struct BlockingRedLock {
    nodes: Arc<[Arc<dyn BlockingLockBackend>]>,
    quorum: usize,
    name: Arc<str>,
    owner: OwnerToken,
    config: Arc<RedisLockConfig>,
}

impl BlockingRedLock {
    /// Build a blocking quorum lock over `nodes`.
    ///
    /// # Errors
    ///
    /// [`RedisLockError::InvalidConfiguration`] when fewer than three nodes are
    /// given.
    pub fn new(
        nodes: Vec<Arc<dyn BlockingLockBackend>>,
        name: impl Into<String>,
        owner: OwnerToken,
        config: Arc<RedisLockConfig>,
    ) -> Result<Self, RedisLockError> {
        if nodes.len() < MIN_NODES {
            return Err(RedisLockError::InvalidConfiguration(format!(
                "a quorum lock needs at least {MIN_NODES} independent Redis URLs, got {}",
                nodes.len()
            )));
        }

        let quorum = quorum_for(nodes.len());

        Ok(Self {
            nodes: nodes.into(),
            quorum,
            name: Arc::from(name.into()),
            owner,
            config,
        })
    }

    /// The lock name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The owner token every node is acquired under.
    pub fn owner_id(&self) -> &str {
        self.owner.as_str()
    }

    /// How many nodes must agree.
    pub fn quorum(&self) -> usize {
        self.quorum
    }

    /// How many nodes are configured.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Acquire a majority, waiting for as long as it takes.
    ///
    /// # Errors
    ///
    /// Returns the backend error when no attempt could be evaluated at all.
    pub fn lock(&self) -> Result<BlockingRedLockGuard, RedisLockError> {
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

    /// Try once, without waiting.
    ///
    /// # Errors
    ///
    /// Returns the backend error when no attempt could be evaluated at all.
    pub fn try_lock(&self) -> Result<Option<BlockingRedLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::Once,
            Lease::watchdog(self.config.watchdog_timeout),
        )
    }

    /// Retry until `wait` elapses.
    ///
    /// # Errors
    ///
    /// Returns the backend error when no attempt could be evaluated at all.
    pub fn try_lock_for(
        &self,
        wait: Duration,
    ) -> Result<Option<BlockingRedLockGuard>, RedisLockError> {
        self.acquire_with(
            Deadline::after(wait),
            Lease::watchdog(self.config.watchdog_timeout),
        )
    }

    /// One acquisition attempt across every node.
    fn acquire_with(
        &self,
        deadline: Deadline,
        lease: Lease,
    ) -> Result<Option<BlockingRedLockGuard>, RedisLockError> {
        self.config.validate()?;
        if lease.duration.is_zero() {
            return Err(RedisLockError::InvalidDuration { field: "lease" });
        }

        loop {
            let started = Instant::now();
            let request = acquire(&self.name, self.owner.as_str(), lease.duration)?;

            let mut acquired = Vec::new();
            for (index, node) in self.nodes.iter().enumerate() {
                if matches!(
                    node.eval(&request, self.config.command_timeout),
                    Ok(ScriptReply::Nil)
                ) {
                    acquired.push(index);
                }
            }

            let valid = started
                .elapsed()
                .saturating_add(validity_drift(lease.duration))
                < lease.duration;

            if acquired.len() >= self.quorum && valid {
                return Ok(Some(self.grant(lease, acquired)));
            }

            self.roll_back(&acquired, lease.duration);

            let Some(delay) = retry_delay(deadline, self.config.retry_interval) else {
                return Ok(None);
            };

            std::thread::sleep(delay);
        }
    }

    /// Release every node acquired by an attempt that did not reach a quorum.
    fn roll_back(&self, acquired: &[usize], lease: Duration) {
        for index in acquired {
            let Some(node) = self.nodes.get(*index) else {
                continue;
            };
            if let Ok(request) = crate::lock::protocol::commands::unlock(
                &self.name,
                self.owner.as_str(),
                lease,
                node.publish_command(),
            ) {
                let _ = node.eval(&request, self.config.command_timeout);
            }
        }
    }

    /// Build the guard for an attempt that reached the quorum.
    fn grant(&self, lease: Lease, acquired: Vec<usize>) -> BlockingRedLockGuard {
        let mut renewals = Vec::new();
        let mut leases = Vec::new();

        for index in &acquired {
            let node = NodeLease::new(*index, lease);
            if lease.watchdog {
                if let Some(backend) = self.nodes.get(*index) {
                    renewals.push(RenewalThread::spawn(
                        backend.clone(),
                        self.name.clone(),
                        Arc::new(self.owner.clone()),
                        lease,
                        node.lost.clone(),
                        self.config.command_timeout,
                    ));
                }
            }
            leases.push(node);
        }

        BlockingRedLockGuard::new(
            GuardCore::new(self.name.clone(), self.owner.clone(), self.quorum, leases),
            self.nodes.clone(),
            renewals,
            self.config.command_timeout,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{backend::memory::MemoryLockBackend, test_support::new_owner_token};

    fn config() -> Arc<RedisLockConfig> {
        Arc::new(RedisLockConfig::default())
    }

    #[test]
    fn quorum_is_a_strict_majority() {
        assert_eq!(quorum_for(3), 2);
        assert_eq!(quorum_for(4), 3);
        assert_eq!(quorum_for(5), 3);
    }

    #[test]
    fn fewer_than_three_nodes_is_rejected() {
        let nodes: Vec<Arc<dyn LockBackend>> = vec![MemoryLockBackend::new_shared()];
        assert!(matches!(
            RedLock::new(nodes, "orders", new_owner_token(), config()),
            Err(RedisLockError::InvalidConfiguration(_))
        ));

        let blocking: Vec<Arc<dyn BlockingLockBackend>> = vec![MemoryLockBackend::new_shared()];
        assert!(matches!(
            BlockingRedLock::new(blocking, "orders", new_owner_token(), config()),
            Err(RedisLockError::InvalidConfiguration(_))
        ));
    }

    #[tokio::test]
    async fn a_single_unavailable_node_still_reaches_the_quorum() {
        let healthy_a = MemoryLockBackend::new_shared();
        let healthy_b = MemoryLockBackend::new_shared();
        let broken = MemoryLockBackend::new_shared();
        broken.set_unavailable(true);

        let nodes: Vec<Arc<dyn LockBackend>> = vec![healthy_a, healthy_b, broken];
        let lock = RedLock::new(nodes, "orders", new_owner_token(), config()).unwrap();

        let guard = lock.try_lock().await.unwrap().unwrap();
        assert_eq!(guard.node_count(), 2);
        assert_eq!(guard.quorum(), 2);

        guard.unlock().await.unwrap();
    }

    #[tokio::test]
    async fn two_unavailable_nodes_fail_and_roll_the_third_back() {
        let healthy = MemoryLockBackend::new_shared();
        let broken_a = MemoryLockBackend::new_shared();
        let broken_b = MemoryLockBackend::new_shared();
        broken_a.set_unavailable(true);
        broken_b.set_unavailable(true);

        let nodes: Vec<Arc<dyn LockBackend>> = vec![healthy.clone(), broken_a, broken_b];
        let lock = RedLock::new(nodes, "orders", new_owner_token(), config()).unwrap();

        assert!(lock.try_lock().await.unwrap().is_none());
        // The reachable node must not keep the lock after a failed attempt.
        assert!(!healthy.exists("orders"));
    }

    #[test]
    fn a_blocking_red_lock_reaches_the_quorum() {
        let nodes: Vec<Arc<dyn BlockingLockBackend>> = vec![
            MemoryLockBackend::new_shared(),
            MemoryLockBackend::new_shared(),
            MemoryLockBackend::new_shared(),
        ];
        let lock = BlockingRedLock::new(nodes, "orders", new_owner_token(), config()).unwrap();

        let guard = lock.try_lock().unwrap().unwrap();
        assert_eq!(guard.node_count(), 3);
        assert_eq!(
            guard.unlock().unwrap(),
            crate::lock::error::UnlockOutcome::Released
        );
    }

    #[tokio::test]
    async fn a_second_owner_cannot_reach_the_quorum() {
        let backend_a = MemoryLockBackend::new_shared();
        let backend_b = MemoryLockBackend::new_shared();
        let backend_c = MemoryLockBackend::new_shared();

        let nodes: Vec<Arc<dyn LockBackend>> =
            vec![backend_a.clone(), backend_b.clone(), backend_c.clone()];
        let first = RedLock::new(nodes, "orders", new_owner_token(), config()).unwrap();
        let guard = first.try_lock().await.unwrap().unwrap();

        let nodes: Vec<Arc<dyn LockBackend>> = vec![backend_a, backend_b, backend_c];
        let second = RedLock::new(nodes, "orders", new_owner_token(), config()).unwrap();
        assert!(second.try_lock().await.unwrap().is_none());

        guard.unlock().await.unwrap();
        assert!(second.try_lock().await.unwrap().is_some());
    }
}
