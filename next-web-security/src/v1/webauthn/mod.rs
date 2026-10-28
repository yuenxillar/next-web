//! Web Authentication.
//!
//! A ceremony is three checks piled on each other: the client data says which
//! ceremony ran and against which challenge, the authenticator data says which
//! relying party it ran for and how many times the credential was used, and the
//! signature ties the two together. The first two are JSON and fixed-layout
//! bytes; the public key of the credential and the attestation are CBOR, and the
//! signature is the arithmetic of the algorithm the key names.

pub mod attestation;
pub mod authenticator_data;
pub mod base64url;
pub mod challenge;
pub mod client_data;
pub mod cose;
pub mod credential;
pub mod error;
pub mod manager;
pub mod options;
pub mod request;

#[cfg(test)]
pub(crate) mod test_support;

pub use attestation::{
    AttestationError, AttestationErrorKind, AttestationObject, AttestedCredential,
};
pub use authenticator_data::{AttestedCredentialData, AuthenticatorData, AuthenticatorDataError};
pub use challenge::{ChallengeRepository, InMemoryChallengeRepository};
pub use client_data::{ClientData, ClientDataError, CREATE, GET};
pub use cose::{CoseError, CoseErrorKind, CosePublicKey, ES256, RS256};
pub use credential::{Credential, InMemoryUserCredentialRepository, UserCredentialRepository};
pub use error::{WebAuthnError, WebAuthnErrorKind};
pub use manager::WebAuthnManager;
pub use options::{
    PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialRequestOptions,
    PublicKeyCredentialUserEntity, RelyingParty,
};
pub use request::{
    AuthenticationRequest, AuthenticationResponse, RegistrationRequest, RegistrationResponse,
};
