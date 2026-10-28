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
    core::authority::mapping::GrantedAuthoritiesMapper,
    oauth2::{
        client::web::{
            OAuth2AuthorizationRequestRedirectFilter, OAuth2LoginAuthenticationFilter,
            OAuth2LoginRequestMatcher,
        },
        AuthenticatedPrincipalOAuth2AuthorizedClientRepository, AuthorizationGrantType,
        AuthorizationRequestRepository, ClientRegistrationRepository,
        DefaultOAuth2AuthorizationRequestResolver, DefaultOAuth2UserService,
        HttpSessionOAuth2AuthorizationRequestRepository, NullOAuth2AuthorizedClientService,
        OAuth2AccessTokenResponseClient, OAuth2AuthorizationCodeGrantRequest,
        OAuth2AuthorizationRequestResolver, OAuth2AuthorizedClientRepository,
        OAuth2AuthorizedClientService, OAuth2LoginAuthenticationProvider, OAuth2User,
        OAuth2UserRequest, OAuth2UserService, OidcAuthenticationRequestChecker,
        OidcLoginAuthenticationProvider, OidcUser, OidcUserRequest, OidcUserService,
        RestClientAuthorizationCodeTokenResponseClient,
    },
    web::{
        authentication::{ui::DefaultLoginPageGeneratingFilter, LoginUrlAuthenticationEntryPoint},
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

    base: BaseAuthenticationFilterConfigurer<H, Self, OAuth2LoginAuthenticationFilter>,
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
        self.base.login_processing_url(login_processing_url);
        self.set_login_processing_matcher();
        self
    }

    /// Installs the matcher of the login processing URL on the authentication
    /// filter.
    ///
    /// The redirect back from the provider is a GET request, so the matcher
    /// must not restrict the method and must match the registration id the
    /// callback names.
    fn set_login_processing_matcher(&mut self) {
        let matcher = Arc::new(OAuth2LoginRequestMatcher::new(
            self.login_processing_url.as_ref(),
        ));
        if let Some(authentication_filter) = self.base.get_authentication_filter_mut() {
            authentication_filter.set_requires_authentication_request_matcher(matcher);
        }
    }

    pub fn security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) -> &mut Self {
        self.security_context_repository = Some(security_context_repository.clone());
        if let Some(authentication_filter) = self.base.get_authentication_filter_mut() {
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
            login_page_generating_filter.set_login_page_url(self.base.get_login_page());
            if let Some(failure_url) = self.base.get_failure_url() {
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
            self.base.get_security_context_holder_strategy().to_owned(),
        );
        if let Some(security_context_repository) = self.security_context_repository.as_ref() {
            authentication_filter
                .set_security_context_repository(security_context_repository.clone());
        }
        self.base.set_authentication_filter(authentication_filter);
        self.base
            .login_processing_url(self.login_processing_url.as_ref());
        self.set_login_processing_matcher();

        let login_links = self.get_login_links(&client_registration_repository);
        if let Some(login_page) = self.login_page.as_deref() {
            self.base.login_page(login_page, http);
        } else if login_links.len() == 1 {
            if let Some(provider_login_page) = login_links.keys().next() {
                self.base.register_authentication_entry_point(
                    http,
                    Arc::new(LoginUrlAuthenticationEntryPoint::new(
                        provider_login_page.as_str(),
                    )),
                );
            }
        }

        let access_token_response_client = self.get_access_token_response_client();
        let authorities_mapper = self
            .user_info_endpoint_config
            .user_authorities_mapper
            .as_ref()
            .cloned()
            .or_else(|| {
                http.shared_object::<Arc<dyn GrantedAuthoritiesMapper>>()
                    .cloned()
            });
        if let Some(authorities_mapper) = authorities_mapper.as_ref() {
            http.set_shared_object::<Arc<dyn GrantedAuthoritiesMapper>>(authorities_mapper.clone());
        }

        // The OpenID Connect login runs before the plain OAuth 2.0 login, so
        // that a grant that asks for the `openid` scope is authenticated with
        // the identifier token it receives.
        let mut oidc_login_authentication_provider = OidcLoginAuthenticationProvider::new(
            access_token_response_client.clone(),
            self.get_oidc_user_service(),
        );
        let mut oauth2_login_authentication_provider = OAuth2LoginAuthenticationProvider::new(
            access_token_response_client,
            self.get_oauth2_user_service(),
        );
        if let Some(authorities_mapper) = authorities_mapper {
            oidc_login_authentication_provider.set_authorities_mapper(authorities_mapper.clone());
            oauth2_login_authentication_provider.set_authorities_mapper(authorities_mapper);
        }

        http.authentication_provider(Arc::new(oidc_login_authentication_provider));
        http.authentication_provider(Arc::new(oauth2_login_authentication_provider));
        http.authentication_provider(Arc::new(OidcAuthenticationRequestChecker::default()));

        self.base.init(http);
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

        if let Some(authentication_filter) = self.base.get_authentication_filter_mut() {
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

        self.base.configure(http);
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
            base: BaseAuthenticationFilterConfigurer::default(),
        };
        configurer
            .base
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
        &self.base
    }
}

