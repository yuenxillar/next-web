//! The messages sent to the token endpoint and the client that sends them.

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{ClientRegistration, OAuth2AccessTokenResponse, OAuth2AuthorizationExchange},
};

#[derive(Clone)]
pub struct OAuth2AuthorizationCodeGrantRequest {
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
}

impl OAuth2AuthorizationCodeGrantRequest {
    pub fn new(
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
    ) -> Self {
        Self {
            client_registration,
            authorization_exchange,
        }
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    pub fn authorization_exchange(&self) -> &OAuth2AuthorizationExchange {
        &self.authorization_exchange
    }
}

pub trait OAuth2AccessTokenResponseClient<R>: Send + Sync {
    fn get_token_response(
        &self,
        request: &R,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError>;
}

#[derive(Clone, Default)]
pub struct RestClientAuthorizationCodeTokenResponseClient;

impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>
    for RestClientAuthorizationCodeTokenResponseClient
{
    fn get_token_response(
        &self,
        request: &OAuth2AuthorizationCodeGrantRequest,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
        let code = request
            .authorization_exchange()
            .authorization_response()
            .code()
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "Authorization code is missing",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;
        Ok(OAuth2AccessTokenResponse::new(code))
    }
}
