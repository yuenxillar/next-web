//! Redisson 4.7 compatible reentrant distributed locks (`RedissonLock`).
//!
//! This is the concrete leaf orchestration layer; naming, expiration,
//! ownership, Pub/Sub and renewal contracts are defined by sibling modules.

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use futures::{StreamExt, future::join_all};
use next_web_core::traits::{service::Service, singleton::Singleton};
use redis::{Cmd, FromRedisValue, RedisError, cmd};
use serde::Deserialize;
use tokio::sync::watch;
use tracing::warn;
use uuid::Uuid;

use super::lock_scripts::{ACQUIRE_SCRIPT, FORCE_UNLOCK_SCRIPT, RENEW_SCRIPT, UNLOCK_SCRIPT};

/// Errors returned by distributed lock operations.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("Redis operation failed: {0}")]
    Redis(#[from] RedisError),
    #[error("invalid distributed lock configuration: {0}")]
    InvalidConfiguration(String),
    #[error("invalid lock duration: {0}")]
    InvalidDuration(String),
    #[error("the lock isn't held by this owner")]
    NotOwner,
    #[error("the lock watchdog lost ownership")]
    WatchdogLost,
    #[error("lock quorum unavailable: required {required}, acquired {acquired}")]
    QuorumUnavailable { required: usize, acquired: usize },
    #[error("distributed lock protocol error: {0}")]
    Protocol(String),
    #[error("lock release was only partially confirmed; {failures} node(s) failed")]
    PartialUnlock { failures: usize },
}

/// Backend used by the standard lock returned from the service.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedisLockMode {
    #[default]
    Direct,
    Cluster,
}

/// Runtime configuration for distributed locks.
#[derive(Debug, Clone)]
pub struct RedisLockConfig {
    pub mode: RedisLockMode,
    pub watchdog_timeout: Duration,
    pub retry_interval: Duration,
    pub command_timeout: Duration,
    pub cluster_urls: Vec<String>,
    pub redlock_urls: Vec<String>,
}

impl Default for RedisLockConfig {
    fn default() -> Self {
        Self {
            mode: RedisLockMode::Direct,
            watchdog_timeout: Duration::from_secs(30),
            retry_interval: Duration::from_millis(100),
            command_timeout: Duration::from_secs(3),
            cluster_urls: Vec::new(),
            redlock_urls: Vec::new(),
        }
    }
}

impl RedisLockConfig {
    fn validate(&self) -> Result<(), LockError> {
        for (name, value) in [
            ("watchdog-timeout", self.watchdog_timeout),
            ("retry-interval", self.retry_interval),
            ("command-timeout", self.command_timeout),
        ] {
            if value.is_zero() || value.as_millis() > u64::MAX as u128 {
                return Err(LockError::InvalidDuration(name.to_string()));
            }
        }
        if self.mode == RedisLockMode::Cluster && self.cluster_urls.is_empty() {
            return Err(LockError::InvalidConfiguration(
                "cluster mode requires at least one cluster URL".to_string(),
            ));
        }
        validate_redlock_urls(&self.redlock_urls)
    }
}

#[derive(Clone)]
enum RedisBackend {
    Direct(Arc<redis::Client>),
    Cluster(Arc<redis::cluster::ClusterClient>),
}

impl RedisBackend {
    fn direct_url(url: &str) -> Result<Self, LockError> {
        Ok(Self::Direct(Arc::new(redis::Client::open(url)?)))
    }

    fn cluster(urls: Vec<String>) -> Result<Self, LockError> {
        let client = redis::cluster::ClusterClient::new(urls)?;
        Ok(Self::Cluster(Arc::new(client)))
    }

