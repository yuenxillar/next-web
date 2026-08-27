use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use next_web_core::traits::required::Required;

use crate::{
    authorization::AuthenticationManager,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    core::context::SecurityContextHolderStrategy,
    oauth2::{
        AuthenticatedPrincipalOAuth2AuthorizedClientRepository, AuthorizationRequestRepository,
        ClientRegistrationRepository, DefaultOAuth2AuthorizationRequestResolver,
        NullOAuth2AuthorizedClientService, OAuth2AccessTokenResponseClient,
        OAuth2AuthorizedClientRepository, OAuth2AuthorizationCodeAuthenticationProvider,
        OAuth2AuthorizationCodeGrantRequest, OAuth2AuthorizationRequestResolver,
        RestClientAuthorizationCodeTokenResponseClient,
    },
    web::{
        authentication::{
            OAuth2AuthorizationCodeGrantFilter, OAuth2AuthorizationRequestRedirectFilter,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        savedrequest::RequestCache,
        RedirectStrategy,
    },
};

/// Configures OAuth 2.0 Client support (without login/authentication).
///
/// This configurer enables an application to act as an OAuth 2.0 Client, i.e.
/// to obtain and use access tokens to call protected resources on behalf of a
/// user. It is a faithful port of Spring Security's `OAuth2ClientConfigurer`
/// and delegates all behaviour to its inner `AuthorizationCodeGrantConfigurer`.
///
/// The following security filters are registered:
/// - [`OAuth2AuthorizationRequestRedirectFilter`]
/// - [`OAuth2AuthorizationCodeGrantFilter`]
///
/// And the following shared objects are created/used:
/// - [`ClientRegistrationRepository`] (required)
/// - [`OAuth2AuthorizedClientRepository`] (optional)
#[derive(Clone)]
pub struct OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    authorization_code_grant_configurer: AuthorizationCodeGrantConfigurer,
    client_registration_repository: Option<Arc<dyn ClientRegistrationRepository>>,
    authorized_client_repository: Option<Arc<dyn OAuth2AuthorizedClientRepository>>,
    base: BaseHttpConfigurer<OAuth2ClientConfigurer<H>, H>,
}

impl<H> Default for OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            authorization_code_grant_configurer: AuthorizationCodeGrantConfigurer::default(),
            client_registration_repository: None,
            authorized_client_repository: None,
            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, http: &mut H) {
        if let Some(client_registration_repository) = &self.client_registration_repository {
            http.set_shared_object::<Arc<dyn ClientRegistrationRepository>>(
                client_registration_repository.clone(),
            );
        }
        if let Some(authorized_client_repository) = &self.authorized_client_repository {
            http.set_shared_object::<Arc<dyn OAuth2AuthorizedClientRepository>>(
                authorized_client_repository.clone(),
            );
        }
        self.authorization_code_grant_configurer.init(http);
    }

    fn configure(&mut self, http: &mut H) {
        self.authorization_code_grant_configurer.configure(
            http,
            self.client_registration_repository.clone(),
            self.authorized_client_repository.clone(),
            self.base.get_security_context_holder_strategy().clone(),
        );
    }
}

impl<H> Deref for OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> OAuth2ClientConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Sets the repository of client registrations.
    pub fn client_registration_repository(
        &mut self,
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> &mut Self {
        self.client_registration_repository = Some(client_registration_repository);
        self
    }

    /// Sets the repository for authorized client(s).
    pub fn authorized_client_repository(
        &mut self,
        authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    ) -> &mut Self {
        self.authorized_client_repository = Some(authorized_client_repository);
        self
    }

    /// Sets the service for authorized client(s).
    pub fn authorized_client_service(
        &mut self,
        authorized_client_service: Arc<dyn crate::oauth2::OAuth2AuthorizedClientService>,
    ) -> &mut Self {
        self.authorized_client_repository(Arc::new(
            AuthenticatedPrincipalOAuth2AuthorizedClientRepository::new(authorized_client_service),
        ))
    }

    /// Configures the OAuth 2.0 Authorization Code Grant.
    pub fn authorization_code_grant<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut AuthorizationCodeGrantConfigurer),
    {
        customizer(&mut self.authorization_code_grant_configurer);
        self
    }
}

