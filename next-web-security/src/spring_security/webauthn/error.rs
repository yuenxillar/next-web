//! The error of a Web Authentication ceremony.
//!
//! A ceremony is a chain of checks, and a relying party that refuses one has to
//! be able to tell which: the log of a refused login says whether the challenge
//! was stale, the origin was wrong, or the signature did not verify. The kind is
//! the part a caller can match on; the message is the part a reader can act on.

use std::{error::Error, fmt};

use super::{
    attestation::AttestationError,
    authenticator_data::AuthenticatorDataError,
    client_data::ClientDataError,
    cose::CoseError,
};

/// What part of a ceremony refused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebAuthnErrorKind {
    /// The request the browser submitted is not the request of a ceremony.
    Request,
    /// The client data of the ceremony does not hold.
    ClientData,
    /// The authenticator data of the ceremony does not hold.
    AuthenticatorData,
    /// The challenge was never issued, was already answered, or expired.
    Challenge,
    /// The attestation statement is not one this port can verify.
    Attestation,
    /// The signature does not verify against the credential.
    Signature,
    /// The credential is unknown, belongs to another user, or went backwards.
    Credential,
    /// The relying party is not configured to run a ceremony.
    Configuration,
}

/// The error of a Web Authentication ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebAuthnError {
    kind: WebAuthnErrorKind,
    message: String,
}

impl WebAuthnError {
    /// Creates the error of the given kind with the given message.
    pub fn new(kind: WebAuthnErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns what part of the ceremony refused it.
    pub fn kind(&self) -> WebAuthnErrorKind {
        self.kind
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for WebAuthnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for WebAuthnError {}

impl From<ClientDataError> for WebAuthnError {
    fn from(error: ClientDataError) -> Self {
        Self::new(WebAuthnErrorKind::ClientData, error.to_string())
    }
}

impl From<AuthenticatorDataError> for WebAuthnError {
    fn from(error: AuthenticatorDataError) -> Self {
        Self::new(WebAuthnErrorKind::AuthenticatorData, error.to_string())
    }
}

impl From<AttestationError> for WebAuthnError {
    fn from(error: AttestationError) -> Self {
        Self::new(WebAuthnErrorKind::Attestation, error.to_string())
    }
}

impl From<CoseError> for WebAuthnError {
    fn from(error: CoseError) -> Self {
        match error.kind() {
            super::cose::CoseErrorKind::Signature => {
                Self::new(WebAuthnErrorKind::Signature, error.to_string())
            }
            _ => Self::new(WebAuthnErrorKind::Credential, error.to_string()),
        }
    }
}
