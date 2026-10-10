//! The user information loaded from the OpenID Connect user-info endpoint.

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{ClientRegistration, OAuth2AccessTokenResponse, OAuth2UserService},
};

#[derive(Clone)]
pub struct OidcUserRequest {
    client_registration: ClientRegistration,
    access_token_response: OAuth2AccessTokenResponse,
}

impl OidcUserRequest {
    pub fn new(
        client_registration: ClientRegistration,
        access_token_response: OAuth2AccessTokenResponse,
    ) -> Self {
        Self {
            client_registration,
            access_token_response,
        }
    }
}

#[derive(Clone, Default)]
pub struct OidcUser {
    name: String,
    issuer: Option<String>,
}

#[derive(Clone, Default)]
pub struct OidcUserService;

impl OAuth2UserService<OidcUserRequest, OidcUser> for OidcUserService {
    fn load_user(&self, request: &OidcUserRequest) -> Result<OidcUser, AuthenticationError> {
        let token = request.access_token_response.access_token();
        if token.trim().is_empty() {
            return Err(AuthenticationError::with_kind(
                "Access token is empty",
                AuthenticationErrorKind::BadCredentials,
            ));
        }
        Ok(OidcUser {
            name: token.to_owned(),
            issuer: Some(request.client_registration.authorization_uri().to_owned()),
        })
    }
}
