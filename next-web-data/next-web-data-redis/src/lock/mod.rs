//! Modular, Redisson-compatible distributed locks.
//!
//! The subsystem is split by responsibility:
//!
//! - [`protocol`] owns everything that reaches the wire: key naming, the five
//!   Lua scripts, the request builders, and the reply parser.
//! - [`backend`] owns everything that performs IO: one trait for the async
//!   transport, one for the blocking transport, plus the cluster and in-memory
//!   implementations.
//! - [`async_lock`] and [`blocking`] are the two lock flavours. They run the
//!   same state machine over their own transport.
//! - [`redlock`] composes several independent deployments into a quorum lock.
//! - [`service`] is the factory application code goes through.
//!
//! # Two flavours, one protocol
//!
//! [`RedisLock`] is asynchronous and wakes its waiters through Pub/Sub.
//! [`BlockingRedisLock`] has the same semantics through the `redis` crate's
//! synchronous client: no `block_on`, no runtime, and it blocks the calling
//! thread. Neither is a wrapper around the other.
//!
//! # Reentrancy follows the handle
//!
//! Redisson identifies a lock holder by thread. A lock is routinely held across
//! `.await` points here, where no thread identity survives, so a handle carries
//! an owner token instead: acquiring through the same handle (or a clone) is
//! reentrant, while a second [`RedisLockService::get_lock`] call for one name
//! mints a different token and therefore contends.
//!
//! # Only Lua mutates
//!
//! Every state change — acquire, renew, release, force-release — is a single
//! script invocation, so reentrancy, renewal and release cannot interleave with
//! a competing handle.

pub mod async_lock;
pub mod backend;
pub mod blocking;
pub mod config;
pub mod error;
pub mod ownership;
pub mod protocol;
pub mod redlock;
pub mod service;

pub(crate) mod guard;
pub(crate) mod lease;
pub(crate) mod retry;

#[cfg(test)]
mod test_support;

pub use async_lock::{RedisLock, RedisLockGuard};
pub use backend::{BlockingLockBackend, LockBackend};
pub use blocking::{BlockingRedisLock, BlockingRedisLockGuard};
pub use config::RedisLockConfig;
pub use error::{RedisLockError, UnlockOutcome};
pub use ownership::{LockOwner, OwnerToken};
pub use protocol::{LockRequest, LockScript, ScriptArg, ScriptReply};
pub use redlock::{BlockingRedLock, BlockingRedLockGuard, RedLock, RedLockGuard, quorum_for};
pub use service::RedisLockService;
