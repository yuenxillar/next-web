use std::{fmt, ops::Deref, ops::DerefMut, sync::Arc};

use next_web_core::{traits::required::Required, ApplicationContext};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{error_handling_configurer::ErrorHandlingConfigurer, BaseHttpConfigurer},
            HttpSecurityBuilder,
        },
    },
    oauth2_resource_server::{
        bearer::{
            BearerTokenAuthenticationConverter, BearerTokenRequestMatcher, BearerTokenResolver,
            DefaultBearerTokenResolver,
        },
        entry::{
            BearerTokenAccessDeniedHandler, BearerTokenAuthenticationEntryPoint, DPoPConfigurer,
            ProtectedResourceMetadataConfigurer,
        },
        jwt::JwtConfigurer,
        opaque::OpaqueTokenConfigurer,
    },
    web::{
        access::AccessDeniedHandler,
        authentication::{
            bearer_token_authentication_filter::BearerTokenAuthenticationFilter,
            AuthenticationConverter,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        AuthenticationEntryPoint,
    },
};

/// Adds a bearer-token (JWT or opaque) resource server to an application. The
/// resulting `BearerTokenAuthenticationFilter` authenticates the bearer token
/// presented in the `Authorization` header, and the default
/// `BearerTokenAuthenticationEntryPoint`/`BearerTokenAccessDeniedHandler` are
/// registered with the exception handling infrastructure.
///
/// This is a faithful port of Spring Security's `OAuth2ResourceServerConfigurer`.
#[derive(Clone)]
pub struct OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    authentication_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,
    access_denied_handler: Option<Arc<dyn AccessDeniedHandler>>,
    bearer_token_resolver: Option<Arc<dyn BearerTokenResolver>>,
    authentication_converter: Option<Arc<dyn AuthenticationConverter>>,
    jwt_configurer: Option<JwtConfigurer<H>>,
    opaque_token_configurer: Option<OpaqueTokenConfigurer<H>>,
    dpop_configurer: Option<DPoPConfigurer<H>>,
    protected_resource_metadata_configurer: Option<ProtectedResourceMetadataConfigurer<H>>,
    base: BaseHttpConfigurer<OAuth2ResourceServerConfigurer<H>, H>,
}

impl<H> OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Sets the `AuthenticationEntryPoint` used when a bearer token is missing or
    /// invalid.
    pub fn authentication_entry_point(
        &mut self,
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> &mut Self {
        self.authentication_entry_point = Some(authentication_entry_point);
        self
    }

    /// Sets the `AccessDeniedHandler` used when an authenticated request is missing
    /// the required authority.
    pub fn access_denied_handler(
        &mut self,
        access_denied_handler: Arc<dyn AccessDeniedHandler>,
    ) -> &mut Self {
        self.access_denied_handler = Some(access_denied_handler);
        self
    }

    /// Sets the `BearerTokenResolver` used to extract the bearer token from the
    /// request.
    pub fn bearer_token_resolver(
        &mut self,
        bearer_token_resolver: Arc<dyn BearerTokenResolver>,
    ) -> &mut Self {
        self.bearer_token_resolver = Some(bearer_token_resolver);
        self
    }

    /// Sets the `AuthenticationConverter` used to build the unauthenticated token
    /// from the request.
    pub fn authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) -> &mut Self {
        self.authentication_converter = Some(authentication_converter);
        self
    }

    /// Enables JWT bearer token support with the provided customizer.
    pub fn jwt<F>(&mut self, mut jwt_customizer: F) -> &mut Self
    where
        F: FnMut(&mut JwtConfigurer<H>),
    {
        let configurer = self
            .jwt_configurer
            .get_or_insert_with(JwtConfigurer::default);
        jwt_customizer(configurer);
        self
    }

    /// Enables opaque bearer token support with the provided customizer.
    pub fn opaque_token<F>(&mut self, mut opaque_token_customizer: F) -> &mut Self
    where
        F: FnMut(&mut OpaqueTokenConfigurer<H>),
    {
        let configurer = self
            .opaque_token_configurer
            .get_or_insert_with(OpaqueTokenConfigurer::default);
        opaque_token_customizer(configurer);
        self
    }

    /// Enables OAuth2 DPoP (Demonstrating Proof-of-Possession) support.
    pub fn d_pop<F>(&mut self, mut dpop_customizer: F) -> &mut Self
    where
        F: FnMut(&mut DPoPConfigurer<H>),
    {
        let configurer = self
            .dpop_configurer
            .get_or_insert_with(DPoPConfigurer::default);
        dpop_customizer(configurer);
        self
    }

    /// Publishes the `/.well-known/oauth-protected-resource` document.
    pub fn protected_resource_metadata<F>(&mut self, mut metadata_customizer: F) -> &mut Self
    where
        F: FnMut(&mut ProtectedResourceMetadataConfigurer<H>),
    {
        let configurer = self
            .protected_resource_metadata_configurer
            .get_or_insert_with(ProtectedResourceMetadataConfigurer::default);
        metadata_customizer(configurer);
        self
    }

    /// Creates a new `OAuth2ResourceServerConfigurer` with the given application
    /// context (mirrors the Java constructor).
    pub fn new(_ctx: &ApplicationContext) -> Self {
        Self::default()
    }
}