impl<H> DerefMut for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
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

#[cfg(all(test, feature = "oauth2-client"))]
mod tests {
    use std::{any::TypeId, collections::HashMap, sync::Arc};

    use axum::{body::Body, extract::Request, response::Response};

    use super::*;
    use crate::{
        authentication::dao::DaoAuthenticationProvider,
        config::authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        config::security_builder::SecurityBuilder,
        config::web::builders::HttpSecurity,
        config::web::http_security_builder::HttpSecurityBuilder,
        core::userdetails::{User, UserDetailsService},
        crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
        oauth2::client::registration::{ClientRegistration, InMemoryClientRegistrationRepository},
        provisioning::InMemoryUserDetailsManager,
        web::security_filter_chain::SecurityFilterChain,
        web::test_support::Chain,
        web::util::matcher::PathPatternRequestMatcherBuilder,
    };

    /// Returns the registration the flow is performed for.
    fn registration() -> ClientRegistration {
        ClientRegistration::new(
            "okta",
            "Okta",
            "https://idp.example.org/oauth2/authorize",
        )
        .set_client_id("client")
    }

    /// Returns a chain whose OAuth 2.0 login is configured for one registration.
    fn http_security() -> HttpSecurity {
        // The configurer takes its matchers from a shared builder, the way the
        // configuration of an application provides one.
        let mut shared_objects: HashMap<TypeId, Box<dyn next_web_core::any::AnyClone>> =
            HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        http.oauth2_login(|login| {
            login.client_registration_repository(Arc::new(
                InMemoryClientRegistrationRepository::new(vec![registration()]),
            ));
        });

        // A chain has to be able to authenticate something at all; the redirect
        // does not use it, but the chain is assembled around it.
        let encoder = BCryptPasswordEncoder::new(4);
        let hash = encoder
            .encode(Some("password"))
            .expect("the password is encoded")
            .expect("the encoder returns a hash");
        let user_details_service: Arc<dyn UserDetailsService> =
            Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("user")
                    .password(Some(hash))
                    .build(),
            )]));
        let mut provider = DaoAuthenticationProvider::new(user_details_service);
        provider.set_password_encoder(Arc::new(encoder));
        http.authentication_provider(Arc::new(provider));

        http
    }

    /// Returns the `Location` header of a response.
    fn redirect_of(response: &Response) -> Option<String> {
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    #[tokio::test]
    async fn the_authorization_endpoint_redirects_to_the_provider() {
        let mut http = http_security();
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let mut request = Request::builder()
            .uri("https://sp.example.org/oauth2/authorization/okta")
            .body(Body::empty())
            .expect("a valid request");
        let mut response = Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        assert!(
            response.status().is_redirection(),
            "the authorization endpoint redirects, status was {}",
            response.status()
        );
        let location = redirect_of(&response).expect("the redirect names a location");
        assert!(
            location.starts_with("https://idp.example.org/oauth2/authorize?"),
            "the redirect goes to the authorization uri, was: {location}"
        );
        assert!(
            location.contains("response_type=code"),
            "the request asks for a code, was: {location}"
        );
        assert!(
            location.contains("client_id=client"),
            "the request names the client, was: {location}"
        );
        assert!(
            location.contains("state="),
            "the request carries a state, was: {location}"
        );
    }
}

