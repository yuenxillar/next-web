//! Time limiter.
//!
//! A time limiter puts an upper bound on how long one call may take. It is the
//! answer to "is it taking too long?", and the simplest of the five patterns:
//! it keeps no window and makes no decision about the health of the resource, it
//! only stops waiting.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::timelimiter::{TimeLimiter, TimeLimiterConfig};
//!
//! # async fn charge() -> Result<u32, std::io::Error> {
//! #     Ok(1)
//! # }
//! # async fn example() {
//! // Give the payment service at most two seconds to answer.
//! let limiter = TimeLimiter::new(
//!     "payments",
//!     TimeLimiterConfig::builder()
//!         .timeout_duration(Duration::from_secs(2))
//!         .build(),
//! );
//!
//! match limiter.call(charge).await {
//!     Ok(id) => println!("charged {id}"),
//!     Err(error) => eprintln!("not charged: {error}"),
//! }
//! # }
//! ```
//!
//! [`TimeLimiter::call`] cancels the guarded future when the limit is reached;
//! [`TimeLimiter::call_non_cancelling`] detaches it instead, and documents why
//! that is rarely what an application wants.

mod time_limiter;
mod time_limiter_config;
mod time_limiter_error;
mod time_limiter_event;
mod time_limiter_snapshot;

pub use time_limiter::TimeLimiter;
pub use time_limiter_config::{TimeLimiterConfig, TimeLimiterConfigBuilder};
pub use time_limiter_error::TimeLimiterError;
pub use time_limiter_event::TimeLimiterEvent;
pub use time_limiter_snapshot::TimeLimiterSnapshot;
