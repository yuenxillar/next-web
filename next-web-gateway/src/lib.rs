//! A Spring Cloud Gateway equivalent built on top of pingora.
//!
//! The crate is organised bottom up:
//!
//! - [`config`] parses and validates `next.gateway` and collects every problem
//!   with its document path.
//! - [`handler`] turns predicate and filter expressions into behaviour, failing
//!   at startup on an unknown name.
//! - [`upstream`] resolves names and balances `lb://` routes.
//! - [`proxy`] runs the request lifecycle.
//! - [`metrics`] exposes counters in the Prometheus text format.

mod context;
mod error;
mod filter;
mod handler;
mod server;
mod service;
mod util;

pub mod circuit_breaker;
pub mod config;
pub mod metrics;
pub mod proxy;
pub mod upstream;

pub use proxy::GatewayProxy;

/// A component with an explicit ordering value.
pub trait Ordered {
    /// Lower values run first.
    fn order(&self) -> i32;
}
