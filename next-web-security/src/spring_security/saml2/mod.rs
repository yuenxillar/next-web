//! The SAML 2.0 service provider of the application.
//!
//! The module holds the relying party registration a service provider describes
//! itself with, the repository that holds those registrations, the metadata
//! endpoint that publishes them, and the login profile: a response is believed
//! only when the signature the asserting party put on it covers what it claims
//! (`xml_dsig`), and its conditions hold for this service provider
//! ([`login::StrictAssertionVerifier`]). The logout profile is not wired yet.

pub mod error;
pub mod login;
pub mod metadata;
pub mod registration;
mod xml;
mod xml_dsig;

pub use error::{Saml2Error, Saml2ErrorKind};