    fn publish_command(&self) -> &'static str {
        match self {
            Self::Direct(_) => "PUBLISH",
            Self::Cluster(_) => "SPUBLISH",
        }
    }

    async fn execute<T>(&self, mut command: Cmd, timeout: Duration) -> Result<T, LockError>
    where
        T: FromRedisValue + Send,
    {
        let operation = async {
            match self {
                Self::Direct(client) => {
                    let mut connection = client.get_multiplexed_async_connection().await?;
                    command.query_async(&mut connection).await
                }
                Self::Cluster(client) => {
                    let mut connection = client.get_async_connection().await?;
                    command.query_async(&mut connection).await
                }
            }
        };
        tokio::time::timeout(timeout, operation)
            .await
            .map_err(|_| LockError::Protocol("Redis command timed out".to_string()))?
            .map_err(LockError::Redis)
    }

    async fn wait_for_unlock(
        &self,
        name: &str,
        channel: &str,
        timeout: Duration,
        command_timeout: Duration,
    ) -> Result<(), LockError> {
        match self {
            Self::Direct(client) => {
                let mut pubsub = client.get_async_pubsub().await?;
                pubsub.subscribe(channel).await?;
                let mut exists = cmd("EXISTS");
                exists.arg(name);
                if !self.execute::<bool>(exists, command_timeout).await? {
                    return Ok(());
                }
                let mut messages = pubsub.on_message();
                let _ = tokio::time::timeout(timeout, messages.next()).await;
                Ok(())
            }
            // RESP2 cluster subscriptions require node-addressed connection ownership.
            // Bounded PTTL polling remains correct and works for both RESP2 and RESP3.
            Self::Cluster(_) => {
                tokio::time::sleep(timeout).await;
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
struct LockTarget {
    name: Arc<str>,
    owner: Arc<str>,
    nodes: Arc<[RedisBackend]>,
    quorum: usize,
    config: Arc<RedisLockConfig>,
    local_hold_count: AtomicU32,
}

impl LockTarget {
    fn channel_name(&self) -> String {
        channel_name(&self.name)
    }

    async fn acquire(
        self: &Arc<Self>,
        wait: Option<Duration>,
        lease: Option<Duration>,
    ) -> Result<Option<LockGuard>, LockError> {
        if self.name.trim().is_empty() {
            return Err(LockError::InvalidConfiguration(
                "lock name cannot be empty".to_string(),
            ));
        }
        if let Some(value) = lease {
            validate_duration("lease", value)?;
        }
        if let Some(value) = wait {
            validate_duration("wait", value)?;
        }

        let deadline = wait.and_then(|value| Instant::now().checked_add(value));
        loop {
            let attempt_started = Instant::now();
            let lease_time = lease.unwrap_or(self.config.watchdog_timeout);
            let lease_ms = duration_millis(lease_time, "lease")?;
            let attempts = self.nodes.iter().map(|node| {
                try_acquire_node(
                    node,
                    &self.name,
                    &self.owner,
                    lease_ms,
                    self.config.command_timeout,
                )
            });
            let results = join_all(attempts).await;

            let mut acquired = Vec::new();
            let mut ttls = Vec::new();
            let mut first_error = None;
            for (index, result) in results.into_iter().enumerate() {
                match result {
                    Ok(None) => acquired.push(index),
                    Ok(Some(ttl)) => ttls.push(ttl),
                    Err(error) if first_error.is_none() => first_error = Some(error),
                    Err(_) => {}
                }
            }

            let drift = if self.quorum > 1 {
                Duration::from_millis((lease_ms / 100).saturating_add(2))
            } else {
                Duration::ZERO
            };
            let validity_ok = attempt_started.elapsed().saturating_add(drift) < lease_time;
            if acquired.len() >= self.quorum && validity_ok {
                let hold_count = self.local_hold_count.fetch_add(1, Ordering::AcqRel) + 1;
                let leases = acquired
                    .into_iter()
                    .map(|index| NodeLease::new(self.clone(), index, lease_time, lease.is_none()))
                    .collect();
                return Ok(Some(LockGuard {
                    target: self.clone(),
                    leases,
                    hold_count,
                    unlocked: false,
                }));
            }

            if !acquired.is_empty() {
                rollback_nodes(self, &acquired, lease_ms).await;
            }
            if let Some(error) = first_error {
                return Err(error);
            }
            if wait.is_none() {
                return Ok(None);
            }
            let Some(deadline) = deadline else {
                return Ok(None);
            };
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let remaining = deadline.saturating_duration_since(now);
            let redis_ttl = ttls
                .into_iter()
                .filter(|ttl| *ttl > 0)
                .min()
                .map(|ttl| Duration::from_millis(ttl as u64));
            let mut delay = remaining;
            if let Some(ttl) = redis_ttl {
                delay = delay.min(ttl);
            }
            if matches!(&self.nodes[0], RedisBackend::Cluster(_)) || self.quorum > 1 {
                delay = delay.min(self.config.retry_interval);
            }
            let delay = delay.max(Duration::from_millis(1));
            if self.quorum == 1 {
                self.nodes[0]
                    .wait_for_unlock(
                        &self.name,
                        &self.channel_name(),
                        delay,
                        self.config.command_timeout,
                    )
                    .await?;
            } else {
                tokio::time::sleep(delay).await;
            }
        }
    }

    async fn status_values<T, F, Fut>(&self, operation: F) -> Result<Vec<T>, LockError>
    where
        F: Fn(RedisBackend) -> Fut,
        Fut: Future<Output = Result<T, LockError>>,
    {
        let results = join_all(self.nodes.iter().cloned().map(operation)).await;
        let mut values = Vec::new();
        let mut first_error = None;
        for result in results {
            match result {
                Ok(value) => values.push(value),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if values.len() < self.quorum {
            return Err(first_error.unwrap_or(LockError::QuorumUnavailable {
                required: self.quorum,
                acquired: values.len(),
            }));
        }
        Ok(values)
    }
}

/// Application service used to create direct, cluster, and quorum locks.
#[derive(Clone)]
pub struct RedisDistributedLockService {
    primary: RedisBackend,
    redlock_nodes: Arc<[RedisBackend]>,
    config: Arc<RedisLockConfig>,
    client_id: Arc<str>,
    owner_sequence: Arc<AtomicU64>,
}

impl RedisDistributedLockService {
    pub fn with_direct(url: impl AsRef<str>, config: RedisLockConfig) -> Result<Self, LockError> {
        let client = Arc::new(redis::Client::open(url.as_ref())?);
        Self::with_direct_client(client, config)
    }

    pub fn with_direct_client(
        client: Arc<redis::Client>,
        config: RedisLockConfig,
    ) -> Result<Self, LockError> {
        config.validate()?;
        let primary = if config.mode == RedisLockMode::Cluster {
            RedisBackend::cluster(config.cluster_urls.clone())?
        } else {
            RedisBackend::Direct(client)
        };
        Self::build(primary, config)
    }

    pub fn with_cluster(urls: Vec<String>, mut config: RedisLockConfig) -> Result<Self, LockError> {
        config.mode = RedisLockMode::Cluster;
        config.cluster_urls = urls.clone();
        config.validate()?;
        Self::build(RedisBackend::cluster(urls)?, config)
    }

    fn build(primary: RedisBackend, config: RedisLockConfig) -> Result<Self, LockError> {
        let redlock_nodes = config
            .redlock_urls
            .iter()
            .map(|url| RedisBackend::direct_url(url))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            primary,
            redlock_nodes: redlock_nodes.into(),
            config: Arc::new(config),
            client_id: Uuid::new_v4().to_string().into(),
            owner_sequence: Arc::new(AtomicU64::new(1)),
        })
    }

    pub fn with_redlock_nodes(mut self, urls: Vec<String>) -> Result<Self, LockError> {
        validate_redlock_urls(&urls)?;
        self.redlock_nodes = urls
            .iter()
            .map(|url| RedisBackend::direct_url(url))
            .collect::<Result<Vec<_>, _>>()?
            .into();
        Ok(self)
    }

    pub fn get_lock(&self, name: impl Into<String>) -> RedisLock {
        RedisLock {
            target: self.target(name.into(), vec![self.primary.clone()], 1),
        }
    }

    pub fn get_redlock(&self, name: impl Into<String>) -> Result<RedisRedLock, LockError> {
        if self.redlock_nodes.len() < 3 {
            return Err(LockError::InvalidConfiguration(
                "RedLock requires at least three independent Redis URLs".to_string(),
            ));
        }
        let quorum = self.redlock_nodes.len() / 2 + 1;
        Ok(RedisRedLock {
            target: self.target(name.into(), self.redlock_nodes.to_vec(), quorum),
        })
    }

    fn target(&self, name: String, nodes: Vec<RedisBackend>, quorum: usize) -> Arc<LockTarget> {
        let owner_id = self.owner_sequence.fetch_add(1, Ordering::Relaxed);
        Arc::new(LockTarget {
            name: name.into(),
            owner: format!("{}:{owner_id}", self.client_id).into(),
            nodes: nodes.into(),
            quorum,
            config: self.config.clone(),
            local_hold_count: AtomicU32::new(0),
        })
    }
}

impl Singleton for RedisDistributedLockService {}
impl Service for RedisDistributedLockService {}

/// A Redisson-compatible reentrant lock on one logical Redis deployment.
#[derive(Clone)]
pub struct RedisLock {
    target: Arc<LockTarget>,
}

/// A quorum lock across at least three independent Redis deployments.
#[derive(Clone)]
pub struct RedisRedLock {
    target: Arc<LockTarget>,
}

macro_rules! impl_lock_api {
    ($ty:ty) => {
        impl $ty {
            pub fn name(&self) -> &str {
                &self.target.name
            }
            pub fn owner_id(&self) -> &str {
                &self.target.owner
            }

            pub async fn acquire(&self) -> Result<LockGuard, LockError> {
                loop {
                    if let Some(guard) = self
                        .target
                        .acquire(Some(Duration::from_secs(24 * 60 * 60)), None)
                        .await?
                    {
                        return Ok(guard);
                    }
                }
            }

            pub async fn acquire_with_lease(
                &self,
                lease: Duration,
            ) -> Result<LockGuard, LockError> {
                loop {
                    if let Some(guard) = self
                        .target
                        .acquire(Some(Duration::from_secs(24 * 60 * 60)), Some(lease))
                        .await?
                    {
                        return Ok(guard);
                    }
                }
            }

            pub async fn try_acquire(&self) -> Result<Option<LockGuard>, LockError> {
                self.target.acquire(None, None).await
            }

            pub async fn try_acquire_for(
                &self,
                wait: Duration,
            ) -> Result<Option<LockGuard>, LockError> {
                self.target.acquire(Some(wait), None).await
            }

            pub async fn try_acquire_for_with_lease(
                &self,
                wait: Duration,
                lease: Duration,
            ) -> Result<Option<LockGuard>, LockError> {
                self.target.acquire(Some(wait), Some(lease)).await
            }

            pub async fn is_locked(&self) -> Result<bool, LockError> {
                let name = self.target.name.clone();
                let timeout = self.target.config.command_timeout;
                let values = self
                    .target
                    .status_values(move |node| {
                        let name = name.clone();
                        async move {
                            let mut command = cmd("EXISTS");
                            command.arg(&*name);
                            node.execute::<u64>(command, timeout).await.map(|v| v > 0)
                        }
                    })
                    .await?;
                Ok(values.into_iter().filter(|value| *value).count() >= self.target.quorum)
            }

            pub async fn is_held_by_owner(&self) -> Result<bool, LockError> {
                let name = self.target.name.clone();
                let owner = self.target.owner.clone();
                let timeout = self.target.config.command_timeout;
                let values = self
                    .target
                    .status_values(move |node| {
                        let name = name.clone();
                        let owner = owner.clone();
                        async move {
                            let mut command = cmd("HEXISTS");
                            command.arg(&*name).arg(&*owner);
                            node.execute::<bool>(command, timeout).await
                        }
                    })
                    .await?;
                Ok(values.into_iter().filter(|value| *value).count() >= self.target.quorum)
            }

            pub async fn hold_count(&self) -> Result<u32, LockError> {
                let name = self.target.name.clone();
                let owner = self.target.owner.clone();
                let timeout = self.target.config.command_timeout;
                let mut values = self
                    .target
                    .status_values(move |node| {
                        let name = name.clone();
                        let owner = owner.clone();
                        async move {
                            let mut command = cmd("HGET");
                            command.arg(&*name).arg(&*owner);
                            node.execute::<Option<u32>>(command, timeout)
                                .await
                                .map(|v| v.unwrap_or(0))
                        }
                    })
                    .await?;
                values.sort_unstable_by(|a, b| b.cmp(a));
                Ok(values.get(self.target.quorum - 1).copied().unwrap_or(0))
            }

            pub async fn remaining_ttl(&self) -> Result<Option<Duration>, LockError> {
                let name = self.target.name.clone();
                let timeout = self.target.config.command_timeout;
                let mut values = self
                    .target
                    .status_values(move |node| {
                        let name = name.clone();
                        async move {
                            let mut command = cmd("PTTL");
                            command.arg(&*name);
                            node.execute::<i64>(command, timeout).await
                        }
                    })
                    .await?;
                values.retain(|value| *value >= 0);
                if values.len() < self.target.quorum {
                    return Ok(None);
                }
                values.sort_unstable_by(|a, b| b.cmp(a));
                Ok(values
                    .get(self.target.quorum - 1)
                    .map(|value| Duration::from_millis(*value as u64)))
            }

            pub async fn force_unlock(&self) -> Result<bool, LockError> {
                force_unlock_target(&self.target).await
            }

            pub async fn with_lock<F, Fut, T, E>(
                &self,
                operation: F,
            ) -> Result<Result<T, E>, LockError>
            where
                F: FnOnce() -> Fut,
                Fut: Future<Output = Result<T, E>>,
            {
                let guard = self.acquire().await?;
                let result = operation().await;
                guard.unlock().await?;
                Ok(result)
            }
        }
    };
}

impl_lock_api!(RedisLock);
impl_lock_api!(RedisRedLock);

/// Result of releasing one reentrant acquisition level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockResult {
    Reentrant,
    Released,
}

struct NodeLease {
    node_index: usize,
    lease: Duration,
    cancel: Option<watch::Sender<bool>>,
    lost: Arc<AtomicBool>,
}

impl NodeLease {
    fn new(target: Arc<LockTarget>, node_index: usize, lease: Duration, watchdog: bool) -> Self {
        let lost = Arc::new(AtomicBool::new(false));
        if !watchdog {
            return Self {
                node_index,
                lease,
                cancel: None,
                lost,
            };
        }
        let (cancel, mut cancellation) = watch::channel(false);
        let lost_task = lost.clone();
        tokio::spawn(async move {
            let interval = (lease / 3).max(Duration::from_millis(1));
            loop {
                tokio::select! {
                    changed = cancellation.changed() => {
                        if changed.is_err() || *cancellation.borrow() { break; }
                    }
                    _ = tokio::time::sleep(interval) => {
                        let mut renewed = false;
                        for _ in 0..2 {
                            match renew_node(&target.nodes[node_index], &target.name, &target.owner, lease, target.config.command_timeout).await {
                                Ok(true) => {
                                    renewed = true;
                                    break;
                                }
                                Ok(false) => break,
                                Err(_) => {}
                            }
                        }
                        if !renewed {
                            lost_task.store(true, Ordering::Release);
                            warn!(lock = %target.name, owner = %target.owner, "distributed lock watchdog lost ownership");
                            break;
                        }
                    }
                }
            }
        });
        Self {
            node_index,
            lease,
            cancel: Some(cancel),
            lost,
        }
    }

    fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(true);
        }
    }
}

