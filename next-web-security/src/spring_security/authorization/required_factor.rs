use std::time::Duration;

use crate::core::factor_granted_authority::FactorGrantedAuthority;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RequiredFactor {
    authority: String,
    valid_duration: Option<Duration>,
}

impl RequiredFactor {
    pub fn with_authority(authority: impl Into<String>) -> RequiredFactorBuilder {
        RequiredFactorBuilder::default().authority(authority)
    }

    pub fn builder() -> RequiredFactorBuilder {
        RequiredFactorBuilder::default()
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }

    pub fn valid_duration(&self) -> Option<Duration> {
        self.valid_duration
    }
}

#[derive(Clone, Debug, Default)]
pub struct RequiredFactorBuilder {
    authority: Option<String>,
    valid_duration: Option<Duration>,
}

impl RequiredFactorBuilder {
    pub fn authority(mut self, authority: impl Into<String>) -> Self {
        let authority = authority.into();
        assert!(!authority.trim().is_empty(), "authority cannot be empty");
        self.authority = Some(authority);
        self
    }

    pub fn authorization_code_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::AUTHORIZATION_CODE_AUTHORITY)
    }

    pub fn bearer_token_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::BEARER_AUTHORITY)
    }

    pub fn cas_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::CAS_AUTHORITY)
    }

    pub fn password_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::PASSWORD_AUTHORITY)
    }

    pub fn ott_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::OTT_AUTHORITY)
    }

    pub fn saml_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::SAML_RESPONSE_AUTHORITY)
    }

    pub fn webauthn_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::WEBAUTHN_AUTHORITY)
    }

    pub fn x509_authority(self) -> Self {
        self.authority(FactorGrantedAuthority::X509_AUTHORITY)
    }

    pub fn valid_duration(mut self, valid_duration: Duration) -> Self {
        self.valid_duration = Some(valid_duration);
        self
    }

    pub fn build(self) -> RequiredFactor {
        RequiredFactor {
            authority: self.authority.expect("authority cannot be null"),
            valid_duration: self.valid_duration,
        }
    }
}
