//! The metadata a service provider publishes about itself.
//!
//! The document tells an asserting party where to post an assertion, where to
//! send a logout request, and which certificate the service provider signs
//! with, so the two sides can be configured from one exchange.

mod saml2_metadata;

pub use saml2_metadata::Saml2Metadata;