/// An acquired lock level. It must be released explicitly with [`LockGuard::unlock`].
pub struct LockGuard {
    target: Arc<LockTarget>,
    leases: Vec<NodeLease>,
    hold_count: u32,
    unlocked: bool,
}

impl LockGuard {
    pub fn name(&self) -> &str {
        &self.target.name
    }
    pub fn owner_id(&self) -> &str {
        &self.target.owner
    }
    pub fn hold_count(&self) -> u32 {
        self.hold_count
    }
    pub fn is_lost(&self) -> bool {
        self.leases
            .iter()
            .filter(|lease| !lease.lost.load(Ordering::Acquire))
            .count()
            < self.target.quorum
    }

    pub async fn unlock(mut self) -> Result<UnlockResult, LockError> {
        if self.is_lost() {
            self.stop_watchdogs();
            let lease_ms =
                duration_millis(self.target.config.watchdog_timeout, "watchdog-timeout")?;
            rollback_nodes(
                &self.target,
                &self
                    .leases
                    .iter()
                    .map(|lease| lease.node_index)
                    .collect::<Vec<_>>(),
                lease_ms,
            )
            .await;
            self.decrement_local_hold_count();
            self.unlocked = true;
            return Err(LockError::WatchdogLost);
        }
        let lease_ms = duration_millis(
            self.leases
                .first()
                .map(|lease| lease.lease)
                .unwrap_or(self.target.config.watchdog_timeout),
            "lease",
        )?;
        let futures = self.leases.iter().map(|lease| {
            unlock_node(
                &self.target.nodes[lease.node_index],
                &self.target.name,
                &self.target.owner,
                lease_ms,
                self.target.config.command_timeout,
            )
        });
        let results = join_all(futures).await;
        self.stop_watchdogs();
        self.decrement_local_hold_count();
        self.unlocked = true;

        let failures = results.iter().filter(|result| result.is_err()).count();
        if failures > 0 {
            return Err(LockError::PartialUnlock { failures });
        }
        let values = results.into_iter().collect::<Result<Vec<_>, _>>()?;
        if values.iter().any(|value| value.is_none()) {
            return Err(LockError::NotOwner);
        }
        let fully_released = values.iter().filter(|value| **value == Some(true)).count();
        Ok(if fully_released >= self.target.quorum {
            UnlockResult::Released
        } else {
            UnlockResult::Reentrant
        })
    }

