use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        authority::mapping::GrantedAuthoritiesMapper,
        Authentication, {AuthenticationError, AuthenticationErrorKind},
    },
    oauth2::{
        OAuth2AccessToken, OAuth2AccessTokenResponseClient, OAuth2AuthorizationCodeGrantRequest,
        OAuth2AuthorizationExchange, OAuth2LoginAuthenticationToken, OAuth2User, OAuth2UserRequest,
        OAuth2UserService,
    },
    web::authentication::AuthPrincipal,
};

#[derive(Clone)]
pub struct OAuth2LoginAuthenticationProvider {
    access_token_response_client:
        Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>,
    user_service: Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>,
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
        let token_response = self.access_token_response_client.get_token_response(
            &OAuth2AuthorizationCodeGrantRequest::new(registration.clone(), exchange.clone()),
        )?;
        let access_token = OAuth2AccessToken::from_response(&token_response);
        let user = self.user_service.load_user(&OAuth2UserRequest::new(
            registration.clone(),
            token_response,
        ))?;
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
