//! The factory application code goes through.
//!
//! A service owns the topology and the owner tokens for every handle it hands
//! out, so two handles for the same name contend while clones of one handle
//! re-enter.

use std::sync::Arc;

use crate::lock::{
    async_lock::RedisLock,
    backend::{BlockingLockBackend, LockBackend, cluster::ClusterLockBackend},
    blocking::BlockingRedisLock,
    config::RedisLockConfig,
    error::RedisLockError,
    ownership::LockOwner,
    redlock::{BlockingRedLock, RedLock},
};

/// Creates lock handles over one deployment or a quorum of them.
#[derive(Clone)]
pub struct RedisLockService {
    async_nodes: Arc<[Arc<dyn LockBackend>]>,
    blocking_nodes: Arc<[Arc<dyn BlockingLockBackend>]>,
    config: Arc<RedisLockConfig>,
    owner: LockOwner,
}

impl RedisLockService {
    /// Build a service over `nodes`.
    ///
    /// The first node is the primary used by [`RedisLockService::get_lock`] and
    /// [`RedisLockService::get_blocking_lock`]; all of them are used by the
    /// quorum handles.
    ///
    /// # Errors
    ///
    /// [`RedisLockError::InvalidConfiguration`] when `nodes` is empty, and
    /// [`RedisLockError::InvalidDuration`] for an unusable configuration.
    pub fn new<B>(nodes: Vec<Arc<B>>, config: RedisLockConfig) -> Result<Self, RedisLockError>
    where
        B: LockBackend + BlockingLockBackend + 'static,
    {
        if nodes.is_empty() {
            return Err(RedisLockError::InvalidConfiguration(
                "a lock service needs at least one Redis node".to_string(),
            ));
        }
        config.validate()?;

        let async_nodes: Vec<Arc<dyn LockBackend>> = nodes
            .iter()
            .map(|node| node.clone() as Arc<dyn LockBackend>)
            .collect();
        let blocking_nodes: Vec<Arc<dyn BlockingLockBackend>> = nodes
            .into_iter()
            .map(|node| node as Arc<dyn BlockingLockBackend>)
            .collect();

        Ok(Self {
            async_nodes: async_nodes.into(),
            blocking_nodes: blocking_nodes.into(),
            config: Arc::new(config),
            owner: LockOwner::new(),
        })
    }

    /// Build a service on a standalone [`crate::core::redis_template::RedisTemplate`].
    ///
    /// # Errors
    ///
    /// As [`RedisLockService::new`].
    pub fn from_template(
        template: &crate::core::redis_template::RedisTemplate,
        config: RedisLockConfig,
    ) -> Result<Self, RedisLockError> {
        Self::new(vec![Arc::new(template.clone())], config)
    }

    /// Build a service on a Redis Cluster deployment.
    ///
    /// # Errors
    ///
    /// As [`RedisLockService::new`], plus whatever the cluster client rejects.
    pub fn from_cluster(
        urls: Vec<String>,
        config: RedisLockConfig,
    ) -> Result<Self, RedisLockError> {
        let command_timeout = config.command_timeout;
        let backend = ClusterLockBackend::new(urls, command_timeout)?;

        Self::new(vec![Arc::new(backend)], config)
    }

    /// An asynchronous handle for `name`. One call, one owner token.
    pub fn get_lock(&self, name: impl Into<String>) -> RedisLock {
        RedisLock::new(
            self.primary_async(),
            name,
            self.owner.next_token(),
            self.config.clone(),
        )
    }

    /// A blocking handle for `name`. One call, one owner token.
    pub fn get_blocking_lock(&self, name: impl Into<String>) -> BlockingRedisLock {
        BlockingRedisLock::new(
            self.primary_blocking(),
            name,
            self.owner.next_token(),
            self.config.clone(),
        )
    }

    /// An asynchronous quorum handle for `name`.
    ///
    /// # Errors
    ///
    /// [`RedisLockError::InvalidConfiguration`] when the service has fewer than
    /// three nodes.
    pub fn get_red_lock(&self, name: impl Into<String>) -> Result<RedLock, RedisLockError> {
        RedLock::new(
            self.async_nodes.to_vec(),
            name,
            self.owner.next_token(),
            self.config.clone(),
        )
    }

