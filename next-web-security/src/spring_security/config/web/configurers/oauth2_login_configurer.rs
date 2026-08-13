use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{
                BaseAuthenticationFilterConfigurer, BaseAuthenticationFilterConfigurerExt,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::authority_mapping::GrantedAuthoritiesMapper,
    oauth2::{
        AuthenticatedPrincipalOAuth2AuthorizedClientRepository, AuthorizationGrantType,
        AuthorizationRequestRepository, ClientRegistrationRepository,
        DefaultOAuth2AuthorizationRequestResolver, DefaultOAuth2UserService,
        HttpSessionOAuth2AuthorizationRequestRepository, NullOAuth2AuthorizedClientService,
        OAuth2AccessTokenResponseClient, OAuth2AuthorizationCodeGrantRequest,
        OAuth2AuthorizationRequestResolver, OAuth2AuthorizedClientRepository,
        OAuth2AuthorizedClientService, OAuth2LoginAuthenticationProvider, OAuth2User,
        OAuth2UserRequest, OAuth2UserService, OidcAuthenticationRequestChecker, OidcUser,
        OidcUserRequest, OidcUserService, RestClientAuthorizationCodeTokenResponseClient,
    },
    web::{
        authentication::{
            ui::DefaultLoginPageGeneratingFilter, LoginUrlAuthenticationEntryPoint,
            OAuth2AuthorizationRequestRedirectFilter, OAuth2LoginAuthenticationFilter,
            OAuth2LoginRequestMatcher,
        },
        context::SecurityContextRepository,
        default_security_filter_chain::DefaultSecurityFilterChain,
        DefaultRedirectStrategy, RedirectStrategy,
    },
};

/// Configures OAuth 2.0 / OpenID Connect 1.0 Login authentication.
///
/// This minimal translation wires Spring Security's OAuth2 login shape into
/// the current Rust HTTP security builder. Token endpoint, UserInfo, and OIDC
/// JWT processing remain explicit placeholders and will fail closed.
#[derive(Clone)]
pub struct OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    authorization_endpoint_config: AuthorizationEndpointConfig,
    token_endpoint_config: TokenEndpointConfig,
    redirection_endpoint_config: RedirectionEndpointConfig,
    user_info_endpoint_config: UserInfoEndpointConfig,

    login_page: Option<Box<str>>,
    login_processing_url: Box<str>,
    client_registration_repository: Option<Arc<dyn ClientRegistrationRepository>>,
    authorized_client_repository: Option<Arc<dyn OAuth2AuthorizedClientRepository>>,
    security_context_repository: Option<Arc<dyn SecurityContextRepository>>,

    inner: BaseAuthenticationFilterConfigurer<H, Self, OAuth2LoginAuthenticationFilter>,
}

