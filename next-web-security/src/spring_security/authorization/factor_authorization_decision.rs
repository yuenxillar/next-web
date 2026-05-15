use crate::authorization::required_factor_error::RequiredFactorError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorAuthorizationDecision {
    factor_errors: Vec<RequiredFactorError>,
}

impl FactorAuthorizationDecision {
    pub fn new(factor_errors: Vec<RequiredFactorError>) -> Self {
        Self { factor_errors }
    }

    pub fn is_granted(&self) -> bool {
        self.factor_errors.is_empty()
    }

    pub fn factor_errors(&self) -> &[RequiredFactorError] {
        &self.factor_errors
    }
}
