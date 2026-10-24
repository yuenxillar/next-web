//! The core OAuth 2.0 types shared by the client and the resource server.
//!
//! The module layout follows the OAuth 2.0 core package: the authorization
//! grant type lives here, and the tokens, the endpoint messages, and the user
//! live in their own submodules.

pub mod endpoint;
pub mod token;
pub mod user;

pub use endpoint::{
    OAuth2AccessTokenResponse, OAuth2AuthorizationExchange, OAuth2AuthorizationRequest,
    OAuth2AuthorizationResponse,
};
pub use token::{OAuth2AccessToken, OAuth2RefreshToken};
pub use user::OAuth2User;

/// The authorization grant type of an OAuth 2.0 client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationGrantType {
    /// The authorization code grant.
    AuthorizationCode,
    /// A grant type that this crate does not model in more detail.
    Other(String),
}

impl AuthorizationGrantType {
    /// Returns the authorization code grant type.
    pub fn authorization_code() -> Self {
        Self::AuthorizationCode
    }
}
