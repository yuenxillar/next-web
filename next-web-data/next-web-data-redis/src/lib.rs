//! Redis starter support for the `next-web` ecosystem.
//!
//! This crate is intentionally organized like a Spring Boot starter:
//! - `autoconfigure`: configuration bound from `application.yaml` and the
//!   registration that wires Redis into the application context
//! - `connection`: message and listener types for Pub/Sub
//! - `core`: `RedisTemplate`, the listener container, and the distributed locks
//! - `lock`: the asynchronous and blocking distributed locks
//!
//! # Quick Start
//!
//! 1. Add this crate as a dependency.
//! 2. Configure Redis under the `next.data.redis` prefix.
//! 3. Resolve [`RedisTemplate`](crate::core::redis_template::RedisTemplate) from
//!    the application context and use it directly.
//!
//! ```yaml
//! next:
//!   data:
//!     redis:
//!       host: 127.0.0.1
//!       port: 6379
//!       database: 0
//! ```
//!
//! Distributed locks are opt-in through `next.data.redis.lock.enabled`; see the
//! crate README for the asynchronous and blocking APIs.

pub mod autoconfigure;
pub mod connection;
pub mod core;

#[cfg(feature = "distributed-lock")]
pub use crate::lock::{
    BlockingLockBackend, BlockingRedLock, BlockingRedLockGuard, BlockingRedisLock,
    BlockingRedisLockGuard, LockBackend, LockRequest, LockScript, RedLock, RedLockGuard, RedisLock,
    RedisLockConfig, RedisLockError, RedisLockGuard, RedisLockService, ScriptArg, ScriptReply,
    UnlockOutcome,
};

pub use redis::{AsyncCommands, Commands, RedisError, aio::MultiplexedConnection};

#[cfg(feature = "distributed-lock")]
pub mod lock;
