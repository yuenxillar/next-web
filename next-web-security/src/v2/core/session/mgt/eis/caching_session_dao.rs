//! Shiro's `CachingSessionDAO`: a session store in front of another one.
//!
//! A session is read on every request that carries its identifier, and a store
//! that is not in memory — a database, a shared cache — would be asked on every
//! one of them. This store keeps the sessions it read in memory and only asks the
//! store behind it for the ones it does not hold.

use std::sync::Arc;

use dashmap::DashMap;
use next_web_core::async_trait;
use tracing::trace;

use crate::core::session::{mgt::eis::session_dao::SessionDAO, Session, SessionId};

/// A session store that caches the sessions of another store.
#[derive(Clone)]
pub struct CachingSessionDAO {
    delegate: Arc<dyn SessionDAO>,
    active_sessions: Arc<DashMap<String, Arc<dyn Session>>>,
    cache_enabled: bool,
}

impl CachingSessionDAO {
    /// Creates a store in front of the given store.
    ///
    /// # Arguments
    ///
    /// * `delegate` - The store the sessions are read from and written to.
    pub fn new(delegate: Arc<dyn SessionDAO>) -> Self {
        Self {
            delegate,
            active_sessions: Arc::new(DashMap::new()),
            cache_enabled: true,
        }
    }

    /// Sets whether the store caches the sessions it read.
    ///
    /// # Arguments
    ///
    /// * `cache_enabled` - Whether a read is served from memory.
    pub fn with_cache_enabled(mut self, cache_enabled: bool) -> Self {
        self.cache_enabled = cache_enabled;
        self
    }

    /// Returns the store behind this one.
    pub fn delegate(&self) -> &Arc<dyn SessionDAO> {
        &self.delegate
    }

    /// Returns whether the store caches the sessions it read.
    pub fn is_cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    /// Returns the number of sessions the cache holds.
    pub fn cached_sessions(&self) -> usize {
        self.active_sessions.len()
    }

    /// Drops the session of the given identifier from the cache.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the session.
    pub fn uncache(&self, session_id: &SessionId) {
        self.active_sessions.remove(&session_id.to_string());
    }

    /// Forgets every cached session.
    pub fn clear_cache(&self) {
        self.active_sessions.clear();
    }
}

#[async_trait]
impl SessionDAO for CachingSessionDAO {
    async fn create(&self, session: Arc<dyn Session>) {
        self.delegate.create(session.clone()).await;
        self.active_sessions
            .insert(session.id().to_string(), session);
    }

    async fn read(&self, session_id: &SessionId) -> Option<Arc<dyn Session>> {
        let key = session_id.to_string();

        if self.cache_enabled {
            if let Some(session) = self.active_sessions.get(&key) {
                trace!("Serving the session [{}] from the cache.", session_id);

                return Some(session.value().clone());
            }
        }

        let session = self.delegate.read(session_id).await?;
        self.active_sessions.insert(key, session.clone());

        Some(session)
    }

    async fn update(&self, session: Arc<dyn Session>) -> Result<(), &'static str> {
        self.delegate.update(session.clone()).await?;
        self.active_sessions
            .insert(session.id().to_string(), session);

        Ok(())
    }

    async fn delete(&self, session: &dyn Session) {
        self.uncache(session.id());
        self.delegate.delete(session).await;
    }

    async fn get_active_sessions(&self) -> Vec<Arc<dyn Session>> {
        self.active_sessions
            .iter()
            .map(|session| session.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use next_web_core::async_trait;

    use super::CachingSessionDAO;
    use crate::core::session::{
        mgt::{
            eis::{memory_session_dao::MemorySessionDAO, session_dao::SessionDAO},
            simple_session::SimpleSession,
        },
        Session, SessionId,
    };

    /// A store that counts the reads it served.
    struct CountingSessionDAO {
        delegate: MemorySessionDAO,
        reads: AtomicUsize,
    }

    #[async_trait]
    impl SessionDAO for CountingSessionDAO {
        async fn create(&self, session: Arc<dyn Session>) {
            self.delegate.create(session).await;
        }

        async fn read(&self, session_id: &SessionId) -> Option<Arc<dyn Session>> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.delegate.read(session_id).await
        }

        async fn update(&self, session: Arc<dyn Session>) -> Result<(), &'static str> {
            self.delegate.update(session).await
        }

        async fn delete(&self, session: &dyn Session) {
            self.delegate.delete(session).await;
        }

        async fn get_active_sessions(&self) -> Vec<Arc<dyn Session>> {
            self.delegate.get_active_sessions().await
        }
    }

    fn session() -> Arc<dyn Session> {
        Arc::new(SimpleSession::new("example.org"))
    }

    #[tokio::test]
    async fn a_session_is_read_from_the_store_behind_it_once() {
        let delegate = Arc::new(CountingSessionDAO {
            delegate: MemorySessionDAO::default(),
            reads: AtomicUsize::new(0),
        });
        let caching = CachingSessionDAO::new(delegate.clone());
        let session = session();
        caching.create(session.clone()).await;

        assert!(caching.read(session.id()).await.is_some());
        assert!(caching.read(session.id()).await.is_some());
        assert!(caching.read(session.id()).await.is_some());

        // The store behind it is asked when the session is created, and the cache
        // serves the reads after that.
        assert_eq!(delegate.reads.load(Ordering::SeqCst), 0);
        assert_eq!(caching.cached_sessions(), 1);
    }

    #[tokio::test]
    async fn a_session_that_was_not_cached_is_read_through() {
        let delegate = Arc::new(CountingSessionDAO {
            delegate: MemorySessionDAO::default(),
            reads: AtomicUsize::new(0),
        });
        let session = session();
        delegate.create(session.clone()).await;

        let caching = CachingSessionDAO::new(delegate.clone());
        assert!(caching.read(session.id()).await.is_some());
        assert!(caching.read(session.id()).await.is_some());

        assert_eq!(delegate.reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_deleted_session_is_gone_from_the_cache_and_the_store() {
        let delegate = Arc::new(MemorySessionDAO::default());
        let caching = CachingSessionDAO::new(delegate.clone());
        let session = session();
        caching.create(session.clone()).await;

        caching.delete(session.as_ref()).await;

        assert!(caching.read(session.id()).await.is_none());
        assert_eq!(caching.cached_sessions(), 0);
        assert!(delegate.read(session.id()).await.is_none());
    }

    #[tokio::test]
    async fn a_store_without_a_cache_asks_the_store_behind_it_every_time() {
        let delegate = Arc::new(CountingSessionDAO {
            delegate: MemorySessionDAO::default(),
            reads: AtomicUsize::new(0),
        });
        let caching =
            CachingSessionDAO::new(delegate.clone()).with_cache_enabled(false);
        let session = session();
        delegate.create(session.clone()).await;

        assert!(caching.read(session.id()).await.is_some());
        assert!(caching.read(session.id()).await.is_some());

        assert_eq!(delegate.reads.load(Ordering::SeqCst), 2);
        assert!(!caching.is_cache_enabled());
    }

    #[tokio::test]
    async fn the_cached_sessions_are_the_active_ones() {
        let delegate = Arc::new(MemorySessionDAO::default());
        let caching = CachingSessionDAO::new(delegate);
        let first = session();
        let second = session();
        caching.create(first.clone()).await;
        caching.create(second.clone()).await;

        assert_eq!(caching.get_active_sessions().await.len(), 2);

        caching.clear_cache();
        assert_eq!(caching.get_active_sessions().await.len(), 0);
        assert_eq!(caching.cached_sessions(), 0);
    }
}
