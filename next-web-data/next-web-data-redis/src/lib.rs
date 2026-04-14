//! Redis starter support for the `next-web` ecosystem.
//!
//! This crate is intentionally organized like a Spring Boot starter:
//! - `properties`: configuration objects bound from `application.yaml`
//! - `auto_register`: framework entry that registers Redis infrastructure automatically
//! - `service`: ready-to-use Redis services
//! - `core`: optional advanced capabilities such as distributed locks and expired-key listeners
//!
//! # Quick Start
//!
//! 1. Add this crate as a dependency.
//! 2. Call [`enable`] once during startup so cross-crate auto-registration metadata is linked.
//! 3. Configure Redis under the `next.data.redis` prefix.
//! 4. Resolve [`RedisService`] from the application context and use it directly.
//!
//! ```ignore
//! use next_web_data_redis::enable;
//!
//! fn main() {
//!     enable();
//!     // Start your next-web application here.
//! }
//! ```
//!
//! ```yaml
//! next:
//!   data:
//!     redis:
//!       host: 127.0.0.1
//!       port: 6379
//!       database: 0
//! ```

pub mod autoconfigure;
pub mod connection;
pub mod core;
pub mod listener;

#[cfg(feature = "lock")]
pub use service::redis_lock_service::RedisLockService;

pub use redis::{AsyncCommands, Commands, RedisError, aio::MultiplexedConnection};
