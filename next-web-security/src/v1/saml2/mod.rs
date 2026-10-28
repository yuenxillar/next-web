//! The SAML 2.0 service provider of the application.
//!
//! The module holds the relying party registration a service provider describes
//! itself with, the repository that holds those registrations, the metadata
//! endpoint that publishes them, and the two profiles: a response is believed only
//! when the signature the asserting party put on it covers what it claims
//! (`xml_dsig`) and its conditions hold for this service provider
//! ([`login::StrictAssertionVerifier`]), and a logout request of the asserting
//! party is believed the same way ([`logout`]).

pub mod error;
pub mod login;
pub mod logout;
pub mod metadata;
pub mod registration;
pub(crate) mod xml;
pub(crate) mod xml_dsig;

pub use error::{Saml2Error, Saml2ErrorKind};
