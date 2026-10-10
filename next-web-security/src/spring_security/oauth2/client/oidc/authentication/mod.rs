//! The OpenID Connect authentication support of the OAuth 2.0 client.

use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{Authentication, AuthenticationError, AuthenticationErrorKind},
    oauth2::{OAuth2AuthorizationExchange, OAuth2LoginAuthenticationToken},
};

#[derive(Clone, Default)]
pub struct OidcAuthenticationRequestChecker;

#[async_trait]
impl AuthenticationProvider for OidcAuthenticationRequestChecker {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OAuth2LoginAuthenticationToken>() {
            return Ok(None);
        }
        let exchange = match authentication.credentials().and_then(|credentials| {
            credentials
                .as_any()
                .downcast_ref::<OAuth2AuthorizationExchange>()
        }) {
            Some(exchange) => exchange,
            None => return Ok(None),
        };
        if exchange
            .authorization_request()
            .scopes()
            .iter()
            .any(|scope| scope == "openid")
        {
            return Err(AuthenticationError::with_kind(
                "An OpenID Connect Authentication Provider has not been configured. Check to ensure OIDC/JWT support is available.",
                AuthenticationErrorKind::AuthenticationService,
            ));
        }
        Ok(None)
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OAuth2LoginAuthenticationToken>()
    }
}