impl<H> OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    pub fn client_registration_repository(
        &mut self,
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> &mut Self {
        self.client_registration_repository = Some(client_registration_repository);
        self
    }

    pub fn authorized_client_repository(
        &mut self,
        authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    ) -> &mut Self {
        self.authorized_client_repository = Some(authorized_client_repository);
        self
    }

    pub fn authorized_client_service(
        &mut self,
        authorized_client_service: Arc<dyn OAuth2AuthorizedClientService>,
    ) -> &mut Self {
        self.authorized_client_repository(Arc::new(
            AuthenticatedPrincipalOAuth2AuthorizedClientRepository::new(authorized_client_service),
        ))
    }

    pub fn login_page(&mut self, login_page: &str) -> &mut Self {
        assert!(!login_page.trim().is_empty(), "login_page cannot be empty");
        self.login_page = Some(login_page.into());
        self
    }

    pub fn login_processing_url(&mut self, login_processing_url: &str) -> &mut Self {
        assert!(
            !login_processing_url.trim().is_empty(),
            "login_processing_url cannot be empty"
        );
        self.login_processing_url = login_processing_url.into();
        self.inner.login_processing_url(login_processing_url);
        self
    }

    pub fn security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) -> &mut Self {
        self.security_context_repository = Some(security_context_repository.clone());
        if let Some(authentication_filter) = self.inner.get_authentication_filter_mut() {
            authentication_filter.set_security_context_repository(security_context_repository);
        }
        self
    }

    pub fn authorization_endpoint<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut AuthorizationEndpointConfig),
    {
        customizer(&mut self.authorization_endpoint_config);
        self
    }

    pub fn token_endpoint<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut TokenEndpointConfig),
    {
        customizer(&mut self.token_endpoint_config);
        self
    }

    pub fn redirection_endpoint<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut RedirectionEndpointConfig),
    {
        customizer(&mut self.redirection_endpoint_config);
        self
    }

    pub fn user_info_endpoint<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut UserInfoEndpointConfig),
    {
        customizer(&mut self.user_info_endpoint_config);
        self
    }

    fn get_client_registration_repository(
        &self,
        http: &H,
    ) -> Arc<dyn ClientRegistrationRepository> {
        self.client_registration_repository
            .clone()
            .or_else(|| {
                http.shared_object::<Arc<dyn ClientRegistrationRepository>>()
                    .cloned()
            })
            .expect("ClientRegistrationRepository is required for oauth2_login")
    }

    fn get_authorized_client_repository(
        &self,
        http: &H,
    ) -> Arc<dyn OAuth2AuthorizedClientRepository> {
        self.authorized_client_repository
            .clone()
            .or_else(|| {
                http.shared_object::<Arc<dyn OAuth2AuthorizedClientRepository>>()
                    .cloned()
            })
            .unwrap_or_else(|| {
                Arc::new(AuthenticatedPrincipalOAuth2AuthorizedClientRepository::new(
                    Arc::new(NullOAuth2AuthorizedClientService::default()),
                ))
            })
    }

    fn get_authorization_request_repository(&self) -> Arc<dyn AuthorizationRequestRepository> {
        self.authorization_endpoint_config
            .authorization_request_repository
            .clone()
            .unwrap_or_else(|| Arc::new(HttpSessionOAuth2AuthorizationRequestRepository::default()))
    }

    fn get_authorization_request_resolver(
        &self,
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> Arc<dyn OAuth2AuthorizationRequestResolver> {
        self.authorization_endpoint_config
            .authorization_request_resolver
            .clone()
            .unwrap_or_else(|| {
                Arc::new(DefaultOAuth2AuthorizationRequestResolver::new(
                    client_registration_repository,
                    self.authorization_endpoint_config
                        .authorization_request_base_uri
                        .as_deref()
                        .unwrap_or(
                            OAuth2AuthorizationRequestRedirectFilter::DEFAULT_AUTHORIZATION_REQUEST_BASE_URI,
                        ),
                ))
            })
    }

    fn get_access_token_response_client(
        &self,
    ) -> Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>> {
        self.token_endpoint_config
            .access_token_response_client
            .clone()
            .unwrap_or_else(|| Arc::new(RestClientAuthorizationCodeTokenResponseClient::default()))
    }

    fn get_oauth2_user_service(&self) -> Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>> {
        self.user_info_endpoint_config
            .user_service
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultOAuth2UserService::default()))
    }

    fn get_oidc_user_service(&self) -> Arc<dyn OAuth2UserService<OidcUserRequest, OidcUser>> {
        self.user_info_endpoint_config
            .oidc_user_service
            .clone()
            .unwrap_or_else(|| Arc::new(OidcUserService::default()))
    }

    fn get_login_links(
        &self,
        client_registration_repository: &Arc<dyn ClientRegistrationRepository>,
    ) -> HashMap<String, String> {
        let authorization_request_base_uri = self
            .authorization_endpoint_config
            .authorization_request_base_uri
            .as_deref()
            .unwrap_or(
                OAuth2AuthorizationRequestRedirectFilter::DEFAULT_AUTHORIZATION_REQUEST_BASE_URI,
            );

        client_registration_repository
            .client_registrations()
            .into_iter()
            .filter(|registration| {
                registration.authorization_grant_type()
                    == &AuthorizationGrantType::AuthorizationCode
            })
            .map(|registration| {
                (
                    format!(
                        "{}/{}",
                        authorization_request_base_uri.trim_end_matches('/'),
                        registration.registration_id()
                    ),
                    registration.client_name().to_string(),
                )
            })
            .collect()
    }

    fn init_default_login_filter(&self, http: &mut H, login_links: HashMap<String, String>) {
        if self.login_page.is_some() || login_links.is_empty() {
            return;
        }
        if let Some(login_page_generating_filter) =
            http.shared_object_mut::<DefaultLoginPageGeneratingFilter>()
        {
            login_page_generating_filter.set_oauth2_login_enabled(true);
            login_page_generating_filter.set_oauth2_authentication_url_to_client_name(login_links);
            login_page_generating_filter.set_login_page_url(self.inner.get_login_page());
            if let Some(failure_url) = self.inner.get_failure_url() {
                login_page_generating_filter.set_failure_url(failure_url);
            }
        }
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        let client_registration_repository = self.get_client_registration_repository(http);
        let authorized_client_repository = self.get_authorized_client_repository(http);
        let authorization_request_repository = self.get_authorization_request_repository();

        http.set_shared_object::<Arc<dyn ClientRegistrationRepository>>(
            client_registration_repository.clone(),
        );
        http.set_shared_object::<Arc<dyn OAuth2AuthorizedClientRepository>>(
            authorized_client_repository.clone(),
        );

        let mut authentication_filter = OAuth2LoginAuthenticationFilter::new(
            client_registration_repository.clone(),
            authorized_client_repository,
            self.login_processing_url.as_ref(),
        );
        authentication_filter
            .set_authorization_request_repository(authorization_request_repository.clone());
        authentication_filter.set_security_context_holder_strategy(
            self.inner.get_security_context_holder_strategy().to_owned(),
        );
        if let Some(security_context_repository) = self.security_context_repository.as_ref() {
            authentication_filter
                .set_security_context_repository(security_context_repository.clone());
        }
        self.inner.set_authentication_filter(authentication_filter);
        self.inner
            .login_processing_url(self.login_processing_url.as_ref());

        let login_links = self.get_login_links(&client_registration_repository);
        if let Some(login_page) = self.login_page.as_deref() {
            self.inner.login_page(login_page, http);
        } else if login_links.len() == 1 {
            if let Some(provider_login_page) = login_links.keys().next() {
                self.inner.register_authentication_entry_point(
                    http,
                    Arc::new(LoginUrlAuthenticationEntryPoint::new(
                        provider_login_page.as_str(),
                    )),
                );
            }
        }

        let mut oauth2_login_authentication_provider = OAuth2LoginAuthenticationProvider::new(
            self.get_access_token_response_client(),
            self.get_oauth2_user_service(),
        );
        if let Some(authorities_mapper) = self
            .user_info_endpoint_config
            .user_authorities_mapper
            .as_ref()
        {
            http.set_shared_object::<Arc<dyn GrantedAuthoritiesMapper>>(authorities_mapper.clone());
            oauth2_login_authentication_provider.set_authorities_mapper(authorities_mapper.clone());
        } else if let Some(authorities_mapper) =
            http.shared_object::<Arc<dyn GrantedAuthoritiesMapper>>()
        {
            oauth2_login_authentication_provider.set_authorities_mapper(authorities_mapper.clone());
        }

        let _oidc_user_service = self.get_oidc_user_service();
        http.authentication_provider(Arc::new(oauth2_login_authentication_provider));
        http.authentication_provider(Arc::new(OidcAuthenticationRequestChecker::default()));

        self.inner.init(http);
        self.init_default_login_filter(http, login_links);
    }

    fn configure(&mut self, http: &mut H) {
        let client_registration_repository = self.get_client_registration_repository(http);
        let authorization_request_repository = self.get_authorization_request_repository();
        let resolver = self.get_authorization_request_resolver(client_registration_repository);

        let mut authorization_request_filter =
            OAuth2AuthorizationRequestRedirectFilter::new(resolver);
        authorization_request_filter
            .set_authorization_request_repository(authorization_request_repository.clone());
        if let Some(redirect_strategy) = self
            .authorization_endpoint_config
            .authorization_redirect_strategy
            .as_ref()
        {
            authorization_request_filter
                .set_authorization_redirect_strategy(redirect_strategy.clone());
        }
        http.add_filter(authorization_request_filter);

        if let Some(authentication_filter) = self.inner.get_authentication_filter_mut() {
            authentication_filter
                .set_authorization_request_repository(authorization_request_repository);
            if let Some(authorization_response_base_uri) = self
                .redirection_endpoint_config
                .authorization_response_base_uri
                .as_deref()
            {
                authentication_filter.set_requires_authentication_request_matcher(Arc::new(
                    OAuth2LoginRequestMatcher::new(authorization_response_base_uri),
                ));
            }
        }

        self.inner.configure(http);
    }
}

