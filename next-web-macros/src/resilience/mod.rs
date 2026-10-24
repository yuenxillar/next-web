//! The `#[rate_limiter]`, `#[circuit_breaker]`, `#[bulkhead]`, `#[time_limiter]`
//! and `#[retry]` attributes, which guard a function with the patterns of the
//! `next-web-resilience` crate.
//!
//! The entry points live in the crate root, because a procedural macro has to be
//! defined there; this module holds what they expand to. What the user needs to
//! know — the two forms of the instance, the error contract, the order a stack of
//! them resolves in, and the one case that does not work — is documented in the
//! `next-web-resilience` crate, which re-exports them, and repeated on each
//! attribute in the crate root. It is deliberately not duplicated here.

mod attr;
mod expand;

pub(crate) use expand::{Pattern, apply};
