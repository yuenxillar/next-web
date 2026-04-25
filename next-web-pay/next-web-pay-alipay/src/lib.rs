//! Alipay OpenAPI client for page payment and face-to-face payment.

pub mod client;
pub mod config;
pub mod error;
pub mod notify;
pub mod service;
pub mod sign;
pub mod payment;
pub mod util;

mod tests;

/// Crate level error type.
pub use crate::error::alipay_error::AlipayError;

/// Crate level result type.
pub type AlipayResult<T> = std::result::Result<payment::model::AlipayResponse<T>, crate::AlipayError>;


pub trait Named {
    fn name() -> &'static str;   
}