    fn stop_watchdogs(&mut self) {
        self.leases.iter_mut().for_each(NodeLease::cancel);
    }

    fn decrement_local_hold_count(&self) {
        let _ = self.target.local_hold_count.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |value| Some(value.saturating_sub(1)),
        );
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if !self.unlocked {
            self.stop_watchdogs();
            self.decrement_local_hold_count();
            warn!(lock = %self.target.name, owner = %self.target.owner, "lock guard dropped without explicit unlock; TTL will release it");
        }
    }
}

async fn try_acquire_node(
    node: &RedisBackend,
    name: &str,
    owner: &str,
    lease_ms: u64,
    timeout: Duration,
) -> Result<Option<i64>, LockError> {
    let mut command = cmd("EVAL");
    command
        .arg(ACQUIRE_SCRIPT)
        .arg(1)
        .arg(name)
        .arg(lease_ms)
        .arg(owner);
    node.execute(command, timeout).await
}

async fn renew_node(
    node: &RedisBackend,
    name: &str,
    owner: &str,
    lease: Duration,
    timeout: Duration,
) -> Result<bool, LockError> {
    let mut command = cmd("EVAL");
    command
        .arg(RENEW_SCRIPT)
        .arg(1)
        .arg(name)
        .arg(duration_millis(lease, "lease")?)
        .arg(owner);
    node.execute(command, timeout).await
}