impl<H> Deref for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<OAuth2ResourceServerConfigurer<H>, H>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.base
    }
}

impl<H> DerefMut for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, http: &mut H) {
        let authentication_manager =
            match http.shared_object::<Arc<dyn crate::authorization::AuthenticationManager>>() {
                Some(authentication_manager) => authentication_manager.clone(),
                None => return,
            };

        let bearer_token_resolver: Arc<dyn BearerTokenResolver> = self
            .bearer_token_resolver
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultBearerTokenResolver::default()));
        let authentication_converter: Arc<dyn AuthenticationConverter> =
            self.authentication_converter.clone().unwrap_or_else(|| {
                Arc::new(BearerTokenAuthenticationConverter::with_default_resolver())
            });
        let authentication_entry_point: Arc<dyn AuthenticationEntryPoint> = self
            .authentication_entry_point
            .clone()
            .unwrap_or_else(|| Arc::new(BearerTokenAuthenticationEntryPoint::default()));
        let access_denied_handler: Arc<dyn AccessDeniedHandler> = self
            .access_denied_handler
            .clone()
            .unwrap_or_else(|| Arc::new(BearerTokenAccessDeniedHandler::default()));

        if let Some(jwt_configurer) = self.jwt_configurer.as_mut() {
            SecurityConfigurer::init(jwt_configurer, http);
        }
        if let Some(opaque_token_configurer) = self.opaque_token_configurer.as_mut() {
            SecurityConfigurer::init(opaque_token_configurer, http);
        }
        if let Some(dpop_configurer) = self.dpop_configurer.as_mut() {
            SecurityConfigurer::init(dpop_configurer, http);
        }
        if let Some(protected_resource_metadata_configurer) =
            self.protected_resource_metadata_configurer.as_mut()
        {
            SecurityConfigurer::init(protected_resource_metadata_configurer, http);
        }

        let mut filter =
            BearerTokenAuthenticationFilter::new(authentication_manager, bearer_token_resolver);
        filter.set_authentication_entry_point(authentication_entry_point.clone());
        filter.set_authentication_converter(authentication_converter);

        if let Some(error_handling) = http.configurer_mut::<ErrorHandlingConfigurer<H>>() {
            error_handling.authentication_entry_point(authentication_entry_point.clone());
            error_handling.access_denied_handler(access_denied_handler.clone());
            error_handling.default_authentication_entry_point_for(
                authentication_entry_point,
                Arc::new(BearerTokenRequestMatcher::default()),
            );
        }

        http.add_filter(filter);
    }

    fn configure(&mut self, http: &mut H) {
        if let Some(jwt_configurer) = self.jwt_configurer.as_mut() {
            SecurityConfigurer::configure(jwt_configurer, http);
        }
        if let Some(opaque_token_configurer) = self.opaque_token_configurer.as_mut() {
            SecurityConfigurer::configure(opaque_token_configurer, http);
        }
        if let Some(dpop_configurer) = self.dpop_configurer.as_mut() {
            SecurityConfigurer::configure(dpop_configurer, http);
        }
        if let Some(protected_resource_metadata_configurer) =
            self.protected_resource_metadata_configurer.as_mut()
        {
            SecurityConfigurer::configure(protected_resource_metadata_configurer, http);
        }
    }
}

impl<H> Default for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            authentication_entry_point: None,
            access_denied_handler: None,
            bearer_token_resolver: None,
            authentication_converter: None,
            jwt_configurer: None,
            opaque_token_configurer: None,
            dpop_configurer: None,
            protected_resource_metadata_configurer: None,
            base: Default::default(),
        }
    }
}

impl<H> fmt::Debug for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuth2ResourceServerConfigurer")
            .field("jwt", &self.jwt_configurer.is_some())
            .field("opaque_token", &self.opaque_token_configurer.is_some())
            .field("dpop", &self.dpop_configurer.is_some())
            .field(
                "protected_resource_metadata",
                &self.protected_resource_metadata_configurer.is_some(),
            )
            .finish()
    }
}
