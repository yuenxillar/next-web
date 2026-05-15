use crate::core::authentication_error::{AuthenticationError, AuthenticationErrorKind};

pub fn pre_authenticated_credentials_not_found(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::BadCredentials)
}
