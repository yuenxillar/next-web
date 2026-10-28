//! The authentication provider of the OpenID Connect login.

use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        authority::mapping::GrantedAuthoritiesMapper, Authentication, AuthenticationError,
        AuthenticationErrorKind,
    },
    oauth2::{
        DefaultOidcIdTokenValidator, OAuth2AccessToken, OAuth2AccessTokenResponseClient,
        OAuth2AuthorizationCodeGrantRequest, OAuth2LoginAuthenticationToken, OAuth2UserService,
        OidcIdTokenValidator, OidcUser, OidcUserRequest,
    },
    web::authentication::AuthPrincipal,
};

/// Authenticates the OpenID Connect login of a client.
///
/// The provider handles the grants that ask for the `openid` scope, and it
/// leaves every other grant to the provider of the plain OAuth 2.0 login. The
/// identifier token of the grant is validated before the user of the grant is
/// loaded, so the user that becomes the principal of the authentication is the
/// user the provider named, and not the user of a token the client cannot
/// verify.
#[derive(Clone)]
pub struct OidcLoginAuthenticationProvider {
    access_token_response_client:
        Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>,
    user_service: Arc<dyn OAuth2UserService<OidcUserRequest, OidcUser>>,
    id_token_validator: Arc<dyn OidcIdTokenValidator>,
    authorities_mapper: Option<Arc<dyn GrantedAuthoritiesMapper>>,
}

impl OidcLoginAuthenticationProvider {
    /// Creates the provider of the OpenID Connect login.
    ///
    /// # Arguments
    ///
    /// * `access_token_response_client` - The client that exchanges the code
    ///   of the grant for a token at the token endpoint of the provider.
    /// * `user_service` - The service that loads the user of the grant.
    pub fn new(
        access_token_response_client: Arc<
            dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>,
        >,
        user_service: Arc<dyn OAuth2UserService<OidcUserRequest, OidcUser>>,
    ) -> Self {
        Self {
            access_token_response_client,
            user_service,
            id_token_validator: Arc::new(DefaultOidcIdTokenValidator::default()),
            authorities_mapper: None,
        }
    }

    /// Sets the validator of the identifier token of the grant.
    ///
    /// # Arguments
    ///
    /// * `id_token_validator` - The validator of the identifier token.
    pub fn set_oidc_id_token_validator(
        &mut self,
        id_token_validator: Arc<dyn OidcIdTokenValidator>,
    ) {
        self.id_token_validator = id_token_validator;
    }

    /// Sets the mapper of the authorities of the user.
    ///
    /// # Arguments
    ///
    /// * `authorities_mapper` - The mapper of the authorities.
    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = Some(authorities_mapper);
    }
}