impl<H> Default for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn default() -> Self {
        let mut configurer = Self {
            authorization_endpoint_config: AuthorizationEndpointConfig::default(),
            token_endpoint_config: TokenEndpointConfig::default(),
            redirection_endpoint_config: RedirectionEndpointConfig::default(),
            user_info_endpoint_config: UserInfoEndpointConfig::default(),
            login_page: None,
            login_processing_url: OAuth2LoginAuthenticationFilter::DEFAULT_FILTER_PROCESSES_URI
                .into(),
            client_registration_repository: None,
            authorized_client_repository: None,
            security_context_repository: None,
            inner: BaseAuthenticationFilterConfigurer::default(),
        };
        configurer
            .inner
            .login_processing_url(OAuth2LoginAuthenticationFilter::DEFAULT_FILTER_PROCESSES_URI);
        configurer
    }
}

impl<H> Deref for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseAuthenticationFilterConfigurer<H, Self, OAuth2LoginAuthenticationFilter>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Clone)]
pub struct AuthorizationEndpointConfig {
    authorization_request_base_uri: Option<Box<str>>,
    authorization_request_resolver: Option<Arc<dyn OAuth2AuthorizationRequestResolver>>,
    authorization_request_repository: Option<Arc<dyn AuthorizationRequestRepository>>,
    authorization_redirect_strategy: Option<Arc<dyn RedirectStrategy>>,
}

