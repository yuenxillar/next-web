//! Alipay OpenAPI client for page payment and face-to-face payment.

pub mod client;
pub mod config;
pub mod error;
pub mod payment;
pub mod service;
pub mod sign;
pub mod util;

/// Crate level error type.
pub use crate::error::alipay_error::AlipayError;

/// Crate level result type.
pub type AlipayResult<T> = std::result::Result<T, crate::AlipayError>;

pub trait Named {
    fn name() -> &'static str;
}

pub trait Method {
    fn method() -> &'static str;
}
