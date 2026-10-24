//! Retry.
//!
//! Repeats a call that failed, waiting between attempts. Use it for transient
//! faults — dropped connections, momentary timeouts — and only for operations
//! that are safe to run more than once.
//!
//! ```no_run
//! use std::time::Duration;
//! use next_web_resilience::retry::{ExponentialIntervalFunction, Retry, RetryConfig};
//!
//! let retry = Retry::new(
//!     "inventory",
//!     RetryConfig::builder()
//!         .max_attempts(4)
//!         .interval_function(
//!             ExponentialIntervalFunction::new(Duration::from_millis(100), 2.0)
//!                 .with_max_interval(Duration::from_secs(2))
//!                 .with_randomization(0.5),
//!         )
//!         .build(),
//! );
//! ```

mod interval_function;
// The public type is `Retry` and it lives in `retry/retry.rs`, so the module
// necessarily shares its name with the module that contains it.
#[allow(clippy::module_inception)]
mod retry;
mod retry_config;
mod retry_error;
mod retry_event;
mod retry_snapshot;

pub use interval_function::{
    BoxedIntervalFunction, DEFAULT_MULTIPLIER, DEFAULT_WAIT_DURATION, ExponentialIntervalFunction,
    FixedIntervalFunction, IntervalFunction,
};
pub use retry::{Retry, RetryBuilder};
pub use retry_config::{RetryConfig, RetryConfigBuilder, RetryExceptionPredicate};
pub use retry_error::RetryError;
pub use retry_event::RetryEvent;
pub use retry_snapshot::RetrySnapshot;
