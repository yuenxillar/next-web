use crate::core::{AuthenticationError, AuthenticationErrorKind};

pub fn invalid_one_time_token(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::BadCredentials)
}
