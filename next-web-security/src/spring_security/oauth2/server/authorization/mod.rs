//! The authorization server.
//!
//! The module serves the endpoints a client of an application uses to obtain a
//! token: the authorization endpoint that grants an authorization code, the
//! token endpoint that exchanges the code (or a refresh token, or the
//! credentials of the client) for an access token, the JWK set and the metadata
//! endpoints a client reads to discover the server, and the introspection and
//! revocation endpoints a resource server uses to manage a token.

pub mod authorization;
pub mod client;
pub mod client_authentication;
pub mod context;
pub mod error;
pub mod jose;
pub mod settings;
pub mod token;
pub mod web;

pub use authorization::{
    InMemoryOAuth2AuthorizationService, OAuth2Authorization, OAuth2AuthorizationService,
};
pub use client::{
    AuthorizationGrantType, ClientAuthenticationMethod, ClientSettings,
    InMemoryRegisteredClientRepository, RegisteredClient, RegisteredClientRepository,
    TokenSettings,
};
pub use context::{AuthorizationServerContext, AuthorizationServerContextBuilder};
pub use error::OAuth2Error;
pub use jose::{Jwk, JwkSet, JwkSource, SecretJwkSource};
pub use settings::{AuthorizationServerSettings, AuthorizationServerSettingsBuilder};
pub use token::{
    DelegatingOAuth2TokenGenerator, JwtGenerator, OAuth2AccessToken, OAuth2AccessTokenGenerator,
    OAuth2RefreshToken, OAuth2RefreshTokenGenerator, OAuth2Token, OAuth2TokenContext,
    OAuth2TokenGenerator, OAuth2TokenType,
};
pub use web::{
    AuthorizationEndpointFilter, AuthorizationServerMetadataEndpointFilter, JwkSetEndpointFilter,
    TokenEndpointFilter, TokenIntrospectionEndpointFilter, TokenRevocationEndpointFilter,
};