/// The tests of the callback: the request the redirect stores, and the response
/// the callback exchanges for it.
#[cfg(all(test, feature = "oauth2-client"))]
mod callback_tests {
    use std::{any::TypeId, collections::HashMap, sync::Arc};

    use axum::{body::Body, extract::Request, response::Response};

    use super::*;
    use crate::{
        config::authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        config::security_builder::SecurityBuilder,
        config::web::builders::HttpSecurity,
        config::web::http_security_builder::HttpSecurityBuilder,
        oauth2::client::registration::{ClientRegistration, InMemoryClientRegistrationRepository},
        oauth2::client::web::AuthorizationRequestRepository,
        oauth2::core::endpoint::OAuth2AuthorizationRequest,
        web::security_filter_chain::SecurityFilterChain,
        web::test_support::Chain,
        web::util::matcher::PathPatternRequestMatcherBuilder,
    };
    use next_web_core::http::{HttpRequest, HttpResponse};

    /// A repository that keeps the authorization request in memory.
    ///
    /// The default repository writes to the session of the request, and a test has
    /// no session: without this double, the redirect cannot be shown to remember
    /// anything at all.
    #[derive(Default, Clone)]
    struct InMemoryAuthorizationRequestRepository {
        request: Arc<std::sync::RwLock<Option<OAuth2AuthorizationRequest>>>,
    }

    impl AuthorizationRequestRepository for InMemoryAuthorizationRequestRepository {
        fn load_authorization_request(
            &self,
            _request: &dyn HttpRequest,
        ) -> Option<OAuth2AuthorizationRequest> {
            // The callback looks the request up by the state it answers, so the
            // double hands out a copy, the way the session repository does.
            let stored = self.request.read().expect("the repository is not poisoned");

            (*stored).clone()
        }

        fn save_authorization_request(
            &self,
            authorization_request: Option<OAuth2AuthorizationRequest>,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) {
            *self
                .request
                .write()
                .expect("the repository is not poisoned") = authorization_request;
        }

        fn remove_authorization_request(
            &self,
            _request: &mut dyn HttpRequest,
        ) -> Option<OAuth2AuthorizationRequest> {
            self.request
                .write()
                .expect("the repository is not poisoned")
                .take()
        }
    }

    /// Returns the registration the flow is performed for.
    fn registration() -> ClientRegistration {
        ClientRegistration::new(
            "okta",
            "Okta",
            "https://idp.example.org/oauth2/authorize",
        )
        .set_client_id("client")
    }

    /// Answers the token exchange with a fixed token.
    struct FakeTokenClient;

    #[next_web_core::async_trait]
    impl crate::oauth2::OAuth2AccessTokenResponseClient<
        crate::oauth2::OAuth2AuthorizationCodeGrantRequest,
    > for FakeTokenClient
    {
        async fn get_token_response(
            &self,
            _request: &crate::oauth2::OAuth2AuthorizationCodeGrantRequest,
        ) -> Result<
            crate::oauth2::core::endpoint::OAuth2AccessTokenResponse,
            crate::core::AuthenticationError,
        > {
            Ok(crate::oauth2::core::endpoint::OAuth2AccessTokenResponse::new(
                "an-access-token",
            ))
        }
    }

    /// Answers the profile of the user without asking the provider.
    struct FakeUserService;

