//! Resilience patterns for Next Web applications.
//!
//! Guarding a call to an external service means answering five questions, and
//! this crate answers each of them with one module:
//!
//! | Question | Module | What it does |
//! |---|---|---|
//! | Is the dependency still healthy? | [`circuitbreaker`] | Opens when the failure rate crosses a threshold and rejects calls until it recovers |
//! | Are we calling it too often? | [`ratelimiter`] | Caps the number of calls per period |
//! | Are we using too much of our own capacity? | [`bulkhead`] | Caps how many calls run at the same time |
//! | Should we try again? | [`retry`] | Repeats a failed call, with backoff |
//! | Is it taking too long? | [`timelimiter`] | Caps how long one call may take |
//!
//! Each pattern can be used on its own, driven either through its
//! `call` helper or by hand through its `acquire_permission` / `on_success` /
//! `on_error` primitives. [`decorator::Resilience`] composes several of them
//! into one pipeline in the order Resilience4j recommends.
//!
//! [`registry::ResilienceRegistry`] hands out one shared instance per guarded
//! resource, which is what an application normally wants: the counters of a
//! pattern are only meaningful if every caller shares them.
//!
//! # Example
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::circuitbreaker::{CircuitBreaker, CircuitBreakerConfig};
//!
//! # async fn charge() -> Result<u32, std::io::Error> {
//! #     Ok(1)
//! # }
//! # async fn example() {
//! // Open the breaker once half of the last 10 calls failed.
//! let breaker = CircuitBreaker::new(
//!     "payments",
//!     CircuitBreakerConfig::builder()
//!         .sliding_window_size(10)
//!         .minimum_number_of_calls(10)
//!         .failure_rate_threshold(50.0)
//!         .wait_duration_in_open_state(Duration::from_secs(30))
//!         .build(),
//! );
//!
//! match breaker.call(charge).await {
//!     Ok(id) => println!("charged {id}"),
//!     Err(error) => eprintln!("not charged: {error}"),
//! }
//! # }
//! ```

#![warn(missing_docs)]

pub mod bulkhead;
pub mod circuitbreaker;
pub mod clock;
pub mod decorator;
pub mod error;
pub mod ratelimiter;
pub mod registry;
pub mod retry;
pub mod sliding_window;
pub mod timelimiter;

mod event;

/// The attribute macro this crate writes its async traits with.
///
/// Re-exported so that an implementor of
/// [`ratelimiter::RateLimiterStore`] — or of any other async trait added here
/// later — can use the exact version this crate was built against, instead of
/// having to pin a matching `async-trait` dependency of its own.
pub use next_web_core::async_trait;

pub use clock::{Clock, ManualClock, SharedClock, SystemClock};
pub use error::{
    AbortedError, BulkheadFullError, CallNotPermittedError, MaxRetriesExceededError,
    RateLimitExceededError, ResilienceError, TimeoutError,
};
pub use sliding_window::{CallOutcome, SlidingWindowType, WindowSnapshot};

pub use registry::global_registry;

/// The attribute macros.
///
/// Re-exported from `next-web-resilience-macros`, which the default `macros`
/// feature turns on. They share the module and macro namespaces with the pattern
/// modules, so `retry` names both the module and the attribute.
#[cfg(feature = "macros")]
pub use next_web_resilience_macros::{
    bulkhead, circuit_breaker, rate_limiter, retry, time_limiter,
};
