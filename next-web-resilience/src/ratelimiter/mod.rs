//! Rate limiter.
//!
//! Caps how often a guarded resource may be called. The budget lives in a
//! [`RateLimiterStore`]: the default, [`InMemoryRateLimiterStore`], keeps it in
//! this process, and implementing the trait lets a shared service such as Redis
//! hold it so that one quota covers every instance of an application.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::ratelimiter::{RateLimiter, RateLimiterConfig};
//!
//! # async fn example() {
//! let limiter = RateLimiter::new(
//!     "payments",
//!     RateLimiterConfig::builder()
//!         .limit_for_period(10)
//!         .limit_refresh_period(Duration::from_secs(1))
//!         .timeout_duration(Duration::from_millis(500))
//!         .build(),
//! );
//!
//! match limiter.call(|| async { Ok::<_, std::io::Error>("charged") }).await {
//!     Ok(value) => println!("{value}"),
//!     Err(error) => eprintln!("not charged: {error}"),
//! }
//! # }
//! ```

mod rate_limiter;
mod rate_limiter_config;
mod rate_limiter_error;
mod rate_limiter_event;
mod rate_limiter_snapshot;
mod rate_limiter_store;

pub use rate_limiter::RateLimiter;
pub use rate_limiter_config::{RateLimiterConfig, RateLimiterConfigBuilder};
pub use rate_limiter_error::RateLimiterError;
pub use rate_limiter_event::RateLimiterEvent;
pub use rate_limiter_snapshot::RateLimiterSnapshot;
pub use rate_limiter_store::{
    InMemoryRateLimiterStore, PermissionRequest, RateLimiterStore, Refusal,
};
