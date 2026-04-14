use std::ops::Deref;

use next_web_core::traits::service::Service;
use rslock::LockManager;

/// Distributed lock service backed by Redis.
///
/// This service is only available when the `lock` feature is enabled.
#[derive(Clone)]
pub struct RedisLockService {
    /// Internal Redlock-compatible manager.
    lock: LockManager,
}

impl RedisLockService {
    /// Create a lock service from one or more Redis URLs.
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            lock: LockManager::new(urls),
        }
    }
}

impl Service for RedisLockService {}

impl Deref for RedisLockService {
    type Target = LockManager;

    /// Expose the underlying lock manager.
    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}