async fn unlock_node(
    node: &RedisBackend,
    name: &str,
    owner: &str,
    lease_ms: u64,
    timeout: Duration,
) -> Result<Option<bool>, LockError> {
    let request_id = Uuid::new_v4().to_string();
    let latch = unlock_latch_name(name, &request_id);
    let latch_ttl = duration_millis(timeout, "command-timeout")?;
    let mut last_error = None;
    let mut result = None;
    for _ in 0..2 {
        let mut command = cmd("EVAL");
        command
            .arg(UNLOCK_SCRIPT)
            .arg(3)
            .arg(name)
            .arg(channel_name(name))
            .arg(&latch)
            .arg(0)
            .arg(lease_ms)
            .arg(owner)
            .arg(node.publish_command())
            .arg(latch_ttl);
        match node.execute::<Option<i64>>(command, timeout).await {
            Ok(value) => {
                result = Some(value);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let result = result.ok_or_else(|| {
        last_error.unwrap_or_else(|| LockError::Protocol("unlock produced no result".to_string()))
    })?;
    let mut cleanup = cmd("DEL");
    cleanup.arg(latch);
    let _ = node.execute::<u64>(cleanup, timeout).await;
    Ok(result.map(|value| value == 1))
}

async fn rollback_nodes(target: &LockTarget, indices: &[usize], lease_ms: u64) {
    let futures = indices.iter().map(|index| {
        unlock_node(
            &target.nodes[*index],
            &target.name,
            &target.owner,
            lease_ms,
            target.config.command_timeout,
        )
    });
    let _ = join_all(futures).await;
}

async fn force_unlock_target(target: &LockTarget) -> Result<bool, LockError> {
    let channel = target.channel_name();
    let futures = target.nodes.iter().map(|node| {
        let mut command = cmd("EVAL");
        command
            .arg(FORCE_UNLOCK_SCRIPT)
            .arg(2)
            .arg(&*target.name)
            .arg(&channel)
            .arg(0)
            .arg(node.publish_command());
        node.execute::<bool>(command, target.config.command_timeout)
    });
    let results = join_all(futures).await;
    let successes = results
        .iter()
        .filter(|result| matches!(result, Ok(true)))
        .count();
    if successes >= target.quorum {
        return Ok(true);
    }
    if let Some(error) = results.into_iter().find_map(Result::err) {
        return Err(error);
    }
    Ok(false)
}

fn validate_duration(name: &str, duration: Duration) -> Result<(), LockError> {
    if duration.is_zero() || duration.as_millis() > u64::MAX as u128 {
        Err(LockError::InvalidDuration(name.to_string()))
    } else {
        Ok(())
    }
}

fn duration_millis(duration: Duration, name: &str) -> Result<u64, LockError> {
    validate_duration(name, duration)?;
    Ok(duration.as_millis() as u64)
}

fn validate_redlock_urls(urls: &[String]) -> Result<(), LockError> {
    if urls.is_empty() {
        return Ok(());
    }
    if urls.len() < 3 {
        return Err(LockError::InvalidConfiguration(
            "RedLock requires at least three independent Redis URLs".to_string(),
        ));
    }
    let unique = urls.iter().collect::<std::collections::HashSet<_>>();
    if unique.len() != urls.len() {
        return Err(LockError::InvalidConfiguration(
            "RedLock URLs must be unique".to_string(),
        ));
    }
    Ok(())
}

fn prefixed_name(prefix: &str, name: &str) -> String {
    if name.contains('{') {
        format!("{prefix}:{name}")
    } else {
        format!("{prefix}:{{{name}}}")
    }
}

fn channel_name(name: &str) -> String {
    prefixed_name("redisson_lock__channel", name)
}

fn unlock_latch_name(name: &str, request_id: &str) -> String {
    format!(
        "{}:{request_id}",
        prefixed_name("redisson_unlock_latch", name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redisson_names_preserve_cluster_slot() {
        assert_eq!(channel_name("orders"), "redisson_lock__channel:{orders}");
        assert_eq!(
            unlock_latch_name("orders", "1"),
            "redisson_unlock_latch:{orders}:1"
        );
        assert_eq!(
            channel_name("orders:{42}"),
            "redisson_lock__channel:orders:{42}"
        );
    }

    #[test]
    fn config_rejects_invalid_redlock_nodes() {
        let config = RedisLockConfig {
            redlock_urls: vec!["redis://one".into(), "redis://two".into()],
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(LockError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn defaults_match_redisson_watchdog() {
        let config = RedisLockConfig::default();
        assert_eq!(config.watchdog_timeout, Duration::from_secs(30));
        assert_eq!(config.watchdog_timeout / 3, Duration::from_secs(10));
    }
}
