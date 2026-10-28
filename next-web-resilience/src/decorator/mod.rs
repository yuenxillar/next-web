//! Composes the resilience patterns into one pipeline.
//!
//! Each pattern answers a different question, and a real call to an external
//! service usually needs more than one answer: a circuit breaker to stop
//! hammering a dependency that is down, a rate limiter to stay inside a quota,
//! a time limiter so a slow answer does not pin a task, a bulkhead so one slow
//! dependency cannot consume every slot, and a retry for the faults that are
//! genuinely transient.
//!
//! ```
//! use std::time::Duration;
//! use next_web_resilience::circuitbreaker::{CircuitBreaker, CircuitBreakerConfig};
//! use next_web_resilience::decorator::Resilience;
//! use next_web_resilience::registry::ResilienceRegistry;
//!
//! # async fn charge() -> Result<u32, std::io::Error> {
//! #     Ok(1)
//! # }
//! # async fn example() {
//! let registry = ResilienceRegistry::new();
//! registry.circuit_breaker("payments", || {
//!     CircuitBreakerConfig::builder()
//!         .sliding_window_size(20)
//!         .minimum_number_of_calls(10)
//!         .failure_rate_threshold(50.0)
//!         .wait_duration_in_open_state(Duration::from_secs(30))
//!         .build()
//! });
//!
//! let resilience = Resilience::from_registry(&registry, "payments");
//!
//! match resilience.execute(charge).await {
//!     Ok(id) => println!("charged {id}"),
//!     Err(error) => eprintln!("not charged: {error}"),
//! }
//! # }
//! ```

mod resilience;

pub use resilience::{Resilience, ResilienceBuilder};
