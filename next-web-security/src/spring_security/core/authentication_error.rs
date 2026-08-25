use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::Arc,
};

use next_web_core::{
    anys::{any_error::AnyError, any_value::AnyValue},
    filter::{FilterChainError, FilterError},
};

use crate::core::Authentication;

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
    AuthenticationService,
    InternalAuthentication,
    SessionAuthentication,
    InvalidClientRegistrationId,
}

#[derive(Clone)]
pub struct AuthenticationError {
    msg: String,
    kind: AuthenticationErrorKind,
    cause: Option<Box<dyn AnyError>>,
    authentication_request: Option<Arc<dyn Authentication>>,
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
            authentication_request: None,
        }
    }

    pub fn message(&self) -> &str {
        &self.msg
    }

    pub fn kind(&self) -> AuthenticationErrorKind {
        self.kind
    }

    pub fn set_authentication_request(&mut self, authentication_request: Arc<dyn Authentication>) {
        self.authentication_request = Some(authentication_request);
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

impl Debug for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication Error: {}", self.msg)
    }
}

impl Into<AnyValue> for AuthenticationError {
    fn into(self) -> AnyValue {
        AnyValue::Object(Box::new(self))
    }
}

impl From<AuthenticationError> for FilterError {
    fn from(value: AuthenticationError) -> Self {
        FilterError::Chain(FilterChainError::AnyError(Box::new(value)))
    }
}