/// Configuration options for the OAuth 2.0 Authorization Code Grant.
///
/// Mirrors the inner `AuthorizationCodeGrantConfigurer` of Spring Security's
/// `OAuth2ClientConfigurer`.
#[derive(Clone)]
pub struct AuthorizationCodeGrantConfigurer {
    authorization_request_resolver: Option<Arc<dyn OAuth2AuthorizationRequestResolver>>,
    authorization_request_repository: Option<Arc<dyn AuthorizationRequestRepository>>,
    authorization_redirect_strategy: Option<Arc<dyn RedirectStrategy>>,
    access_token_response_client:
        Option<Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>>,
}

impl Default for AuthorizationCodeGrantConfigurer {
    fn default() -> Self {
        Self {
            authorization_request_resolver: None,
            authorization_request_repository: None,
            authorization_redirect_strategy: None,
            access_token_response_client: None,
        }
    }
}

impl AuthorizationCodeGrantConfigurer {
    /// Sets the resolver used for resolving `OAuth2AuthorizationRequest`s.
    pub fn authorization_request_resolver(
        &mut self,
        authorization_request_resolver: Arc<dyn OAuth2AuthorizationRequestResolver>,
    ) -> &mut Self {
        self.authorization_request_resolver = Some(authorization_request_resolver);
        self
    }

    /// Sets the repository used for storing `OAuth2AuthorizationRequest`s.
    pub fn authorization_request_repository(
        &mut self,
        authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    ) -> &mut Self {
        self.authorization_request_repository = Some(authorization_request_repository);
        self
    }

    /// Sets the redirect strategy for the Authorization Endpoint redirect URI.
    pub fn authorization_redirect_strategy(
        &mut self,
        authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
    ) -> &mut Self {
        self.authorization_redirect_strategy = Some(authorization_redirect_strategy);
        self
    }

    /// Sets the client used for requesting the access token credential from the
    /// Token Endpoint.
    pub fn access_token_response_client(
        &mut self,
        access_token_response_client: Arc<
            dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>,
        >,
    ) -> &mut Self {
        self.access_token_response_client = Some(access_token_response_client);
        self
    }

    fn init<H>(&mut self, http: &mut H)
    where
        H: HttpSecurityBuilder<H>,
    {
        let access_token_response_client = self.get_access_token_response_client();
        let authorization_code_authentication_provider =
            OAuth2AuthorizationCodeAuthenticationProvider::new(access_token_response_client);
        http.authentication_provider(Arc::new(authorization_code_authentication_provider));
    }

    fn configure<H>(
        &mut self,
        http: &mut H,
        outer_client_registration_repository: Option<Arc<dyn ClientRegistrationRepository>>,
        outer_authorized_client_repository: Option<Arc<dyn OAuth2AuthorizedClientRepository>>,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) where
        H: HttpSecurityBuilder<H>,
    {
        let authorization_request_redirect_filter =
            self.create_authorization_request_redirect_filter(
                http,
                &outer_client_registration_repository,
            );
        http.add_filter(authorization_request_redirect_filter);

        let authorization_code_grant_filter = self.create_authorization_code_grant_filter(
            http,
            &outer_client_registration_repository,
            &outer_authorized_client_repository,
            security_context_holder_strategy,
        );
        http.add_filter(authorization_code_grant_filter);
    }

    fn create_authorization_request_redirect_filter<H>(
        &self,
        http: &H,
        outer_client_registration_repository: &Option<Arc<dyn ClientRegistrationRepository>>,
    ) -> OAuth2AuthorizationRequestRedirectFilter
    where
        H: HttpSecurityBuilder<H>,
    {
        let authorization_request_resolver = self.get_authorization_request_resolver(
            http,
            outer_client_registration_repository,
        );
        let mut authorization_request_redirect_filter =
            OAuth2AuthorizationRequestRedirectFilter::new(authorization_request_resolver);
        if let Some(authorization_request_repository) = &self.authorization_request_repository {
            authorization_request_redirect_filter
                .set_authorization_request_repository(authorization_request_repository.clone());
        }
        if let Some(authorization_redirect_strategy) = &self.authorization_redirect_strategy {
            authorization_request_redirect_filter
                .set_authorization_redirect_strategy(authorization_redirect_strategy.clone());
        }
        if let Some(request_cache) = http.shared_object::<Arc<dyn RequestCache>>() {
            authorization_request_redirect_filter.set_request_cache(request_cache.clone());
        }
        authorization_request_redirect_filter
    }