impl AuthorizationEndpointConfig {
    pub fn base_uri(&mut self, authorization_request_base_uri: &str) -> &mut Self {
        assert!(
            !authorization_request_base_uri.trim().is_empty(),
            "authorization_request_base_uri cannot be empty"
        );
        self.authorization_request_base_uri = Some(authorization_request_base_uri.into());
        self
    }

    pub fn authorization_request_resolver(
        &mut self,
        authorization_request_resolver: Arc<dyn OAuth2AuthorizationRequestResolver>,
    ) -> &mut Self {
        self.authorization_request_resolver = Some(authorization_request_resolver);
        self
    }

    pub fn authorization_request_repository(
        &mut self,
        authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    ) -> &mut Self {
        self.authorization_request_repository = Some(authorization_request_repository);
        self
    }

    pub fn authorization_redirect_strategy(
        &mut self,
        authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
    ) -> &mut Self {
        self.authorization_redirect_strategy = Some(authorization_redirect_strategy);
        self
    }
}

impl Default for AuthorizationEndpointConfig {
    fn default() -> Self {
        Self {
            authorization_request_base_uri: Some(
                OAuth2AuthorizationRequestRedirectFilter::DEFAULT_AUTHORIZATION_REQUEST_BASE_URI
                    .into(),
            ),
            authorization_request_resolver: None,
            authorization_request_repository: None,
            authorization_redirect_strategy: Some(Arc::new(DefaultRedirectStrategy::default())),
        }
    }
}

#[derive(Clone, Default)]
pub struct TokenEndpointConfig {
    access_token_response_client:
        Option<Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>>,
}

impl TokenEndpointConfig {
    pub fn access_token_response_client(
        &mut self,
        access_token_response_client: Arc<
            dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>,
        >,
    ) -> &mut Self {
        self.access_token_response_client = Some(access_token_response_client);
        self
    }
}

#[derive(Clone, Default)]
pub struct RedirectionEndpointConfig {
    authorization_response_base_uri: Option<Box<str>>,
}

impl RedirectionEndpointConfig {
    pub fn base_uri(&mut self, authorization_response_base_uri: &str) -> &mut Self {
        assert!(
            !authorization_response_base_uri.trim().is_empty(),
            "authorization_response_base_uri cannot be empty"
        );
        self.authorization_response_base_uri = Some(authorization_response_base_uri.into());
        self
    }
}

#[derive(Clone, Default)]
pub struct UserInfoEndpointConfig {
    user_service: Option<Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>>,
    oidc_user_service: Option<Arc<dyn OAuth2UserService<OidcUserRequest, OidcUser>>>,
    user_authorities_mapper: Option<Arc<dyn GrantedAuthoritiesMapper>>,
}

impl UserInfoEndpointConfig {
    pub fn user_service(
        &mut self,
        user_service: Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>,
    ) -> &mut Self {
        self.user_service = Some(user_service);
        self
    }

    pub fn oidc_user_service(
        &mut self,
        oidc_user_service: Arc<dyn OAuth2UserService<OidcUserRequest, OidcUser>>,
    ) -> &mut Self {
        self.oidc_user_service = Some(oidc_user_service);
        self
    }

    pub fn user_authorities_mapper(
        &mut self,
        user_authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) -> &mut Self {
        self.user_authorities_mapper = Some(user_authorities_mapper);
        self
    }
}
