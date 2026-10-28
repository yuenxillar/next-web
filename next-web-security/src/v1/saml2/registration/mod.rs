//! The relying party registrations a service provider acts with.
//!
//! A registration is what the service provider knows about one identity
//! provider: the identifier the two sides use for each other, the endpoints the
//! service provider is reachable at, and the material that signs its messages.
//! The metadata endpoint publishes these values so an asserting party can be
//! configured from them.

mod relying_party_registration;
mod relying_party_registration_repository;
mod saml2_message_binding;

pub use relying_party_registration::{
    RelyingPartyRegistration, RelyingPartyRegistrationBuilder,
};
pub use relying_party_registration_repository::{
    InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistrationRepository,
};
pub use saml2_message_binding::Saml2MessageBinding;
