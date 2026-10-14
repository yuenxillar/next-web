//! Bulkhead.
//!
//! A bulkhead bounds how many calls run against one dependency at the same
//! time, so a dependency that gets slow cannot consume every worker the caller
//! has. It is the semaphore flavour of the pattern: a call takes one of a fixed
//! number of permits before it starts and returns it when it finishes.
//!
//! A caller that finds every permit taken is rejected immediately by default.
//! Configure a [`max_wait_duration`] to let it queue for a free slot instead,
//! which trades latency for a lower rejection count.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::bulkhead::{Bulkhead, BulkheadConfig};
//!
//! # async fn example() {
//! let bulkhead = Bulkhead::new(
//!     "inventory",
//!     BulkheadConfig::builder()
//!         .max_concurrent_calls(10)
//!         .max_wait_duration(Duration::from_millis(50))
//!         .build(),
//! );
//!
//! match bulkhead.call(|| async { Ok::<_, std::io::Error>(42) }).await {
//!     Ok(value) => println!("inventory answered {value}"),
//!     Err(error) => eprintln!("inventory not called: {error}"),
//! }
//! # }
//! ```
//!
//! [`max_wait_duration`]: BulkheadConfigBuilder::max_wait_duration

// The module holding the type is named after the directory it lives in, as the
// pattern modules are laid out file per concern; clippy would rather it were
// named something else.
#[allow(clippy::module_inception)]
mod bulkhead;
mod bulkhead_config;
mod bulkhead_error;
mod bulkhead_event;
mod bulkhead_snapshot;

pub use bulkhead::{Bulkhead, BulkheadPermit};
pub use bulkhead_config::{BulkheadConfig, BulkheadConfigBuilder};
pub use bulkhead_error::BulkheadError;
pub use bulkhead_event::BulkheadEvent;
pub use bulkhead_snapshot::BulkheadSnapshot;
