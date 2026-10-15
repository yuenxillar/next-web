//! Instances configured from the application's properties.
//!
//! A pattern instance is normally built in code and reached through the
//! registry. This module adds the other half of that: settings written in
//! `application.yaml` under a prefix per pattern, which are read once at startup
//! and registered, so `name = "..."` on [the attribute macros] resolves to an
//! instance whose configuration came from the file.
//!
//! ```yaml
//! next:
//!   ratelimiter:
//!     instances:
//!       backendA:
//!         limitForPeriod: 5
//!         limitRefreshPeriod: 1s
//!         timeoutDuration: 0
//! ```
//!
//! Every instance setting is optional: what a file leaves out keeps the builder's
//! default.
//!
//! # Where the instances end up
//!
//! Each one is built from the corresponding `*Config::builder()` and registered
//! **twice, under the same name and behind the same `Arc`**:
//!
//! * in the process-wide registry, which is what `name = "..."` looks in;
//! * in the application context, so code that resolves things through the
//!   container finds it too.
//!
//! Because it is one `Arc`, the two routes cannot drift apart.
//!
//! # Naming
//!
//! An instance name is free-form and is used **exactly as written**: whatever the
//! file calls an instance is the name to hand to `name = "..."`, so a
//! configuration copied from another project works unchanged and no spelling is
//! rewritten on the way in. `backendE` in the file is reached as `backendE` —
//! asking for `backend_e` finds nothing, reports the mistake once, and then serves
//! the call on the defaults rather than refusing it.
//!
//! Field names are `snake_case` in Rust and accept both spellings in the file:
//! `limitForPeriod` and `limit_for_period` are the same setting. A key inside an
//! instance that matches no field is an error rather than being ignored.
//!

mod duration_value;
mod resilience_auto_configuration;
mod resilience_properties;

pub use duration_value::{DurationParseError, DurationValue};
pub use resilience_auto_configuration::{
    Registered, RegisteredInstances, ResilienceAutoConfiguration,
};
pub use resilience_properties::{
    BulkheadInstanceProperties, BulkheadProperties, CircuitBreakerInstanceProperties,
    CircuitBreakerProperties, RateLimiterInstanceProperties, RateLimiterProperties,
    RetryInstanceProperties, RetryProperties, SlidingWindowTypeProperty,
    TimeLimiterInstanceProperties, TimeLimiterProperties,
};
