use crate::authorization::required_factor::RequiredFactor;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum RequiredFactorErrorKind {
    Missing,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RequiredFactorError {
    required_factor: RequiredFactor,
    kind: RequiredFactorErrorKind,
}

impl RequiredFactorError {
    pub fn create_missing(required_factor: RequiredFactor) -> Self {
        Self {
            required_factor,
            kind: RequiredFactorErrorKind::Missing,
        }
    }

    pub fn create_expired(required_factor: RequiredFactor) -> Self {
        Self {
            required_factor,
            kind: RequiredFactorErrorKind::Expired,
        }
    }

    pub fn required_factor(&self) -> &RequiredFactor {
        &self.required_factor
    }

    pub fn kind(&self) -> &RequiredFactorErrorKind {
        &self.kind
    }
}
