use std::fmt::{Display, Formatter};

use crate::authorization::authorization_decision::AuthorizationDecision;

#[derive(Clone, Debug)]
pub struct AuthorizationDeniedError {
    decision: AuthorizationDecision,
}

impl AuthorizationDeniedError {
    pub fn new(decision: AuthorizationDecision) -> Self {
        Self { decision }
    }

    pub fn decision(&self) -> &AuthorizationDecision {
        &self.decision
    }
}

impl Display for AuthorizationDeniedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "authorization denied")
    }
}

impl std::error::Error for AuthorizationDeniedError {}
