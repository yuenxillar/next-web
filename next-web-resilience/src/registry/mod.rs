//! The registry of named resilience instances.
//!
//! A pattern is only meaningful if every caller shares one instance, and this
//! module is where those shared instances live.
//!
//! ```
//! use next_web_resilience::registry::ResilienceRegistry;
//!
//! let registry = ResilienceRegistry::new();
//! let breaker = registry.circuit_breaker_or_default("payments");
//! let same = registry.circuit_breaker_or_default("payments");
//!
//! assert!(std::sync::Arc::ptr_eq(&breaker, &same));
//! ```

mod resilience_registry;

pub use resilience_registry::{ResilienceRegistry, global_registry};