    /// A blocking quorum handle for `name`.
    ///
    /// # Errors
    ///
    /// [`RedisLockError::InvalidConfiguration`] when the service has fewer than
    /// three nodes.
    pub fn get_blocking_red_lock(
        &self,
        name: impl Into<String>,
    ) -> Result<BlockingRedLock, RedisLockError> {
        BlockingRedLock::new(
            self.blocking_nodes.to_vec(),
            name,
            self.owner.next_token(),
            self.config.clone(),
        )
    }

    /// The configuration every handle shares.
    pub fn config(&self) -> &RedisLockConfig {
        &self.config
    }

    /// How many nodes the service was built with.
    pub fn node_count(&self) -> usize {
        self.async_nodes.len()
    }

    /// The client id every owner token from this service starts with.
    pub fn client_id(&self) -> &str {
        self.owner.client_id()
    }

    fn primary_async(&self) -> Arc<dyn LockBackend> {
        // `new` rejects an empty node list, so the first node always exists.
        self.async_nodes
            .first()
            .cloned()
            .unwrap_or_else(|| unreachable!("a lock service always has a primary node"))
    }

    fn primary_blocking(&self) -> Arc<dyn BlockingLockBackend> {
        self.blocking_nodes
            .first()
            .cloned()
            .unwrap_or_else(|| unreachable!("a lock service always has a primary node"))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::lock::{backend::memory::MemoryLockBackend, error::UnlockOutcome};

    fn service() -> RedisLockService {
        RedisLockService::new(
            vec![MemoryLockBackend::new_shared()],
            RedisLockConfig::default(),
        )
        .unwrap()
    }

    #[test]
    fn one_service_mints_a_fresh_owner_token_per_handle() {
        let service = service();
        let first = service.get_lock("orders");
        let second = service.get_lock("orders");

        // Two handles on one name contend by design: each mints its own token.
        assert_ne!(first.owner_id(), second.owner_id());
        // A clone keeps the token, which is what makes it reentrant.
        assert_eq!(first.owner_id(), first.clone().owner_id());
        assert!(first.owner_id().starts_with(service.client_id()));
    }

    #[tokio::test]
    async fn a_single_node_service_offers_async_and_blocking_handles() {
        let service = service();
        assert_eq!(service.node_count(), 1);

        let blocking = service.get_blocking_lock("orders");
        let guard = tokio::task::spawn_blocking(move || blocking.try_lock())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            service
                .get_lock("orders")
                .try_lock()
                .await
                .unwrap()
                .is_none()
        );

        let released = tokio::task::spawn_blocking(move || guard.unlock())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(released, UnlockOutcome::Released);
        assert!(
            service
                .get_lock("orders")
                .try_lock()
                .await
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn a_quorum_handle_needs_three_nodes() {
        assert!(matches!(
            service().get_red_lock("orders"),
            Err(RedisLockError::InvalidConfiguration(_))
        ));
        assert!(matches!(
            service().get_blocking_red_lock("orders"),
            Err(RedisLockError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn three_nodes_offer_quorum_handles() {
        let service = RedisLockService::new(
            vec![
                MemoryLockBackend::new_shared(),
                MemoryLockBackend::new_shared(),
                MemoryLockBackend::new_shared(),
            ],
            RedisLockConfig::default(),
        )
        .unwrap();

        assert_eq!(service.node_count(), 3);
        assert_eq!(service.get_red_lock("orders").unwrap().quorum(), 2);
        assert_eq!(service.get_blocking_red_lock("orders").unwrap().quorum(), 2);
    }

    #[test]
    fn an_invalid_configuration_is_rejected_at_construction() {
        let config = RedisLockConfig {
            command_timeout: Duration::ZERO,
            ..RedisLockConfig::default()
        };
        assert!(matches!(
            RedisLockService::new(vec![MemoryLockBackend::new_shared()], config),
            Err(RedisLockError::InvalidDuration {
                field: "command_timeout"
            })
        ));
    }

    #[test]
    fn an_empty_node_list_is_rejected() {
        let nodes: Vec<Arc<MemoryLockBackend>> = Vec::new();
        assert!(matches!(
            RedisLockService::new(nodes, RedisLockConfig::default()),
            Err(RedisLockError::InvalidConfiguration(_))
        ));
    }
}
