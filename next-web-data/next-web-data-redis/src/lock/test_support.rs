//! Helpers shared by the lock layers' own tests.
//!
//! Only compiled for tests: production code mints owner tokens through
//! [`crate::lock::service::RedisLockService`], never directly.
//!
//! The lock types are named by module path here because the crate root still
//! re-exports the previous implementation until the cutover task replaces it.

use std::sync::Arc;

use crate::lock::{
    async_lock::RedisLock,
    backend::memory::MemoryLockBackend,
    config::RedisLockConfig,
    ownership::{LockOwner, OwnerToken},
};

/// A token no other helper call returns.
pub(crate) fn new_owner_token() -> OwnerToken {
    LockOwner::default().next_token()
}

/// An async lock handle on `name` over the in-memory backend.
pub(crate) fn test_lock(backend: Arc<MemoryLockBackend>, name: &str) -> RedisLock {
    RedisLock::new(
        backend,
        name,
        new_owner_token(),
        Arc::new(RedisLockConfig::default()),
    )
}

/// An async lock handle with a specific configuration.
pub(crate) fn test_lock_with(
    backend: Arc<MemoryLockBackend>,
    name: &str,
    config: RedisLockConfig,
) -> RedisLock {
    RedisLock::new(backend, name, new_owner_token(), Arc::new(config))
}
