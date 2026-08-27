use crate::authorization::required_factor::RequiredFactor;

/// An error when the requirements of RequiredFactor are not met.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RequiredFactorError {
    required_factor: RequiredFactor,
    reason: RequiredFactorErrorReason,
}

impl RequiredFactorError {
    /// Creates a new `RequiredFactorError` with the given reason.
    pub fn new(
        required_factor: RequiredFactor,
        reason: RequiredFactorErrorReason,
    ) -> Result<Self, &'static str> {
        if reason == RequiredFactorErrorReason::Expired
            && required_factor.valid_duration().is_none()
        {
            return Err("If expired, RequiredFactor.valid_duration() must not be none ");
        }
        Ok(Self {
            required_factor,
            reason,
        })
    }

    pub fn create_missing(required_factor: RequiredFactor) -> Result<Self, &'static str> {
        Self::new(required_factor, RequiredFactorErrorReason::Missing)
    }

    pub fn create_expired(required_factor: RequiredFactor) -> Result<Self, &'static str> {
        Self::new(required_factor, RequiredFactorErrorReason::Expired)
    }

    pub fn required_factor(&self) -> &RequiredFactor {
        &self.required_factor
    }

    pub fn reason(&self) -> &RequiredFactorErrorReason {
        &self.reason
    }

    /// True if not is_missing() but was older than the RequiredFactor.valid_duration().
    pub fn is_expired(&self) -> bool {
        matches!(self.reason, RequiredFactorErrorReason::Expired)
    }

    /// True if no FactorGrantedAuthority.authority() on the security.core.Authentication matched RequiredFactor.authority().
    pub fn is_missing(&self) -> bool {
        matches!(self.reason, RequiredFactorErrorReason::Missing)
    }
}

/// The reason that the error occurred.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum RequiredFactorErrorReason {
    /// The authority was missing.
    Missing,

    /// The authority was considered expired.
    Expired,
}
