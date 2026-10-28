//! The login profile of the service provider.
//!
//! This slice covers a login up to the point where a response is handed to an
//! [`AssertionVerifier`]: the authentication request that is sent to the
//! asserting party, the response as it is parsed, and the token the response is
//! converted into. The shipped verifier checks the signature of a response
//! ([`XmlSignatureVerifier`]) and the conditions the profile requires of it
//! ([`StrictAssertionVerifier`]); an application that wants another signature
//! implementation supplies one.

mod assertion_verifier;
mod saml2_authentication;
mod saml2_authentication_provider;
mod saml2_authentication_request;
mod saml2_authentication_token;
mod saml2_response;
mod strict_assertion_verifier;
mod xml_signature_verifier;

pub use assertion_verifier::{AssertionVerifier, RejectingAssertionVerifier};
pub use saml2_authentication::Saml2Authentication;
pub use saml2_authentication_provider::Saml2AuthenticationProvider;
pub use saml2_authentication_request::Saml2AuthenticationRequest;
pub use saml2_authentication_token::Saml2AuthenticationToken;
pub use saml2_response::Saml2Response;
pub use strict_assertion_verifier::{
    AssertionReplayCache, InMemoryAssertionReplayCache,
    InMemorySaml2AuthenticationRequestRepository, Saml2AuthenticationRequestRepository,
    SignatureVerifier, StrictAssertionVerifier,
};
pub use xml_signature_verifier::XmlSignatureVerifier;
