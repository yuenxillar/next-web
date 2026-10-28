//! The server side of OAuth 2.0.

#[cfg(feature = "oauth2-authorization-server")]
pub mod authorization;
#[cfg(feature = "oauth2-resource-server")]
pub mod resource;
