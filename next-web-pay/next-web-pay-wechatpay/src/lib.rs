//! WeChat Pay V2 client.

pub mod client;
pub mod config;
pub mod error;
pub mod model;
pub mod notify;
pub mod sign;
pub mod xml;

pub use client::WechatPayClient;
pub use config::WechatPayConfig;
pub use error::WechatPayError;

/// Crate-level result type.
pub type Result<T> = std::result::Result<T, WechatPayError>;
