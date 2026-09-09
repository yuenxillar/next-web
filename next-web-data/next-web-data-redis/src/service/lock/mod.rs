//! Redisson 4.7-compatible distributed lock subsystem.
//!
//! The module is intentionally split by the same responsibilities used by
//! Redisson's Java implementation: configuration, protocol/key naming,
//! transport, ownership/guard lifecycle, standard `RLock`, and quorum
//! `RedLock`.

pub mod backend;
pub mod protocol;
mod redisson_lock;
pub mod redlock;
pub mod rlock;

pub mod config;
pub mod error;
pub mod lock_pub_sub;
pub mod lock_renewal_scheduler;
pub mod lock_scripts;
pub mod redisson_base_lock;
pub mod redisson_expirable;
pub mod redisson_multi_lock;
pub mod redisson_object;
pub mod redisson_red_lock;

pub use redisson_lock::{
    LockError, LockGuard, RedisDistributedLockService, RedisLock, RedisLockConfig, RedisLockMode,
    RedisRedLock, UnlockResult,
};
