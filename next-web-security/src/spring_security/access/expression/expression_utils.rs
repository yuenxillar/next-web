use crate::authorization::AuthorizationDecision;

pub struct ExpressionUtils;

impl ExpressionUtils {
    pub fn evaluate_as_boolean(value: bool) -> bool {
        value
    }

    pub fn decision(value: bool) -> AuthorizationDecision {
        AuthorizationDecision::new(value)
    }
}
