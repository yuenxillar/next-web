//! The OAuth 2.0 support, split into the core types, the client, and the
//! resource server.
//!
//! The module keeps the flat surface the rest of the crate imports from: every
//! type of the packages below is re-exported here as well.

#[cfg(feature = "oauth2-client")]
pub mod client;
pub mod core;
#[cfg(any(
    feature = "oauth2-resource-server",
    feature = "oauth2-authorization-server"
))]
pub mod server;

// The keys of an authorization server are shared by the server that signs the
// tokens and the resource server that verifies them, and the client uses the
// encodings of the module for the proof key of the authorization code grant.
#[cfg(any(
    feature = "oauth2-resource-server",
    feature = "oauth2-authorization-server",
    feature = "oauth2-client"
))]
pub mod jose;

use next_web_core::http::HttpRequest;

#[cfg(feature = "oauth2-client")]
pub use client::web::{
    AuthenticatedPrincipalOAuth2AuthorizedClientRepository, AuthorizationRequestRepository,
    DefaultOAuth2AuthorizationRequestResolver, HttpSessionOAuth2AuthorizationRequestRepository,
    OAuth2AuthorizationRequestResolver, OAuth2AuthorizedClientRepository,
};
#[cfg(feature = "oauth2-client")]
pub use client::{
    AuthorizationCodeOAuth2AuthorizedClientProvider, ClientAuthorizationRequiredError,
    ClientRegistration, ClientRegistrationRepository, DefaultOAuth2AuthorizedClientManager,
    DefaultOAuth2UserService, DefaultOidcIdTokenValidator,
    DelegatingOAuth2AuthorizedClientProvider, InMemoryClientRegistrationRepository,
    NullOAuth2AuthorizedClientService, OAuth2AccessTokenResponseClient,
    OAuth2AuthorizationCodeAuthenticationProvider, OAuth2AuthorizationCodeAuthenticationToken,
    OAuth2AuthorizationCodeAuthenticationTokenBuilder, OAuth2AuthorizationCodeGrantRequest,
    OAuth2AuthorizationContext, OAuth2AuthorizeError, OAuth2AuthorizeRequest,
    OAuth2AuthorizedClient, OAuth2AuthorizedClientManager, OAuth2AuthorizedClientProvider,
    OAuth2AuthorizedClientService, OAuth2LoginAuthenticationProvider,
    OAuth2LoginAuthenticationToken, OAuth2LoginAuthenticationTokenBuilder, OAuth2UserRequest,
    OAuth2UserService, OidcAuthenticationRequestChecker, OidcIdToken, OidcIdTokenValidator,
    OidcLoginAuthenticationProvider, OidcUser, OidcUserAuthority, OidcUserRequest, OidcUserService,
    RestClientAuthorizationCodeTokenResponseClient, REQUEST_SCOPE_ATTRIBUTE_NAME,
};
pub use core::{
    endpoint::{
        OAuth2AccessTokenResponse, OAuth2AuthorizationExchange, OAuth2AuthorizationRequest,
        OAuth2AuthorizationResponse,
    },
    token::{OAuth2AccessToken, OAuth2RefreshToken},
    user::OAuth2User,
    AuthorizationGrantType,
};

/// Reads a query parameter from the request, returning a decoded owned value.
pub fn request_parameter(request: &dyn HttpRequest, name: &str) -> Option<String> {
    request.query()?.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = decode(parts.next().unwrap_or_default());
        if key != name {
            return None;
        }
        Some(decode(parts.next().unwrap_or_default()))
    })
}

fn decode(value: &str) -> String {
    urlencoding::decode(value)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

#[cfg(all(test, feature = "oauth2-client"))]
mod tests {
    use axum::{body::Body, extract::Request as AxumRequest};

    use next_web_core::http::HttpRequest;

    use std::sync::Arc;

    use crate::core::AuthenticationErrorKind;

    use super::*;

    fn client_registration(
        registration_id: &str,
        grant_type: AuthorizationGrantType,
    ) -> ClientRegistration {
        ClientRegistration::new(
            registration_id,
            "Test Client",
            "https://example.com/login/oauth/authorize",
        )
        .set_client_id("client-id")
        .set_authorization_grant_type(grant_type)
    }

    fn resolver() -> DefaultOAuth2AuthorizationRequestResolver {
        DefaultOAuth2AuthorizationRequestResolver::new(
            Arc::new(InMemoryClientRegistrationRepository::new([
                client_registration("registration-id", AuthorizationGrantType::AuthorizationCode),
                client_registration(
                    "client-credentials",
                    AuthorizationGrantType::Other("client_credentials".to_string()),
                ),
            ])),
            "/oauth2/authorization",
        )
    }

    fn get_request(path: &str) -> AxumRequest {
        let mut request = AxumRequest::builder()
            .uri(path)
            .body(Body::empty())
            .unwrap();
        request.ready();
        request
    }

    #[test]
    fn resolve_returns_none_when_not_authorization_request_uri() {
        let resolver = resolver();
        let request = get_request("/path");
        assert!(resolver.resolve(&request).unwrap().is_none());
    }

    #[test]
    fn resolve_errors_when_registration_id_is_unknown() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/missing");
        let error = resolver.resolve(&request).unwrap_err();
        assert_eq!(
            error.kind(),
            AuthenticationErrorKind::InvalidClientRegistrationId
        );
        assert_eq!(
            error.message(),
            "Invalid Client Registration with Id: missing"
        );
    }

    #[test]
    fn resolve_returns_authorization_request_for_authorization_code_client() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/registration-id");
        let authorization_request = resolver.resolve(&request).unwrap().unwrap();
        assert_eq!(authorization_request.registration_id(), "registration-id");
        assert_eq!(
            authorization_request.grant_type(),
            &AuthorizationGrantType::AuthorizationCode
        );
    }

    #[test]
    fn resolve_returns_none_for_non_authorization_code_client() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/client-credentials");
        assert!(resolver.resolve(&request).unwrap().is_none());
    }

    #[test]
    fn resolve_with_registration_id_uses_provided_id() {
        let resolver = resolver();
        // The request path is ignored by resolve_with_registration_id.
        let request = get_request("/path");
        let authorization_request = resolver
            .resolve_with_registration_id(&request, "registration-id")
            .unwrap()
            .unwrap();
        assert_eq!(authorization_request.registration_id(), "registration-id");
    }

    #[test]
    fn an_openid_authorization_request_carries_a_nonce() {
        let resolver = DefaultOAuth2AuthorizationRequestResolver::new(
            Arc::new(InMemoryClientRegistrationRepository::new([
                client_registration("registration-id", AuthorizationGrantType::AuthorizationCode)
                    .set_scopes(["openid", "read"]),
            ])),
            "/oauth2/authorization",
        );
        let request = get_request("/oauth2/authorization/registration-id");

        let authorization_request = resolver.resolve(&request).unwrap().unwrap();

        let nonce = authorization_request
            .nonce()
            .expect("an OpenID Connect request carries a nonce");
        assert!(
            authorization_request
                .authorization_request_uri()
                .contains(&format!("nonce={nonce}")),
            "the authorization endpoint is asked with the nonce"
        );
    }

    #[test]
    fn an_authorization_request_without_openid_carries_no_nonce() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/registration-id");

        let authorization_request = resolver.resolve(&request).unwrap().unwrap();

        assert_eq!(authorization_request.nonce(), None);
        assert!(!authorization_request
            .authorization_request_uri()
            .contains("nonce="));
    }
}
