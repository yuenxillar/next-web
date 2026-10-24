//! The OAuth 2.0 client support.
//!
//! The module layout follows the OAuth 2.0 client package: the registrations,
//! the endpoint messages, the authentication tokens and providers, the user
//! information services, the OpenID Connect support, and the web integration
//! each live in their own submodule.

mod authorized_client;
mod authorized_client_service;
mod client_authorization_required_error;

pub mod authentication;
pub mod endpoint;
pub mod oidc;
#[cfg(feature = "oauth2-resource-server")]
pub mod oidc_back_channel_logout;
#[cfg(feature = "oauth2-resource-server")]
pub mod oidc_session_registry;
pub mod registration;
pub mod userinfo;
pub mod web;

pub use authorized_client::{
    AuthorizationCodeOAuth2AuthorizedClientProvider, DefaultOAuth2AuthorizedClientManager,
    DelegatingOAuth2AuthorizedClientProvider, OAuth2AuthorizationContext, OAuth2AuthorizeError,
    OAuth2AuthorizeRequest, OAuth2AuthorizedClient, OAuth2AuthorizedClientManager,
    OAuth2AuthorizedClientProvider, REQUEST_SCOPE_ATTRIBUTE_NAME,
};
pub use authorized_client_service::{
    NullOAuth2AuthorizedClientService, OAuth2AuthorizedClientService,
};
pub use client_authorization_required_error::ClientAuthorizationRequiredError;
#[cfg(feature = "oauth2-resource-server")]
pub use oidc_back_channel_logout::{
    EitherLogoutHandler, OAuth2ClientConfigurerUtils, OidcBackChannelLogoutAuthentication,
    OidcBackChannelLogoutAuthenticationProvider, OidcBackChannelLogoutFilter,
    OidcBackChannelLogoutHandler, OidcLogoutAuthenticationConverter, OidcLogoutAuthenticationToken,
    OidcLogoutToken,
};
#[cfg(feature = "oauth2-resource-server")]
pub use oidc_session_registry::{
    InMemoryOidcSessionRegistry, OidcSessionInformation, OidcSessionRegistry,
};

pub use authentication::{
    OAuth2AuthorizationCodeAuthenticationProvider, OAuth2AuthorizationCodeAuthenticationToken,
    OAuth2AuthorizationCodeAuthenticationTokenBuilder, OAuth2LoginAuthenticationProvider,
    OAuth2LoginAuthenticationToken, OAuth2LoginAuthenticationTokenBuilder,
};
pub use endpoint::{
    OAuth2AccessTokenResponseClient, OAuth2AuthorizationCodeGrantRequest,
    RestClientAuthorizationCodeTokenResponseClient,
};
pub use oidc::authentication::OidcAuthenticationRequestChecker;
pub use oidc::authentication::{
    access_token_hash_of, DefaultOidcIdTokenValidator, OidcIdToken, OidcIdTokenValidator,
    OidcLoginAuthenticationProvider,
};
pub use oidc::userinfo::{OidcUser, OidcUserAuthority, OidcUserRequest, OidcUserService};
pub use registration::{
    ClientRegistration, ClientRegistrationRepository, InMemoryClientRegistrationRepository,
};
pub use userinfo::{DefaultOAuth2UserService, OAuth2UserRequest, OAuth2UserService};
