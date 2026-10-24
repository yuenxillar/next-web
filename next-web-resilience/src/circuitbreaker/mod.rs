//! Circuit breaker.
//!
//! ```no_run
//! use std::sync::Arc;
//! use next_web_resilience::circuitbreaker::{CircuitBreaker, CircuitBreakerConfig};
//!
//! let breaker = Arc::new(CircuitBreaker::new(
//!     "payments",
//!     CircuitBreakerConfig::builder()
//!         .sliding_window_size(50)
//!         .minimum_number_of_calls(10)
//!         .failure_rate_threshold(40.0)
//!         .wait_duration_in_open_state(std::time::Duration::from_secs(30))
//!         .build(),
//! ));
//! ```

mod circuit_breaker;
mod circuit_breaker_config;
mod circuit_breaker_error;
mod circuit_breaker_event;
mod circuit_breaker_snapshot;
mod circuit_breaker_state;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerBuilder};
pub use circuit_breaker_config::{
    CircuitBreakerConfig, CircuitBreakerConfigBuilder, ErrorPredicate, SlowCallPredicate,
};
pub use circuit_breaker_error::CircuitBreakerError;
pub use circuit_breaker_event::CircuitBreakerEvent;
pub use circuit_breaker_snapshot::CircuitBreakerSnapshot;
pub use circuit_breaker_state::CircuitBreakerState;
