use std::{error::Error, fmt::Display};

use next_web_core::anys::{any_error::AnyError, any_value::AnyValue};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AuthenticationErrorKind {
    General,
    BadCredentials,
    AccountStatus,
    CredentialsNotFound,
    Service,
    InternalService,
    ProviderNotFound,
    InsufficientAuthentication,
}

#[derive(Debug, Clone)]
pub struct AuthenticationError {
    msg: String,
    kind: AuthenticationErrorKind,
    cause: Option<Box<dyn AnyError>>,
}

impl AuthenticationError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::with_kind(msg, AuthenticationErrorKind::General)
    }

    pub fn with_kind(msg: impl Into<String>, kind: AuthenticationErrorKind) -> Self {
        Self {
            msg: msg.into(),
            kind,
            cause: None,
        }
    }

    pub fn get_message(&self) -> &str {
        &self.msg
    }

    pub fn kind(&self) -> AuthenticationErrorKind {
        self.kind
    }

    pub fn is_account_status_error(&self) -> bool {
        self.kind == AuthenticationErrorKind::AccountStatus
    }

    pub fn is_internal_service_error(&self) -> bool {
        self.kind == AuthenticationErrorKind::InternalService
    }
}
impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication Error: {}", self.msg)
    }
}

impl Error for AuthenticationError {}

impl Into<AnyValue> for AuthenticationError {
    fn into(self) -> AnyValue {
        AnyValue::Object(Box::new(self))
    }
}
