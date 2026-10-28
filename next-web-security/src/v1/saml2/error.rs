//! The errors the SAML 2.0 profiles report.

use std::fmt;

/// What went wrong while a SAML 2.0 message was handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Saml2ErrorKind {
    /// The registration does not carry a value the profile needs.
    MissingConfiguration,
    /// The request names a registration the application does not have.
    RegistrationNotFound,
    /// The message could not be read as a SAML 2.0 message.
    InvalidResponse,
    /// The response is not the one the request asked for.
    VerificationFailed,
    /// The subject of the response is not a user of the application.
    UserNotFound,
    /// A message could not be encoded for transport.
    Encoding,
}

/// A failure of one of the SAML 2.0 profiles.
///
/// The kind is what a caller branches on — a missing registration is a
/// configuration error, while a failed verification is a rejected login — and
/// the message is what is logged and handed to the operator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saml2Error {
    kind: Saml2ErrorKind,
    message: String,
}

impl Saml2Error {
    /// Creates an error of the given kind with the given message.
    pub fn new(kind: Saml2ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns what went wrong.
    pub fn kind(&self) -> Saml2ErrorKind {
        self.kind
    }

    /// Returns the message that describes the failure.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Saml2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Saml2Error {}

impl From<std::io::Error> for Saml2Error {
    fn from(error: std::io::Error) -> Self {
        Self::new(Saml2ErrorKind::Encoding, error.to_string())
    }
}
