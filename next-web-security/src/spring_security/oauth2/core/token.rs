//! The OAuth 2.0 access token and refresh token.

use std::fmt;

use super::endpoint::OAuth2AccessTokenResponse;

/// An OAuth 2.0 Access Token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AccessToken {
    token_type: String,
    scopes: Vec<String>,
    expires_at: Option<u64>,
    value: String,
}

impl OAuth2AccessToken {
    /// The default token type used by most providers.
    pub const BEARER_TOKEN_TYPE: &'static str = "Bearer";

    /// Creates a bearer access token with the given value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            token_type: Self::BEARER_TOKEN_TYPE.to_string(),
            scopes: Vec::new(),
            expires_at: None,
            value: value.into(),
        }
    }

    /// Creates an access token with full details.
    pub fn with_details(
        value: impl Into<String>,
        token_type: impl Into<String>,
        scopes: impl IntoIterator<Item = impl Into<String>>,
        expires_at: Option<u64>,
    ) -> Self {
        Self {
            token_type: token_type.into(),
            scopes: scopes.into_iter().map(Into::into).collect(),
            expires_at,
            value: value.into(),
        }
    }

    /// Builds an access token from an `OAuth2AccessTokenResponse`.
    pub fn from_response(response: &OAuth2AccessTokenResponse) -> Self {
        Self::new(response.access_token())
    }

    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn expires_at(&self) -> Option<u64> {
        self.expires_at
    }

    pub fn token_value(&self) -> &str {
        &self.value
    }
}

/// An OAuth 2.0 Refresh Token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2RefreshToken {
    value: String,
    expires_at: Option<u64>,
}

impl OAuth2RefreshToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            expires_at: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn expires_at(&self) -> Option<u64> {
        self.expires_at
    }
}

impl crate::web::authentication::Identity for OAuth2AccessToken {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl fmt::Display for OAuth2AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}
