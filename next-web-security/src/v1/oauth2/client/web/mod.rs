//! The web integration of the OAuth 2.0 client.
//!
//! The filters that start the authorization request of a client and that
//! complete the login of a client live here, together with the types of the web
//! package they are configured with.

mod authorization_request_repository;
mod oauth2_authorization_code_grant_filter;
mod oauth2_authorization_request_redirect_filter;
mod oauth2_authorization_request_resolver;
mod oauth2_authorized_client_repository;
mod oauth2_login_authentication_filter;

pub use authorization_request_repository::{
    AuthorizationRequestRepository, HttpSessionOAuth2AuthorizationRequestRepository,
};
pub use oauth2_authorization_code_grant_filter::{
    OAuth2AuthorizationCodeGrantFilter, DEFAULT_AUTHORIZATION_RESPONSE_BASE_URI,
};
pub use oauth2_authorization_request_redirect_filter::OAuth2AuthorizationRequestRedirectFilter;
pub use oauth2_authorization_request_resolver::{
    extract_registration_id, DefaultOAuth2AuthorizationRequestResolver,
    OAuth2AuthorizationRequestResolver,
};
pub use oauth2_authorized_client_repository::{
    AuthenticatedPrincipalOAuth2AuthorizedClientRepository, OAuth2AuthorizedClientRepository,
};
pub use oauth2_login_authentication_filter::{
    OAuth2LoginAuthenticationFilter, OAuth2LoginRequestMatcher,
};

// The manager is reachable from the web package as well, because a filter that
// has to obtain a token is configured with it.
pub use crate::oauth2::client::DefaultOAuth2AuthorizedClientManager;