    #[next_web_core::async_trait]
    impl crate::oauth2::OAuth2UserService<crate::oauth2::OAuth2UserRequest, crate::oauth2::OAuth2User>
        for FakeUserService
    {
        async fn load_user(
            &self,
            _request: &crate::oauth2::OAuth2UserRequest,
        ) -> Result<crate::oauth2::OAuth2User, crate::core::AuthenticationError> {
            Ok(crate::oauth2::OAuth2User::new(
                "Max",
                std::collections::HashMap::new(),
            ))
        }
    }

    /// Returns the `Location` header of a response.
    fn redirect_of(response: &Response) -> Option<String> {
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    /// Returns a chain that remembers authorization requests in memory.
    fn http_security(repository: Arc<InMemoryAuthorizationRequestRepository>) -> HttpSecurity {
        let mut shared_objects: HashMap<TypeId, Box<dyn next_web_core::any::AnyClone>> =
            HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        http.oauth2_login(|login| {
            login
                .client_registration_repository(Arc::new(
                    InMemoryClientRegistrationRepository::new(vec![registration()]),
                ))
                .authorization_endpoint(|endpoint| {
                    endpoint.authorization_request_repository(repository.clone());
                })
                .token_endpoint(|endpoint| {
                    endpoint.access_token_response_client(Arc::new(FakeTokenClient));
                })
                .user_info_endpoint(|endpoint| {
                    endpoint.user_service(Arc::new(FakeUserService));
                });
        });

        http
    }

    #[tokio::test]
    async fn the_redirect_remembers_the_authorization_request() {
        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut http = http_security(repository.clone());
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let mut request = Request::builder()
            .uri("https://sp.example.org/oauth2/authorization/okta")
            .body(Body::empty())
            .expect("a valid request");
        let mut response = Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        assert!(
            response.status().is_redirection(),
            "the authorization endpoint redirects, status was {}",
            response.status()
        );

        let stored = repository
            .load_authorization_request(&request)
            .expect("the redirect stores the authorization request the callback answers");
        assert_eq!(
            stored.registration_id(),
            "okta",
            "the stored request belongs to the registration of the endpoint"
        );
        assert!(
            !stored.state().is_empty(),
            "the stored request carries the state the callback is answered with"
        );
    }

    #[tokio::test]
    async fn a_callback_with_an_unknown_state_is_refused() {
        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut http = http_security(repository);
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let body = "code=the-code&state=not-a-state-we-issued";
        let mut request = Request::builder()
            .method("POST")
            .uri("https://sp.example.org/login/oauth2/code/okta")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(
                body.as_bytes().to_vec(),
            ));

        let mut response = Response::default();
        let outcome = Chain::new(filters).run(&mut request, &mut response).await;

        // A callback answers the request the redirect stored; one it never issued
        // must not be accepted, whether the filter reports a failure or answers
        // with a redirect to the login page.
        assert!(
            !(outcome.is_ok() && response.status().is_success()),
            "the callback accepted a state this application never issued, status was {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn a_callback_that_answers_the_issued_request_logs_the_user_in() {
        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut http = http_security(repository.clone());
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        // The redirect starts the flow and remembers the request it issued.
        let mut request = Request::builder()
            .uri("https://sp.example.org/oauth2/authorization/okta")
            .body(Body::empty())
            .expect("a valid request");
        let mut response = Response::default();
        Chain::new(filters.clone())
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");
        let state = repository
            .load_authorization_request(&request)
            .expect("the redirect stores the request")
            .state()
            .to_owned();

        // The callback answers it, and the grant is exchanged for the profile.
        let body = format!("code=the-code&state={state}");
        let mut callback = Request::builder()
            .method("POST")
            .uri("https://sp.example.org/login/oauth2/code/okta")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("a valid request");
        callback
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(body.into_bytes()));
        let mut callback_response = Response::default();
        Chain::new(filters)
            .run(&mut callback, &mut callback_response)
            .await
            .expect("the chain runs");

        assert!(
            callback_response.status().is_redirection(),
            "the login redirects, status was {}",
            callback_response.status()
        );
        assert_eq!(
            redirect_of(&callback_response),
            Some("/".to_owned()),
            "the login succeeds and sends the user to the application"
        );
    }

