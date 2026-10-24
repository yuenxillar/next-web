use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        authority::mapping::GrantedAuthoritiesMapper,
        Authentication, {AuthenticationError, AuthenticationErrorKind},
    },
    oauth2::{
        DefaultOidcIdTokenValidator, OAuth2AccessToken, OAuth2AccessTokenResponseClient,
        OAuth2AuthorizationCodeGrantRequest, OAuth2AuthorizationExchange,
        OAuth2LoginAuthenticationToken, OAuth2User, OAuth2UserRequest, OAuth2UserService,
        OidcIdTokenValidator,
    },
    web::authentication::AuthPrincipal,
};

#[derive(Clone)]
pub struct OAuth2LoginAuthenticationProvider {
    access_token_response_client:
        Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>,
    user_service: Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>,
    oidc_id_token_validator: Arc<dyn OidcIdTokenValidator>,
    authorities_mapper: Option<Arc<dyn GrantedAuthoritiesMapper>>,
}

impl OAuth2LoginAuthenticationProvider {
    pub fn new(
        access_token_response_client: Arc<
            dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>,
        >,
        user_service: Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>,
    ) -> Self {
        Self {
            access_token_response_client,
            user_service,
            oidc_id_token_validator: Arc::new(DefaultOidcIdTokenValidator::default()),
            authorities_mapper: None,
        }
    }

    /// Sets the validator of the identifier token of an OpenID Connect grant.
    ///
    /// # Arguments
    ///
    /// * `oidc_id_token_validator` - The validator of the identifier token.
    pub fn set_oidc_id_token_validator(
        &mut self,
        oidc_id_token_validator: Arc<dyn OidcIdTokenValidator>,
    ) {
        self.oidc_id_token_validator = oidc_id_token_validator;
    }

    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = Some(authorities_mapper);
    }
}

#[async_trait]
impl AuthenticationProvider for OAuth2LoginAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OAuth2LoginAuthenticationToken>() {
            return Ok(None);
        }
        let exchange = authentication
            .credentials()
            .and_then(|credentials| {
                credentials
                    .as_any()
                    .downcast_ref::<OAuth2AuthorizationExchange>()
            })
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "OAuth2 authorization exchange is missing",
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        if let Some(error_code) = exchange.authorization_response().error_code() {
            return Err(AuthenticationError::with_kind(
                format!(
                    "OAuth2 authorization response contained error: {}",
                    error_code
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }
        let registration = (authentication.as_ref() as &dyn std::any::Any)
            .downcast_ref::<OAuth2LoginAuthenticationToken>()
            .ok_or_else(|| AuthenticationError::new("Invalid OAuth2 login token"))?
            .client_registration()
            .clone();
        let token_response = self
            .access_token_response_client
            .get_token_response(&OAuth2AuthorizationCodeGrantRequest::new(
                registration.clone(),
                exchange.clone(),
            ))
            .await?;

        // A grant that asks for the `openid` scope receives an identifier token,
        // and the client trusts the identity it names only after the token is
        // verified: a token the client cannot verify is refused.
        if registration.scopes().iter().any(|scope| scope == "openid") {
            let id_token = token_response.id_token().ok_or_else(|| {
                AuthenticationError::with_kind(
                    "The token response of an OpenID Connect grant has no identifier token",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;
            self.oidc_id_token_validator
                .validate(
                    &registration,
                    id_token,
                    exchange.authorization_request().nonce(),
                    Some(token_response.access_token()),
                )
                .await?;
        }

        let access_token = OAuth2AccessToken::from_response(&token_response);
        let user = self
            .user_service
            .load_user(&OAuth2UserRequest::new(
                registration.clone(),
                token_response,
            ))
            .await?;
        let principal: AuthPrincipal = Arc::new(user.name().to_owned());
        let authorities = self
            .authorities_mapper
            .as_ref()
            .map(|m| m.map_authorities(&[]))
            .unwrap_or_default();
        Ok(Some(Arc::new(
            OAuth2LoginAuthenticationToken::authenticated(
                principal,
                registration,
                exchange.clone(),
                access_token,
                authorities,
            ),
        )))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OAuth2LoginAuthenticationToken>()
    }
}
