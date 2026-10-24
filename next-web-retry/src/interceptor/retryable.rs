//! Marks a service whose calls are retried.

/// Marks a service whose calls are retried.
///
/// The original marker interface is implemented by generated proxies. Rust needs no proxy for
/// the interceptors of this crate: they are called explicitly with the closure that performs
/// the call. The marker stays useful as a compile-time guarantee that a type is expected to be
/// wrapped by an interceptor, so implement it on such services.
pub trait Retryable {}
