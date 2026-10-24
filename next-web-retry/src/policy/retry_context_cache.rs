//! Stores retry contexts between the calls of a stateful retry.

use std::sync::Arc;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

/// The default capacity of an in-memory retry context cache.
///
/// The limit is deliberately low: a full cache almost always means the keys are unstable,
/// so failing early surfaces the bug instead of leaking memory.
pub const DEFAULT_CAPACITY: usize = 4096;

/// Stores retry contexts between the calls of a stateful retry.
///
/// A stateful retry does not loop: it fails, stores its context under the item key, and
/// returns. The next call for the item recovers the context so the attempt count and the
/// last error continue where they left off.
///
/// A `None` key means "no information": implementations are free to discard the context
/// instead of storing it. Implementations must be safe to share between tasks.
pub trait RetryContextCache
where
    Self: Send + Sync,
{
    /// Returns the stored context for `key`, if it is still alive.
    fn get(&self, key: &str) -> Option<Arc<dyn RetryContext>>;

    /// Stores a context under `key`.
    ///
    /// Returns [`RetryError::RetryCacheCapacityExceededError`] when the cache is configured
    /// to reject new entries at capacity.
    fn put(&self, key: &str, context: Arc<dyn RetryContext>) -> Result<(), RetryError>;

    /// Removes the context stored under `key`.
    fn remove(&self, key: &str);

    /// Returns whether a live context is stored under `key`.
    fn contains_key(&self, key: &str) -> bool;
}
