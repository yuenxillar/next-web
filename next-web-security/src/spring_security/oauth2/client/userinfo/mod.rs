//! The user information loaded from the user-info endpoint.

use std::collections::HashMap;

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{ClientRegistration, OAuth2AccessTokenResponse, OAuth2User},
};

#[derive(Clone)]
pub struct OAuth2UserRequest {
    client_registration: ClientRegistration,
    access_token_response: OAuth2AccessTokenResponse,
}

impl OAuth2UserRequest {
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

pub trait OAuth2UserService<R, U>: Send + Sync {
    fn load_user(&self, request: &R) -> Result<U, AuthenticationError>;
}

#[derive(Clone, Default)]
pub struct DefaultOAuth2UserService;

impl OAuth2UserService<OAuth2UserRequest, OAuth2User> for DefaultOAuth2UserService {
    fn load_user(&self, request: &OAuth2UserRequest) -> Result<OAuth2User, AuthenticationError> {
        let token = request.access_token_response.access_token();
        if token.trim().is_empty() {
            return Err(AuthenticationError::with_kind(
                "Access token is empty",
                AuthenticationErrorKind::BadCredentials,
            ));
        }
        let mut attributes = HashMap::new();
        attributes.insert("access_token".into(), token.into());
        Ok(OAuth2User::new(token, attributes))
    }
}
