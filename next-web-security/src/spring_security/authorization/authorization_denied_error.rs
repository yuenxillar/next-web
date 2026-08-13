use std::fmt::{Debug, Display, Formatter};

use crate::authorization::{authorization_decision::AuthorizationDecision, AuthorizationResult};

/// An AuthorizationDeniedError that contains the AuthorizationResult
#[derive(Clone)]
pub struct AuthorizationDeniedError {
    msg: String,
    result: Box<dyn AuthorizationResult>,
}

impl AuthorizationDeniedError {
    pub fn new(msg: impl Into<String>, authorization_result: Box<dyn AuthorizationResult>) -> Self {
        assert!(
            !authorization_result.is_granted(),
            "Granted authorization results are not supported"
        );
        Self {
            msg: msg.into(),
            result: authorization_result,
        }
    }

    pub fn with_message(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            result: Box::new(AuthorizationDecision::new(false)),
        }
    }

    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.result.as_ref()
    }

    pub fn msg(&self) -> &str {
        self.msg.as_str()
    }
}

impl AuthorizationResult for AuthorizationDeniedError {
    fn is_granted(&self) -> bool {
        false
    }
}

impl Display for AuthorizationDeniedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "authorization denied")
    }
}

impl std::error::Error for AuthorizationDeniedError {}

impl Debug for AuthorizationDeniedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationDeniedError")
            .field("result", &"<dyn AuthorizationResult>") // trait object 可能无法 Debug
            .finish()
    }
}
