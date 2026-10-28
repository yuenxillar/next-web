use crate::core::{AuthenticationError, AuthenticationErrorKind};

/// Thrown if an authentication request is rejected because the digest nonce
/// has expired.
///
/// This exception is used to signal to the client that a new nonce should
/// be generated, typically resulting in a `stale=true` directive on the
/// WWW-Authenticate challenge.
#[derive(Debug, Clone)]
pub struct NonceExpiredException {
    msg: String,
}

impl NonceExpiredException {
    /// Constructs a `NonceExpiredException` with the specified message.
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    /// Returns the error message.
    pub fn get_message(&self) -> &str {
        &self.msg
    }
}

impl std::fmt::Display for NonceExpiredException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NonceExpired: {}", self.msg)
    }
}

impl std::error::Error for NonceExpiredException {}

impl From<NonceExpiredException> for AuthenticationError {
    fn from(ex: NonceExpiredException) -> Self {
        AuthenticationError::with_kind(ex.msg, AuthenticationErrorKind::CredentialsNotFound)
    }
}
