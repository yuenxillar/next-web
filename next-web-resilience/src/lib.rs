//! Resilience patterns for Next Web applications.
//!
//! Guarding a call to an external service means answering five questions, and
//! this crate answers each of them with one module:
//!
//! | Question | Module | What it does |
//! |---|---|---|
//! | Is the dependency still healthy? | [`circuitbreaker`] | Opens when the failure rate crosses a threshold and rejects calls until it recovers |
//! | Are we calling it too often? | [`ratelimiter`] | Caps the number of calls per period |
//! | Are we using too much of our own capacity? | [`mod@bulkhead`] | Caps how many calls run at the same time |
//! | Should we try again? | [`mod@retry`] | Repeats a failed call, with backoff |
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
//! # Guarding a method with an attribute
//!
//! With the `macros` feature — on by default — each pattern is also an attribute.
//! One replaces the **body** of the annotated function and leaves the signature,
//! and every other attribute on the item, alone:
//!
//! ```ignore
//! use next_web_resilience::{circuit_breaker, retry};
//! ```
//!
//! ## Where the instance comes from
//!
//! Every macro takes the same two forms:
//!
//! ```ignore
//! // 1. An expression that reaches the instance: a field of `self`, a static,
//! //    a local. It may be an `Arc<T>`, an `&Arc<T>` or an `&T`.
//! #[circuit_breaker(self.breakers.payments)]
//! async fn charge(&self) -> Result<u64, MyError> { .. }
//!
//! // 2. A name, resolved in the process-wide registry.
//! #[circuit_breaker(name = "payments")]
//! async fn charge(&self) -> Result<u64, MyError> { .. }
//! ```
//!
//! Prefer the expression form: the instance is then visible to the code that
//! builds it, and a test can hand in its own. The named form reaches
//! [`registry::ResilienceRegistry::global`], which creates an instance with
//! default configuration on first use — so register it at start-up if you want to
//! choose its settings, or configure it in `application.yaml` and let the
//! [`autoconfigure`] module do it.
//!
//! ## What happens when a pattern rejects the call
//!
//! A rejection is not a failure of the guarded operation — a circuit breaker
//! refusing a call never ran it — so the macro has to decide what the caller sees:
//!
//! * with **`fallback = <expression>`**, that expression is returned instead.
//!   This is the degradation path: serve a cached value, a default, a cheaper
//!   answer. A synchronous function returns it directly; in an `async fn` the
//!   expression may use `.await`.
//! * **without** a fallback, the rejection is converted with `Into::into`, so the
//!   function's error type needs a `From` implementation for the rejection it can
//!   produce: [`CallNotPermittedError`], [`RateLimitExceededError`],
//!   [`BulkheadFullError`], [`TimeoutError`], [`AbortedError`] or
//!   [`MaxRetriesExceededError`].
//!
//! ## Stacking, and the order that matters
//!
//! An attribute macro receives the item *with the other attributes still
//! attached*, and re-emits them, so the macros written below it expand afterwards
//! — around what it produced. The consequence is that **the attribute written
//! lowest ends up outermost**, and the one written highest sits closest to the
//! body. To get the order Resilience4j recommends — a retry that re-runs the whole
//! chain, so the circuit breaker sees every attempt — write the retry lowest:
//!
//! ```ignore
//! #[bulkhead(self.bulkhead)]          // innermost: closest to the body
//! #[time_limiter(self.time_limiter)]
//! #[rate_limiter(self.rate_limiter)]
//! #[circuit_breaker(self.breaker)]
//! #[retry(self.retry)]                // outermost: wraps everything above
//! async fn fetch(&self) -> Result<Payload, MyError> { .. }
//! ```
//!
//! Each macro preserves the attributes beneath it, which is what makes the chain
//! work at all.
//!
//! ## Synchronous and asynchronous functions
//!
//! [`macro@circuit_breaker`] and [`macro@retry`] guard either kind. The other
//! three guard `async fn` only, and reject a synchronous one with an explanation
//! rather than silently doing nothing: a rate limiter asks a store that may live
//! in another process, a bulkhead waits on a semaphore, and a time limiter
//! enforces its limit by dropping the future it was given.
//!
//! ## The guarded function must return `Result`
//!
//! A rejection has to reach the caller, so the function must return `Result<_, _>`,
//! spelled out rather than hidden behind a type alias.
//!
//! ## `#[async_trait]`
//!
//! The macros do **not** work on a method inside an `#[async_trait]` impl.
//! `#[async_trait]` is applied to the whole impl block, so it expands before any
//! attribute on the methods inside it and rewrites each `async fn` into a plain
//! `fn` returning a boxed future. By the time a resilience macro runs there is no
//! `async fn` left, and guarding the boxed future would record an instant success
//! for a future nobody has polled. The order cannot be reversed, so the macro
//! refuses and says so.
//!
//! Two ways out:
//!
//! * **Drop `#[async_trait]` where the trait does not need to be a `dyn Trait`.**
//!   An `async fn` in a trait is native, so the method is still an `async fn` when
//!   the macro expands and everything works.
//! * **Leave the attribute off the method** and drive the pattern from inside the
//!   body, where the call is already asynchronous.
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

/// Reading configured instances out of the application's properties.
///
/// Present only with the `autoconfigure` feature, which the framework turns on
/// together with the rest of this crate.
#[cfg(feature = "autoconfigure")]
pub mod autoconfigure;

/// The attribute macros.
///
/// Defined by the framework's `next-web-macros` crate, behind its `resilience`
/// feature, and re-exported here when the default `macros` feature is on. They
/// share the module and macro namespaces with the pattern modules, so `retry`
/// names both the module and the attribute.
#[cfg(feature = "macros")]
pub use next_web_macros::{bulkhead, circuit_breaker, rate_limiter, retry, time_limiter};
