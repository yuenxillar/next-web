use crate::authorization::{AuthorizationResult, RequiredFactorError};

/// An AuthorizationResult that contains RequiredFactorError.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorAuthorizationDecision {
    factor_errors: Vec<RequiredFactorError>,
}

impl FactorAuthorizationDecision {
    /// Creates a new instance.
    pub fn new(factor_errors: Vec<RequiredFactorError>) -> Self {
        assert!(
            !factor_errors.is_empty(),
            "factor_errors must not contain null elements"
        );

        Self { factor_errors }
    }

    /// The specified RequiredFactorErrors
    pub fn factor_errors(&self) -> &[RequiredFactorError] {
        &self.factor_errors
    }
}

impl AuthorizationResult for FactorAuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.factor_errors.is_empty()
    }
}
