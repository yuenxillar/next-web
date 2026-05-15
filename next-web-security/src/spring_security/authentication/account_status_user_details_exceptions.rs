use crate::core::authentication_error::{AuthenticationError, AuthenticationErrorKind};

pub fn bad_credentials() -> AuthenticationError {
    AuthenticationError::with_kind("Bad credentials", AuthenticationErrorKind::BadCredentials)
}

pub fn locked() -> AuthenticationError {
    AuthenticationError::with_kind("User account is locked", AuthenticationErrorKind::AccountStatus)
}

pub fn disabled() -> AuthenticationError {
    AuthenticationError::with_kind("User is disabled", AuthenticationErrorKind::AccountStatus)
}

pub fn account_expired() -> AuthenticationError {
    AuthenticationError::with_kind("User account has expired", AuthenticationErrorKind::AccountStatus)
}

pub fn credentials_expired() -> AuthenticationError {
    AuthenticationError::with_kind(
        "User credentials have expired",
        AuthenticationErrorKind::AccountStatus,
    )
}

pub fn authentication_service(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::Service)
}

pub fn internal_authentication_service(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::InternalService)
}

pub fn provider_not_found(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::ProviderNotFound)
}

pub fn compromised_password(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::Service)
}

pub fn credentials_not_found(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::CredentialsNotFound)
}

pub fn insufficient_authentication(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::InsufficientAuthentication)
}
