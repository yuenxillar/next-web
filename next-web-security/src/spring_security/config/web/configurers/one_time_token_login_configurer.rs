use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{traits::required::Required, ApplicationContext};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    core::userdetails::UserDetailsService,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Configures One-Time Token (OTT) Login.
///
/// When fully implemented, this configurer will:
/// - Create `GenerateOneTimeTokenFilter` to handle token generation requests
/// - Create `OneTimeTokenAuthenticationFilter` to authenticate with the token
/// - Create `DefaultOneTimeTokenSubmitPageGeneratingFilter` for the submit page
/// - Register `OneTimeTokenAuthenticationProvider`
/// - Integrate with `DefaultLoginPageGeneratingFilter` for auto-generated login links
///
/// Requires: `OneTimeTokenService` and `OneTimeTokenGenerationSuccessHandler`.
#[derive(Clone)]
pub struct OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    login_processing_url: Option<String>,
    token_generating_url: Option<String>,
    user_details_service: Option<Arc<dyn UserDetailsService>>,

    inner: BaseHttpConfigurer<OneTimeTokenLoginConfigurer<H>, H>,
}

impl<H> OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new(ctx: &ApplicationContext) -> Self {
        Self::default()
    }

    /// Set the login processing URL. Default: `"/login/ott"`.
    pub fn login_processing_url(mut self, url: &str) -> Self {
        self.login_processing_url = Some(url.to_string());
        self
    }

    /// Set the token generating URL. Default: `"/ott/generate"`.
    pub fn token_generating_url(mut self, url: &str) -> Self {
        self.token_generating_url = Some(url.to_string());
        self
    }

    /// Set the `UserDetailsService` used to look up users after token verification.
    pub fn user_details_service(mut self, uds: Arc<dyn UserDetailsService>) -> Self {
        self.user_details_service = Some(uds);
        self
    }
}

impl<H> Default for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            login_processing_url: None,
            token_generating_url: None,
            user_details_service: None,
            inner: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Register OneTimeTokenAuthenticationProvider when the full OTT
        // infrastructure (OneTimeTokenService, web filters) is available.
        // Requires: OneTimeTokenService + UserDetailsService.
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Requires OTT web filters not yet implemented.
        // When ready:
        //   let generate_filter = GenerateOneTimeTokenFilter::new(...);
        //   http.add_filter(generate_filter);
        //   let auth_filter = OneTimeTokenAuthenticationFilter::new(...);
        //   http.add_filter(auth_filter);
    }
}

impl<H> Deref for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
