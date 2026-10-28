//! Errors and release outcomes for distributed lock operations.

use redis::RedisError;

/// Errors returned by distributed lock operations.
#[derive(Debug, thiserror::Error)]
pub enum RedisLockError {
    /// The Redis command failed.
    #[error("redis operation failed: {0}")]
    Redis(#[from] RedisError),
    /// The lock configuration is unusable.
    #[error("invalid distributed lock configuration: {0}")]
    InvalidConfiguration(String),
    /// A duration was zero, or too large to express in milliseconds.
    #[error("invalid duration for `{field}`")]
    InvalidDuration {
        /// The configuration field that failed validation.
        field: &'static str,
    },
    /// The lock is not held by this owner.
    #[error("the lock is not held by this owner")]
    NotOwner,
    /// The watchdog lost ownership of the lease.
    #[error("the lock watchdog lost ownership")]
    WatchdogLost,
    /// Too few nodes answered to reach a quorum.
    #[error("lock quorum unavailable: required {required}, acquired {acquired}")]
    QuorumUnavailable {
        /// The number of nodes that must answer.
        required: usize,
        /// The number of nodes that did answer.
        acquired: usize,
    },
    /// A backend call exceeded its command timeout.
    #[error("`{script}` exceeded the command timeout")]
    CommandTimeout {
        /// The name of the script whose call timed out.
        script: &'static str,
    },
    /// A script returned a value the caller cannot interpret.
    #[error("unexpected script reply: {0}")]
    UnexpectedReply(String),
    /// Release was confirmed by some nodes and failed on others.
    #[error("lock release was only partially confirmed; {failures} node(s) failed")]
    PartiallyReleased {
        /// The number of nodes whose release failed.
        failures: usize,
    },
}

/// The result of releasing one reentrant acquisition level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockOutcome {
    /// Another acquisition level still holds the lock.
    Reentrant,
    /// The lock was fully released and its waiters were notified.
    Released,
}
