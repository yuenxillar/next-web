use std::fmt::{Debug, Display, Formatter};

use crate::authorization::{authorization_decision::AuthorizationDecision, AuthorizationResult};

#[derive(Clone)]
pub struct AuthorizationDeniedError {
    msg: String,
    result: Box<dyn AuthorizationResult>,
}

impl AuthorizationDeniedError {
    pub fn new<R>(msg: impl Into<String>, authorization_result: R) -> Self
    where
        R: AuthorizationResult,
    {
        assert!(
            !authorization_result.is_granted(),
            "Granted authorization results are not supported"
        );
        let result = Box::new(authorization_result);
        Self {
            msg: msg.into(),
            result,
        }
    }

    pub fn decision(&self) -> &AuthorizationDecision {
        todo!()
    }

    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.result.as_ref()
    }

    pub fn msg(&self) -> &str {
        self.msg.as_str()
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
            .field("decision", &self.decision())
            .field("result", &"<dyn AuthorizationResult>") // trait object 可能无法 Debug
            .finish()
    }
}
