use crate::authorization::authorization_decision::AuthorizationDecision;
use crate::authorization::factor_authorization_decision::FactorAuthorizationDecision;

pub trait AuthorizationResult: Send + Sync {
    fn is_granted(&self) -> bool;
}

impl AuthorizationResult for AuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.is_granted()
    }
}

impl AuthorizationResult for FactorAuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.is_granted()
    }
}
