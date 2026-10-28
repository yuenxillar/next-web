use std::time::Duration;

use crate::core::authority::FactorGrantedAuthority;

/// The requirements for an GrantedAuthority to be considered a valid factor.
/// If the authority() is specified, then it must match GrantedAuthority.authority()
/// If valid_duration() is specified, the matching GrantedAuthority must be of type
/// FactorGrantedAuthority and FactorGrantedAuthority.issued_at() must be such that it is
/// not considered expired when compared to valid_duration().
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RequiredFactor {
    authority: String,
    valid_duration: Option<Duration>,
}

impl RequiredFactor {
    /// Creates a new instance.
    pub fn new(authority: impl Into<String>, valid_duration: Option<Duration>) -> Self {
        Self {
            authority: authority.into(),
            valid_duration,
        }
    }

    /// Creates a RequiredFactorBuilder with the specified authority.
    pub fn with_authority(authority: impl Into<String>) -> RequiredFactorBuilder {
        RequiredFactorBuilder::default().authority(authority)
    }

    /// Returns the authority required for this factor.
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// Returns the valid duration for this factor.
    pub fn valid_duration(&self) -> Option<Duration> {
        self.valid_duration
    }

    /// Creates a new RequiredFactorBuilder
    pub fn builder() -> RequiredFactorBuilder {
        RequiredFactorBuilder::default()
    }
}

/// A builder for RequiredFactor.
#[derive(Clone, Debug, Default)]
pub struct RequiredFactorBuilder {
    authority: Option<String>,
    valid_duration: Option<Duration>,
}

impl RequiredFactorBuilder {
    /// Sets the required authority.
    pub fn authority(mut self, authority: impl Into<String>) -> Self {
        let authority = authority.into();
        assert!(!authority.trim().is_empty(), "authority cannot be empty");
        self.authority = Some(authority);
        self
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.AUTHORIZATION_CODE_AUTHORITY.
    pub fn authorization_code_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::AUTHORIZATION_CODE_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.BEARER_AUTHORITY.
    pub fn bearer_token_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::BEARER_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.CAS_AUTHORITY.
    pub fn cas_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::CAS_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.PASSWORD_AUTHORITY.
    pub fn password_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::PASSWORD_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.OTT_AUTHORITY.
    pub fn ott_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::OTT_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.SAML_RESPONSE_AUTHORITY.
    pub fn saml_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::SAML_RESPONSE_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.WEBAUTHN_AUTHORITY.
    pub fn webauthn_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::WEBAUTHN_AUTHORITY)
    }

    /// A convenience method for invoking authority(String) with FactorGrantedAuthority.X509_AUTHORITY.
    pub fn x509_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::X509_AUTHORITY)
    }

    /// Sets the optional Duration of time that the RequiredFactor is valid for.
    pub fn valid_duration(mut self, valid_duration: Duration) -> Self {
        self.valid_duration = Some(valid_duration);
        self
    }

    /// Builds a new instance.
    pub fn build(self) -> RequiredFactor {
        RequiredFactor {
            authority: self.authority.expect("authority cannot be none"),
            valid_duration: self.valid_duration,
        }
    }
}
