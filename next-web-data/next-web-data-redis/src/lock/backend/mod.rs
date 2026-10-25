//! The two primitives every lock flavour is built from.
//!
//! A backend can do exactly two things: run one lock script atomically, and
//! wait for a lock to be released. Everything else — reentrancy, watchdog
//! renewal, quorum logic — is composed from those two in the lock layers.

use std::time::Duration;

use next_web_core::async_trait;

use crate::lock::error::RedisLockError;
use crate::lock::protocol::{LockRequest, ScriptReply};

pub mod cluster;

/// Asynchronous transport a lock runs its scripts on.
#[async_trait]
pub trait LockBackend: Send + Sync + 'static {
    /// Run one lock script atomically.
    ///
    /// `timeout` is the budget for this call; the implementation enforces it in
    /// whatever way its own IO model allows.
    async fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError>;

    /// Wait until `name` is no longer held, or `timeout` elapses.
    ///
    /// Callers treat a timeout as "retry now"; the wait is an optimisation, not
    /// a correctness requirement.
    async fn wait_for_unlock(
        &self,
        name: &str,
        channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError>;

    /// The publish command this deployment needs for lock notifications.
    ///
    /// `PUBLISH` on a standalone server, `SPUBLISH` in cluster mode.
    fn publish_command(&self) -> &'static str {
        "PUBLISH"
    }
}

/// Blocking transport with the same two primitives.
///
/// This is a separate trait rather than a `block_on` wrapper over
/// [`LockBackend`]: callers that want a blocking lock use the `redis` crate's
/// synchronous client directly.
pub trait BlockingLockBackend: Send + Sync + 'static {
    /// Run one lock script atomically.
    fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError>;

    /// Wait until `name` is no longer held, or `timeout` elapses.
    fn wait_for_unlock(
        &self,
        name: &str,
        channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError>;

    /// The publish command this deployment needs for lock notifications.
    fn publish_command(&self) -> &'static str {
        "PUBLISH"
    }
}

/// Whether a `status` reply says the lock key no longer exists.
pub(crate) fn lock_is_gone(reply: &ScriptReply) -> bool {
    reply
        .ints()
        .and_then(|values| values.first())
        .is_some_and(|exists| *exists == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_traits_are_object_safe() {
        let asynchronous: Option<&dyn LockBackend> = None;
        let blocking: Option<&dyn BlockingLockBackend> = None;

        assert!(asynchronous.is_none());
        assert!(blocking.is_none());
    }

    #[test]
    fn a_missing_lock_key_reads_as_released() {
        assert!(lock_is_gone(&ScriptReply::Ints(vec![0, 0, -2])));
        assert!(!lock_is_gone(&ScriptReply::Ints(vec![1, 1, 30_000])));
        assert!(!lock_is_gone(&ScriptReply::Nil));
    }
}
