use std::{fmt, time::Instant};

use crate::core::GrantedAuthority;

/// A `GrantedAuthority` specifically used for indicating the factor used at time of
/// authentication.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FactorGrantedAuthority {
    authority: String,
    issued_at: Instant,
}

impl FactorGrantedAuthority {
    /// The standard authority that indicates that OAuth2 Authorization Code was used to
    /// authenticate.
    pub const AUTHORIZATION_CODE_AUTHORITY: &'static str = "FACTOR_AUTHORIZATION_CODE";

    /// The standard authority that indicates that bearer authentication was used to
    /// authenticate.
    pub const BEARER_AUTHORITY: &'static str = "FACTOR_BEARER";

    /// The standard authority that indicates that CAS was used to authenticate.
    pub const CAS_AUTHORITY: &'static str = "FACTOR_CAS";

    /// The standard authority that indicates that one time token was used to
    /// authenticate.
    pub const OTT_AUTHORITY: &'static str = "FACTOR_OTT";

    /// The standard authority that indicates that a password was used to authenticate.
    pub const PASSWORD_AUTHORITY: &'static str = "FACTOR_PASSWORD";

    /// The standard authority that indicates that SAML was used to authenticate.
    pub const SAML_RESPONSE_AUTHORITY: &'static str = "FACTOR_SAML_RESPONSE";

    /// The standard authority that indicates that WebAuthn was used to authenticate.
    pub const WEBAUTHN_AUTHORITY: &'static str = "FACTOR_WEBAUTHN";

    /// The standard authority that indicates that X509 was used to authenticate.
    pub const X509_AUTHORITY: &'static str = "FACTOR_X509";

    pub fn new(authority: impl Into<String>, issued_at: Instant) -> Self {
        Self {
            authority: authority.into(),
            issued_at,
        }
    }

    /// Creates a new FactorGrantedAuthority.Builder with the specified authority.
    pub fn with_authority(authority: impl Into<String>) -> FactorGrantedAuthorityBuilder {
        FactorGrantedAuthorityBuilder::new(authority)
    }

    /// Creates a new FactorGrantedAuthority.Builder with the specified factor which is automatically prefixed with "FACTOR_".
    pub fn with_factor(factor: impl Into<String>) -> FactorGrantedAuthorityBuilder {
        let factor = factor.into();
        assert!(!factor.trim().is_empty(), "factor cannot be empty");
        assert!(
            !factor.starts_with("FACTOR_"),
            "factor cannot start with 'FACTOR_'"
        );
        Self::with_authority(format!("FACTOR_{factor}"))
    }

    /// Shortcut for with_authority(authority).build().
    pub fn from_authority(authority: impl Into<String>) -> Self {
        Self::with_authority(authority).build()
    }

    /// Shortcut for with_factor(factor).build().
    pub fn from_factor(factor: impl Into<String>) -> Self {
        Self::with_factor(factor).build()
    }

    /// Returns the instant when this authority was issued.
    pub fn issued_at(&self) -> Instant {
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
            "FactorGrantedAuthority [authority={}, issuedAt={:?}]",
            self.authority, self.issued_at
        )
    }
}

#[derive(Clone, Debug)]
pub struct FactorGrantedAuthorityBuilder {
    authority: String,
    issued_at: Option<Instant>,
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

    /// Sets the instant when this authority was issued.
    pub fn issued_at(mut self, issued_at: Instant) -> Self {
        self.issued_at = Some(issued_at);
        self
    }

    /// Builds a new FactorGrantedAuthority.
    /// If issuedAt is not set, it defaults to Instant.now().
    pub fn build(self) -> FactorGrantedAuthority {
        FactorGrantedAuthority {
            authority: self.authority,
            issued_at: self.issued_at.unwrap_or_else(Instant::now),
        }
    }
}
