use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{traits::required::Required, ApplicationContext};

use crate::{
    authentication::AuthenticationProvider,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{logout_configurer::LogoutConfigurer, BaseHttpConfigurer},
            HttpSecurityBuilder,
        },
    },
    oauth2::{
        client::{
            oidc_back_channel_logout::{
                EitherLogoutHandler, OAuth2ClientConfigurerUtils,
                OidcBackChannelLogoutAuthenticationProvider, OidcBackChannelLogoutFilter,
                OidcBackChannelLogoutHandler, OidcLogoutAuthenticationConverter,
            },
            OidcSessionRegistry,
        },
        ClientRegistrationRepository,
    },
    oauth2_resource_server::jwt::{JwtDecoder, NimbusJwtDecoder},
    web::{
        authentication::{
            logout::{CompositeLogoutHandler, LogoutHandler, SecurityContextLogoutHandler},
            AuthenticationConverter,
        },
        csrf::CsrfFilter,
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

/// Adds OIDC Logout support, in particular OIDC Back-Channel Logout, to an
/// application. Mirrors Spring Security's `OidcLogoutConfigurer`.
#[derive(Clone)]
pub struct OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    client_registration_repository: Option<Arc<dyn ClientRegistrationRepository>>,
    oidc_session_registry: Option<Arc<dyn OidcSessionRegistry>>,
    back_channel: Option<BackChannelLogoutConfigurer>,
    base: BaseHttpConfigurer<Self, B>,
}

impl<B> OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    pub fn new(_ctx: &ApplicationContext) -> Self {
        Self::default()
    }

    /// Sets the repository of client registrations.
    pub fn client_registration_repository(
        &mut self,
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> &mut Self {
        self.client_registration_repository = Some(client_registration_repository);
        self
    }

    /// Sets the registry for managing the OIDC client-provider session link.
    pub fn oidc_session_registry(
        &mut self,
        oidc_session_registry: Arc<dyn OidcSessionRegistry>,
    ) -> &mut Self {
        self.oidc_session_registry = Some(oidc_session_registry);
        self
    }

    /// Configures OIDC Back-Channel Logout using the provided customizer.
    pub fn back_channel<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut BackChannelLogoutConfigurer),
    {
        if self.back_channel.is_none() {
            self.back_channel = Some(BackChannelLogoutConfigurer::default());
        }
        if let Some(back_channel) = self.back_channel.as_mut() {
            customizer(back_channel);
        }
        self
    }
}

impl<B> Default for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn default() -> Self {
        Self {
            client_registration_repository: None,
            oidc_session_registry: None,
            back_channel: None,
            base: Default::default(),
        }
    }
}

impl<B> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>
    for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
    B: 'static,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        self.base.get_mut_object()
    }
}

impl<B> SecurityConfigurer<DefaultSecurityFilterChain, B> for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
    B: 'static,
{
    fn init(&mut self, http: &mut B) {
        if let Some(repository) = &self.client_registration_repository {
            http.set_shared_object(repository.clone());
        }
        if let Some(registry) = &self.oidc_session_registry {
            http.set_shared_object(registry.clone());
        }
    }

    fn configure(&mut self, http: &mut B) {
        if let Some(back_channel) = self.back_channel.as_mut() {
            back_channel.configure(http);
        }
    }
}

impl<B> Deref for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    type Target = BaseHttpConfigurer<Self, B>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<B> DerefMut for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Configurer for OIDC Back-Channel Logout. Mirrors the nested
/// `BackChannelLogoutConfigurer` of Spring Security's `OidcLogoutConfigurer`.
#[derive(Clone, Default)]
pub struct BackChannelLogoutConfigurer {
    authentication_converter: Option<Arc<dyn AuthenticationConverter>>,
    logout_handler: Option<Arc<dyn LogoutHandler>>,
    logout_uri: Option<String>,
}

impl BackChannelLogoutConfigurer {
    /// Sets the URI this application's back-channel logout endpoint listens on.
    pub fn logout_uri(mut self, logout_uri: &str) -> Self {
        self.logout_uri = Some(logout_uri.to_string());
        self
    }

    /// Sets the `LogoutHandler` used to terminate each individual provider
    /// session. Overrides any value supplied via `logout_uri`.
    pub fn logout_handler(mut self, logout_handler: Arc<dyn LogoutHandler>) -> Self {
        self.logout_handler = Some(logout_handler);
        self
    }

    /// Overrides the `AuthenticationConverter` used to extract the logout token.
    pub fn authentication_converter(
        mut self,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) -> Self {
        self.authentication_converter = Some(authentication_converter);
        self
    }

    fn resolve_logout_handler<B: HttpSecurityBuilder<B> + 'static>(
        &self,
        http: &mut B,
    ) -> Arc<dyn LogoutHandler> {
        if let Some(logout_uri) = &self.logout_uri {
            let registry = OAuth2ClientConfigurerUtils::get_oidc_session_registry(http);
            Arc::new(OidcBackChannelLogoutHandler::with_logout_uri(
                registry,
                logout_uri.clone(),
            ))
        } else if let Some(handler) = &self.logout_handler {
            handler.clone()
        } else {
            let registry = OAuth2ClientConfigurerUtils::get_oidc_session_registry(http);
            Arc::new(OidcBackChannelLogoutHandler::new(registry))
        }
    }

    fn resolve_session_logout<B: HttpSecurityBuilder<B> + 'static>(
        &self,
        http: &mut B,
    ) -> Arc<dyn LogoutHandler> {
        if let Some(logout) = http.configurer::<LogoutConfigurer<B>>() {
            let handlers = logout.get_logout_handlers().to_vec();
            Arc::new(CompositeLogoutHandler::new(handlers))
        } else {
            Arc::new(SecurityContextLogoutHandler::default())
        }
    }

    pub fn configure<B: HttpSecurityBuilder<B> + 'static>(&mut self, http: &mut B) {
        let oidc_logout = self.resolve_logout_handler(http);
        let session_logout = self.resolve_session_logout(http);
        let either = EitherLogoutHandler::new(oidc_logout, session_logout);

        let converter = self.authentication_converter.clone().unwrap_or_else(|| {
            let repository = OAuth2ClientConfigurerUtils::get_client_registration_repository(http);
            Arc::new(OidcLogoutAuthenticationConverter::new(repository))
        });

        let decoder: Arc<dyn JwtDecoder> = Arc::new(NimbusJwtDecoder::default());
        let provider: Arc<dyn AuthenticationProvider> =
            Arc::new(OidcBackChannelLogoutAuthenticationProvider::new(decoder));

        let filter = OidcBackChannelLogoutFilter::new(converter, provider, Arc::new(either));
        http.add_filter_before::<OidcBackChannelLogoutFilter, CsrfFilter>(filter);
    }
}
