//! The `ProxyHttp` implementation: the request lifecycle of the gateway.

pub mod error_response;
mod pipeline;

pub use pipeline::GatewayProxy;
