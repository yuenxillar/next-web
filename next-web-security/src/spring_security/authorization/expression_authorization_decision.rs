use crate::authorization::authorization_decision::AuthorizationDecision;

/// An AuthorizationDecision that also carries the expression that produced the decision.
#[derive(Clone, Debug)]
pub struct ExpressionAuthorizationDecision {
    decision: AuthorizationDecision,
    expression: String,
}

impl ExpressionAuthorizationDecision {
    pub fn new(granted: bool, expression: impl Into<String>) -> Self {
        Self {
            decision: AuthorizationDecision::new(granted),
            expression: expression.into(),
        }
    }

    pub fn is_granted(&self) -> bool {
        todo!()
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }
}
