use std::fmt;

use chrono::{DateTime, Utc};
use next_web_core::async_trait;

use crate::core::granted_authority::GrantedAuthority;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FactorGrantedAuthority {
    authority: String,
    issued_at: DateTime<Utc>,
}

impl FactorGrantedAuthority {
    pub const AUTHORIZATION_CODE_AUTHORITY: &'static str = "FACTOR_AUTHORIZATION_CODE";
    pub const BEARER_AUTHORITY: &'static str = "FACTOR_BEARER";
    pub const CAS_AUTHORITY: &'static str = "FACTOR_CAS";
    pub const OTT_AUTHORITY: &'static str = "FACTOR_OTT";
    pub const PASSWORD_AUTHORITY: &'static str = "FACTOR_PASSWORD";
    pub const SAML_RESPONSE_AUTHORITY: &'static str = "FACTOR_SAML_RESPONSE";
    pub const WEBAUTHN_AUTHORITY: &'static str = "FACTOR_WEBAUTHN";
    pub const X509_AUTHORITY: &'static str = "FACTOR_X509";

    pub fn with_authority(authority: impl Into<String>) -> FactorGrantedAuthorityBuilder {
        FactorGrantedAuthorityBuilder::new(authority)
    }

    pub fn with_factor(factor: impl Into<String>) -> FactorGrantedAuthorityBuilder {
        let factor = factor.into();
        assert!(!factor.trim().is_empty(), "factor cannot be empty");
        assert!(
            !factor.starts_with("FACTOR_"),
            "factor cannot start with 'FACTOR_'"
        );
        Self::with_authority(format!("FACTOR_{factor}"))
    }

    pub fn from_authority(authority: impl Into<String>) -> Self {
        Self::with_authority(authority).build()
    }

    pub fn from_factor(factor: impl Into<String>) -> Self {
        Self::with_factor(factor).build()
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }

    pub fn issued_at(&self) -> DateTime<Utc> {
        self.issued_at
    }
}

impl GrantedAuthority for FactorGrantedAuthority {
    fn authority(&self) -> Option<&str> {
        Some(self.authority.as_str())
    }
}

impl fmt::Display for FactorGrantedAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FactorGrantedAuthority [authority={}, issuedAt={}]",
            self.authority, self.issued_at
        )
    }
}

#[derive(Clone, Debug)]
pub struct FactorGrantedAuthorityBuilder {
    authority: String,
    issued_at: Option<DateTime<Utc>>,
}

impl FactorGrantedAuthorityBuilder {
    fn new(authority: impl Into<String>) -> Self {
        let authority = authority.into();
        assert!(
            !authority.trim().is_empty(),
            "A granted authority textual representation is required"
        );
        Self {
            authority,
            issued_at: None,
        }
    }

    pub fn issued_at(mut self, issued_at: DateTime<Utc>) -> Self {
        self.issued_at = Some(issued_at);
        self
    }

    pub fn build(self) -> FactorGrantedAuthority {
        FactorGrantedAuthority {
            authority: self.authority,
            issued_at: self.issued_at.unwrap_or_else(Utc::now),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::FactorGrantedAuthority;

    #[test]
    fn factor_authority_prefixes_factor_and_keeps_issued_at() {
        let issued_at = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let authority = FactorGrantedAuthority::with_factor("PASSWORD")
            .issued_at(issued_at)
            .build();

        assert_eq!(authority.authority(), "FACTOR_PASSWORD");
        assert_eq!(authority.issued_at(), issued_at);
    }

    #[test]
    #[should_panic(expected = "factor cannot start with 'FACTOR_'")]
    fn factor_authority_rejects_prefixed_factor_name() {
        let _ = FactorGrantedAuthority::from_factor("FACTOR_PASSWORD");
    }
}
