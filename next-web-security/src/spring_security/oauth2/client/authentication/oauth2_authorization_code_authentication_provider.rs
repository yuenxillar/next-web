use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        authority::mapping::GrantedAuthoritiesMapper,
        Authentication, GrantedAuthority, {AuthenticationError, AuthenticationErrorKind},
    },
    oauth2::{
        OAuth2AccessToken, OAuth2AccessTokenResponseClient,
        OAuth2AuthorizationCodeAuthenticationToken, OAuth2AuthorizationCodeGrantRequest,
        OAuth2AuthorizedClient,
    },
};

use super::oauth2_authorization_code_request_data::OAuth2AuthorizationCodeRequestData;

/// An `AuthenticationProvider` for the OAuth 2.0 Authorization Code Grant.
///
/// Delegates the token-endpoint exchange to an `OAuth2AccessTokenResponseClient`
/// and, on success, returns an authenticated
/// `OAuth2AuthorizationCodeAuthenticationToken` carrying the issued access token
/// and authorized client.
#[derive(Clone)]
pub struct OAuth2AuthorizationCodeAuthenticationProvider {
    access_token_response_client:
        Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>,
    authorities_mapper: Option<Arc<dyn GrantedAuthoritiesMapper>>,
}

impl OAuth2AuthorizationCodeAuthenticationProvider {
    pub fn new(
        access_token_response_client: Arc<
            dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>,
        >,
    ) -> Self {
        Self {
            access_token_response_client,
            authorities_mapper: None,
        }
    }

    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = Some(authorities_mapper);
    }
}

#[async_trait]
impl AuthenticationProvider for OAuth2AuthorizationCodeAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OAuth2AuthorizationCodeAuthenticationToken>() {
            return Ok(None);
        }
        let request_data = authentication
            .credentials()
            .and_then(|credentials| {
                credentials
                    .as_any()
                    .downcast_ref::<OAuth2AuthorizationCodeRequestData>()
            })
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "OAuth2 authorization code request data is missing",
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        let exchange = request_data.authorization_exchange.clone();
        if let Some(error_code) = exchange.authorization_response().error_code() {
            return Err(AuthenticationError::with_kind(
                format!(
                    "OAuth2 authorization response contained error: {}",
                    error_code
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let client_registration = request_data.client_registration.clone();
        let grant_request =
            OAuth2AuthorizationCodeGrantRequest::new(client_registration.clone(), exchange);
        let response = self
            .access_token_response_client
            .get_token_response(&grant_request)
            .await?;
        let access_token = OAuth2AccessToken::from_response(&response);
        let principal_name = client_registration.registration_id().to_string();
        let authorized_client = OAuth2AuthorizedClient::new(
            client_registration.clone(),
            principal_name.clone(),
            access_token.clone(),
        );

        let authorities: Vec<Arc<dyn GrantedAuthority>> = self
            .authorities_mapper
            .as_ref()
            .map(|_mapper| Vec::new())
            .unwrap_or_default();

        let authenticated = OAuth2AuthorizationCodeAuthenticationToken::authenticated(
            principal_name,
            client_registration,
            grant_request.authorization_exchange().clone(),
            access_token,
            authorized_client,
            authorities,
        );
        Ok(Some(Arc::new(authenticated)))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OAuth2AuthorizationCodeAuthenticationToken>()
    }
}