    /// Returns the authentication a chain stored on the request, if any.
    fn stored_authentication(
        request: &dyn next_web_core::http::HttpRequest,
    ) -> Option<Arc<dyn crate::core::Authentication>> {
        use next_web_core::any::AnyValue;

        let AnyValue::BoxedValue(value) = request.get_attribute(
            crate::web::context::RequestAttributeSecurityContextRepository::DEFAULT_REQUEST_ATTR_NAME,
        )?
        else {
            return None;
        };
        let context = (**value)
            .as_any()
            .downcast_ref::<Arc<dyn crate::core::context::SecurityContext>>()?;

        context.get_authentication()
    }

    /// A user service that cannot load the profile of the user.
    struct RefusingUserService;

    #[next_web_core::async_trait]
    impl crate::oauth2::OAuth2UserService<crate::oauth2::OAuth2UserRequest, crate::oauth2::OAuth2User>
        for RefusingUserService
    {
        async fn load_user(
            &self,
            _request: &crate::oauth2::OAuth2UserRequest,
        ) -> Result<crate::oauth2::OAuth2User, crate::core::AuthenticationError> {
            Err(crate::core::AuthenticationError::new(
                "the profile of the user is not available",
            ))
        }
    }

    /// Maps the profile of every login to `ROLE_USER`.
    struct RoleUserMapper;

    impl crate::core::authority::mapping::GrantedAuthoritiesMapper for RoleUserMapper {
        fn map_authorities(
            &self,
            _authorities: &[Arc<dyn crate::core::GrantedAuthority>],
        ) -> Vec<Arc<dyn crate::core::GrantedAuthority>> {
            vec![Arc::new(crate::core::authority::SimpleGrantedAuthority::new(
                "ROLE_USER",
            ))]
        }
    }

    /// Returns a chain that answers the profile with the given service and maps
    /// the authorities of the login with the given mapper.
    fn http_security_with(
        repository: Arc<InMemoryAuthorizationRequestRepository>,
        user_service: Arc<
            dyn crate::oauth2::OAuth2UserService<
                crate::oauth2::OAuth2UserRequest,
                crate::oauth2::OAuth2User,
            >,
        >,
        user_authorities_mapper: Option<
            Arc<dyn crate::core::authority::mapping::GrantedAuthoritiesMapper>,
        >,
    ) -> HttpSecurity {
        let mut shared_objects: HashMap<TypeId, Box<dyn next_web_core::any::AnyClone>> =
            HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        http.oauth2_login(|login| {
            login
                .client_registration_repository(Arc::new(
                    InMemoryClientRegistrationRepository::new(vec![registration()]),
                ))
                .authorization_endpoint(|endpoint| {
                    endpoint.authorization_request_repository(repository.clone());
                })
                .token_endpoint(|endpoint| {
                    endpoint.access_token_response_client(Arc::new(FakeTokenClient));
                })
                .user_info_endpoint(|endpoint| {
                    endpoint.user_service(user_service.clone());
                    if let Some(mapper) = user_authorities_mapper.clone() {
                        endpoint.user_authorities_mapper(mapper);
                    }
                });
        });

        http
    }

    /// Runs the redirect that starts the flow, and returns the authorization
    /// request the callback has to answer.
    async fn start_the_flow(filters: Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>>) -> String {
        let mut request = Request::builder()
            .uri("https://sp.example.org/oauth2/authorization/okta")
            .body(Body::empty())
            .expect("a valid request");
        let mut response = Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        assert!(
            response.status().is_redirection(),
            "the authorization endpoint redirects, status was {}",
            response.status()
        );

        redirect_of(&response)
            .expect("the redirect carries the location")
            .split(['?', '&'])
            .find_map(|parameter| parameter.strip_prefix("state="))
            .expect("the redirect carries the state")
            .to_string()
    }

