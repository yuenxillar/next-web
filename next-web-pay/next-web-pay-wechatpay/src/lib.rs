//! WeChat Pay V3 client.
pub mod client;
pub mod config;
pub mod error;
pub mod notify;
pub mod payment;
pub mod sign;

/// Alias for the crate-level result type.
pub type WechatPayResult<T> = std::result::Result<T, crate::error::WechatPayError>;

pub trait Path {
    fn path() -> &'static str;
}

pub trait ToPath {
    fn to_path(&self) -> Result<String, crate::error::WechatPayError>;
}