#[async_trait]
impl AuthenticationProvider for OidcLoginAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OAuth2LoginAuthenticationToken>() {
            return Ok(None);
        }

        let as_any: &dyn std::any::Any = authentication.as_ref();
        let token = as_any
            .downcast_ref::<OAuth2LoginAuthenticationToken>()
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "The authentication is not an OAuth2 login",
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        let registration = token.client_registration().clone();

        // A grant that does not ask for the `openid` scope is a plain OAuth 2.0
        // login, which the provider of the plain login authenticates.
        if !registration.scopes().iter().any(|scope| scope == "openid") {
            return Ok(None);
        }

        let exchange = token.authorization_exchange().clone();
        if let Some(error_code) = exchange.authorization_response().error_code() {
            return Err(AuthenticationError::with_kind(
                format!("The authorization response of the provider reported {error_code}"),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let token_response = self
            .access_token_response_client
            .get_token_response(&OAuth2AuthorizationCodeGrantRequest::new(
                registration.clone(),
                exchange.clone(),
            ))
            .await?;

        // The identity of an OpenID Connect grant is named by the identifier
        // token, so the token is required and it has to be verifiable.
        let id_token = token_response.id_token().ok_or_else(|| {
            AuthenticationError::with_kind(
                "The token response of an OpenID Connect grant has no identifier token",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;
        let id_token = self
            .id_token_validator
            .validate(
                &registration,
                id_token,
                exchange.authorization_request().nonce(),
                Some(token_response.access_token()),
            )
            .await?;

        let access_token = OAuth2AccessToken::from_response(&token_response);
        let user = self
            .user_service
            .load_user(
                &OidcUserRequest::new(registration.clone(), token_response).set_id_token(id_token),
            )
            .await?;
        let principal: AuthPrincipal = Arc::new(user.name().to_string());
        // The authorities of the user are the authorities of the grant, and the
        // mapper of the application is applied to them the way it is applied to
        // the authorities of the plain login.
        let authorities = self
            .authorities_mapper
            .as_ref()
            .map(|mapper| mapper.map_authorities(user.authorities()))
            .unwrap_or_else(|| user.authorities().to_vec());

        Ok(Some(Arc::new(
            OAuth2LoginAuthenticationToken::authenticated(
                principal,
                registration,
                exchange,
                access_token,
                authorities,
            ),
        )))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OAuth2LoginAuthenticationToken>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::async_trait;
    use serde_json::json;

    use super::OidcLoginAuthenticationProvider;
    use crate::{
        authentication::AuthenticationProvider,
        core::{Authentication, AuthenticationError},
        oauth2::{
            ClientRegistration, OAuth2AccessTokenResponse, OAuth2AccessTokenResponseClient,
            OAuth2AuthorizationCodeGrantRequest, OAuth2AuthorizationExchange,
            OAuth2AuthorizationRequest, OAuth2AuthorizationResponse,
            OAuth2LoginAuthenticationToken, OAuth2UserService, OidcIdToken, OidcIdTokenValidator,
            OidcUser, OidcUserRequest,
        },
    };

    /// A token response client that answers with a fixed response.
    #[derive(Clone)]
    struct FixedTokenResponseClient(OAuth2AccessTokenResponse);

    #[async_trait]
    impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>
        for FixedTokenResponseClient
    {
        async fn get_token_response(
            &self,
            _request: &OAuth2AuthorizationCodeGrantRequest,
        ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
            Ok(self.0.clone())
        }
    }

    /// A validator that accepts every token and names a fixed subject.
    struct FixedIdTokenValidator;

    #[async_trait]
    impl OidcIdTokenValidator for FixedIdTokenValidator {
        async fn validate(
            &self,
            _client_registration: &ClientRegistration,
            _id_token: &str,
            _nonce: Option<&str>,
            _access_token: Option<&str>,
        ) -> Result<OidcIdToken, AuthenticationError> {
            Ok(OidcIdToken::new(
                "https://provider.example",
                "user",
                vec!["demo-client".to_string()],
                json!({ "sub": "user" }),
            ))
        }
    }

    /// A validator that refuses every token.
    struct RefusingIdTokenValidator;

    #[async_trait]
    impl OidcIdTokenValidator for RefusingIdTokenValidator {
        async fn validate(
            &self,
            _client_registration: &ClientRegistration,
            _id_token: &str,
            _nonce: Option<&str>,
            _access_token: Option<&str>,
        ) -> Result<OidcIdToken, AuthenticationError> {
            Err(AuthenticationError::new("The token is not valid"))
        }
    }

    /// A user service that answers with a fixed user.
    struct FixedUserService;

    #[async_trait]
    impl OAuth2UserService<OidcUserRequest, OidcUser> for FixedUserService {
        async fn load_user(
            &self,
            request: &OidcUserRequest,
        ) -> Result<OidcUser, AuthenticationError> {
            // The user is the subject of the validated identifier token, which
            // is what the provider hands to the service.
            let subject = request
                .id_token()
                .map(|id_token| id_token.subject().to_string())
                .expect("the provider validates the identifier token first");

            Ok(
                OidcUser::new(subject, Some("https://provider.example".to_string()))
                    .set_authorities(vec![Arc::new(
                        crate::core::authority::SimpleGrantedAuthority::new("SCOPE_read"),
                    )
                        as Arc<dyn crate::core::GrantedAuthority>]),
            )
        }
    }

    /// A mapper that renames the authorities it is given.
    struct RenamingAuthoritiesMapper;

    impl crate::core::authority::mapping::GrantedAuthoritiesMapper for RenamingAuthoritiesMapper {
        fn map_authorities(
            &self,
            authorities: &[Arc<dyn crate::core::GrantedAuthority>],
        ) -> Vec<Arc<dyn crate::core::GrantedAuthority>> {
            authorities
                .iter()
                .map(|authority| {
                    let name = authority.authority().unwrap_or_default();
                    Arc::new(crate::core::authority::SimpleGrantedAuthority::new(
                        format!("MAPPED_{name}"),
                    )) as Arc<dyn crate::core::GrantedAuthority>
                })
                .collect()
        }
    }

    /// Returns the registration of a client that asks for the given scopes.
    fn registration(scopes: &[&str]) -> ClientRegistration {
        ClientRegistration::new("demo", "Demo", "https://provider.example/oauth2/authorize")
            .set_client_id("demo-client")
            .set_scopes(scopes.to_vec())
    }

    /// Returns an authentication of a login of the given registration.
    fn login(registration: ClientRegistration) -> Arc<dyn Authentication> {
        let authorization_request = OAuth2AuthorizationRequest::new(
            "demo",
            "https://provider.example/oauth2/authorize",
            "state",
            None,
            registration.scopes().to_vec(),
        );
        let exchange = OAuth2AuthorizationExchange::new(
            authorization_request,
            OAuth2AuthorizationResponse::success("code", Some("state".to_string())),
        );

        Arc::new(OAuth2LoginAuthenticationToken::unauthenticated(
            registration,
            exchange,
        ))
    }

    fn provider(
        id_token_validator: Arc<dyn OidcIdTokenValidator>,
    ) -> OidcLoginAuthenticationProvider {
        let mut provider = OidcLoginAuthenticationProvider::new(
            Arc::new(FixedTokenResponseClient(
                OAuth2AccessTokenResponse::new("the-access-token")
                    .set_id_token("the-identifier-token"),
            )),
            Arc::new(FixedUserService),
        );
        provider.set_oidc_id_token_validator(id_token_validator);

        provider
    }

    #[tokio::test]
    async fn an_openid_login_becomes_the_user_of_the_identifier_token() {
        let provider = provider(Arc::new(FixedIdTokenValidator));

        let authenticated = provider
            .authenticate(&login(registration(&["openid", "read"])))
            .await
            .expect("the login is authenticated")
            .expect("the provider handles the login");
        let as_any: &dyn std::any::Any = authenticated.as_ref();
        let token = as_any
            .downcast_ref::<OAuth2LoginAuthenticationToken>()
            .expect("the authentication is an OAuth2 login");

        let principal = token
            .principal()
            .expect("the login authentication has a principal");
        assert_eq!(principal.to_string(), "user");
        // Without a mapper the authorities of the user are the authorities of
        // the grant.
        let authorities: Vec<String> = token
            .authorities()
            .iter()
            .filter_map(|authority| authority.authority().map(ToString::to_string))
            .collect();
        assert_eq!(authorities, ["SCOPE_read"]);
    }

    #[tokio::test]
    async fn a_grant_without_openid_is_left_to_the_plain_login() {
        let provider = provider(Arc::new(FixedIdTokenValidator));

        let handled = provider
            .authenticate(&login(registration(&["read"])))
            .await
            .expect("the provider does not fail the login");

        assert!(handled.is_none());
    }

    #[tokio::test]
    async fn an_openid_login_with_an_unverifiable_token_is_refused() {
        let provider = provider(Arc::new(RefusingIdTokenValidator));

        let error = provider
            .authenticate(&login(registration(&["openid"])))
            .await
            .expect_err("the identifier token is not verified");

        assert_eq!(error.message(), "The token is not valid");
    }

    #[tokio::test]
    async fn an_openid_login_without_an_identifier_token_is_refused() {
        let mut provider = OidcLoginAuthenticationProvider::new(
            Arc::new(FixedTokenResponseClient(OAuth2AccessTokenResponse::new(
                "the-access-token",
            ))),
            Arc::new(FixedUserService),
        );
        provider.set_oidc_id_token_validator(Arc::new(FixedIdTokenValidator));

        assert!(provider
            .authenticate(&login(registration(&["openid"])))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn the_authorities_of_the_user_are_mapped() {
        let mut provider = provider(Arc::new(FixedIdTokenValidator));
        provider.set_authorities_mapper(Arc::new(RenamingAuthoritiesMapper));

        let authenticated = provider
            .authenticate(&login(registration(&["openid"])))
            .await
            .expect("the login is authenticated")
            .expect("the provider handles the login");
        let as_any: &dyn std::any::Any = authenticated.as_ref();
        let token = as_any
            .downcast_ref::<OAuth2LoginAuthenticationToken>()
            .expect("the authentication is an OAuth2 login");
        let authorities: Vec<String> = token
            .authorities()
            .iter()
            .filter_map(|authority| authority.authority().map(ToString::to_string))
            .collect();

        assert_eq!(authorities, ["MAPPED_SCOPE_read"]);
    }
}
