use crate::authorization::required_factor::RequiredFactor;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum RequiredFactorErrorReason {
    Missing,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RequiredFactorError {
    required_factor: RequiredFactor,
    reason: RequiredFactorErrorReason,
}

impl RequiredFactorError {
    pub fn create_missing(required_factor: RequiredFactor) -> Self {
        Self {
            required_factor,
            reason: RequiredFactorErrorReason::Missing,
        }
    }

    pub fn create_expired(required_factor: RequiredFactor) -> Self {
        Self {
            required_factor,
            reason: RequiredFactorErrorReason::Expired,
        }
    }

    pub fn required_factor(&self) -> &RequiredFactor {
        &self.required_factor
    }

    pub fn reason(&self) -> &RequiredFactorErrorReason {
        &self.reason
    }

    pub fn is_expired(&self) -> bool {
        matches!(self.reason, RequiredFactorErrorReason::Expired)
    }
}