    fn create_authorization_code_grant_filter<H>(
        &self,
        http: &H,
        outer_client_registration_repository: &Option<Arc<dyn ClientRegistrationRepository>>,
        outer_authorized_client_repository: &Option<Arc<dyn OAuth2AuthorizedClientRepository>>,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) -> OAuth2AuthorizationCodeGrantFilter
    where
        H: HttpSecurityBuilder<H>,
    {
        let authentication_manager = http
            .shared_object::<Arc<dyn AuthenticationManager>>()
            .cloned()
            .expect("AuthenticationManager is required for oauth2 client");
        let client_registration_repository =
            self.get_client_registration_repository(http, outer_client_registration_repository);
        let authorized_client_repository = self.get_authorized_client_repository(
            http,
            outer_authorized_client_repository,
        );
        let mut authorization_code_grant_filter = OAuth2AuthorizationCodeGrantFilter::new(
            client_registration_repository,
            authorized_client_repository,
        );
        if let Some(authorization_request_repository) = &self.authorization_request_repository {
            authorization_code_grant_filter
                .set_authorization_request_repository(authorization_request_repository.clone());
        }
        authorization_code_grant_filter
            .set_security_context_holder_strategy(security_context_holder_strategy);
        if let Some(request_cache) = http.shared_object::<Arc<dyn RequestCache>>() {
            authorization_code_grant_filter.set_request_cache(request_cache.clone());
        }
        authorization_code_grant_filter.set_authentication_manager(authentication_manager);
        authorization_code_grant_filter
    }

    fn get_authorization_request_resolver<H>(
        &self,
        http: &H,
        outer_client_registration_repository: &Option<Arc<dyn ClientRegistrationRepository>>,
    ) -> Arc<dyn OAuth2AuthorizationRequestResolver>
    where
        H: HttpSecurityBuilder<H>,
    {
        if let Some(authorization_request_resolver) = &self.authorization_request_resolver {
            return authorization_request_resolver.clone();
        }
        let client_registration_repository =
            self.get_client_registration_repository(http, outer_client_registration_repository);
        Arc::new(DefaultOAuth2AuthorizationRequestResolver::new(
            client_registration_repository,
            OAuth2AuthorizationRequestRedirectFilter::DEFAULT_AUTHORIZATION_REQUEST_BASE_URI,
        ))
    }

    fn get_access_token_response_client(
        &self,
    ) -> Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>> {
        self.access_token_response_client
            .clone()
            .unwrap_or_else(|| Arc::new(RestClientAuthorizationCodeTokenResponseClient::default()))
    }

    fn get_client_registration_repository<H>(
        &self,
        http: &H,
        outer_client_registration_repository: &Option<Arc<dyn ClientRegistrationRepository>>,
    ) -> Arc<dyn ClientRegistrationRepository>
    where
        H: HttpSecurityBuilder<H>,
    {
        outer_client_registration_repository
            .clone()
            .or_else(|| http.shared_object::<Arc<dyn ClientRegistrationRepository>>().cloned())
            .expect("ClientRegistrationRepository is required for oauth2_client")
    }

    fn get_authorized_client_repository<H>(
        &self,
        http: &H,
        outer_authorized_client_repository: &Option<Arc<dyn OAuth2AuthorizedClientRepository>>,
    ) -> Arc<dyn OAuth2AuthorizedClientRepository>
    where
        H: HttpSecurityBuilder<H>,
    {
        outer_authorized_client_repository
            .clone()
            .or_else(|| http.shared_object::<Arc<dyn OAuth2AuthorizedClientRepository>>().cloned())
            .unwrap_or_else(|| {
                Arc::new(AuthenticatedPrincipalOAuth2AuthorizedClientRepository::new(Arc::new(
                    NullOAuth2AuthorizedClientService::default(),
                )))
            })
    }
}