    /// Answers the redirect with the given code and state.
    async fn answer_the_flow(
        filters: Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>>,
        state: &str,
    ) -> (Response, Request) {
        let body = format!("code=the-code&state={state}");
        let mut callback = Request::builder()
            .method("POST")
            .uri("https://sp.example.org/login/oauth2/code/okta")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("a valid request");
        callback
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(body.into_bytes()));
        let mut response = Response::default();
        let _ = Chain::new(filters)
            .run(&mut callback, &mut response)
            .await;

        (response, callback)
    }

    #[tokio::test]
    async fn the_callback_signs_in_as_the_user_the_profile_names() {
        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut http = http_security(repository.clone());
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let state = start_the_flow(filters.clone()).await;
        let (response, callback) = answer_the_flow(filters, &state).await;

        let authentication =
            stored_authentication(&callback).expect("the callback signs the user in");
        assert_eq!(
            authentication.name(),
            "Max",
            "the profile of the user names the user"
        );
        assert!(
            authentication.is_authenticated(),
            "the login authenticates the user"
        );
        assert_eq!(
            authentication.of(),
            TypeId::of::<crate::oauth2::OAuth2LoginAuthenticationToken>(),
            "the stored authentication is the OAuth 2.0 login one"
        );
        assert!(
            response.status().is_redirection(),
            "the login redirects the user to the application, status was {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn the_authorities_of_the_login_are_mapped_by_the_configuration() {
        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut http = http_security_with(
            repository.clone(),
            Arc::new(FakeUserService),
            Some(Arc::new(RoleUserMapper)),
        );
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let state = start_the_flow(filters.clone()).await;
        let (_, callback) = answer_the_flow(filters, &state).await;

        let authentication =
            stored_authentication(&callback).expect("the callback signs the user in");
        let authorities = authentication
            .authorities()
            .iter()
            .filter_map(|authority| authority.authority())
            .map(|authority| authority.to_string())
            .collect::<Vec<_>>();
        assert!(
            authorities.iter().any(|authority| authority == "ROLE_USER"),
            "the mapper of the configuration decides the authorities, they were: {authorities:?}"
        );
    }

    #[tokio::test]
    async fn a_profile_that_cannot_be_loaded_does_not_sign_the_user_in() {
        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut http = http_security_with(
            repository.clone(),
            Arc::new(RefusingUserService),
            None,
        );
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let state = start_the_flow(filters.clone()).await;
        let (response, callback) = answer_the_flow(filters, &state).await;

        assert!(
            stored_authentication(&callback).is_none(),
            "a login whose profile cannot be loaded authenticates nobody"
        );
        assert!(
            !response.status().is_success(),
            "the login reports the failure instead of answering the application, status was {}",
            response.status()
        );
    }

    #[test]
    fn the_login_page_links_the_login_of_the_registration() {
        use crate::config::web::configurers::DefaultLoginPageConfigurer;

        let repository = Arc::new(InMemoryAuthorizationRequestRepository::default());
        let mut shared_objects: HashMap<TypeId, Box<dyn next_web_core::any::AnyClone>> =
            HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        // The page has to be shared before the login configuration initializes,
        // which is the order an application configures its chain in.
        http.with(DefaultLoginPageConfigurer::default(), |_| {});
        http.oauth2_login(|login| {
            login
                .client_registration_repository(Arc::new(
                    InMemoryClientRegistrationRepository::new(vec![registration()]),
                ))
                .user_info_endpoint(|endpoint| {
                    endpoint.user_service(Arc::new(FakeUserService));
                });
        });
        let _chain = http.build();
        let page = http
            .shared_object::<DefaultLoginPageGeneratingFilter>()
            .expect("the login page is shared");

        assert!(
            page.is_enabled(),
            "the page is rendered for an application that only offers the OAuth 2.0 login"
        );
        let links = page.oauth2_authentication_url_to_client_name();
        assert_eq!(
            links.get("/oauth2/authorization/okta").map(String::as_str),
            Some("Okta"),
            "the page links the login of the registration by its name, the links were: {links:?}"
        );
    }
}
