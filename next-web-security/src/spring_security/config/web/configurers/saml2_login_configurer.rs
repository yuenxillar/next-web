use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Configures SAML 2.0 Service Provider authentication.
///
/// When fully implemented, this will:
/// - Create `Saml2WebSsoAuthenticationRequestFilter` to redirect to the IdP
/// - Create `Saml2WebSsoAuthenticationFilter` to process SAML assertions at the ACS
/// - Register `OpenSaml5AuthenticationProvider`
/// - Integrate with `DefaultLoginPageGeneratingFilter` for auto-generated login links
/// - Disable CSRF on the ACS endpoint
///
/// Requires: `RelyingPartyRegistrationRepository`.
#[derive(Clone)]
pub struct Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    login_page: Option<String>,
    login_processing_url: Option<String>,

    inner: BaseHttpConfigurer<Saml2LoginConfigurer<H>, H>,
}

impl<H> Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Set the login page URL.
    pub fn login_page(mut self, page: &str) -> Self {
        self.login_page = Some(page.to_string());
        self
    }

    /// Set the ACS (Assertion Consumer Service) URL.
    /// Default: `"/login/saml2/sso/{registrationId}"`.
    pub fn login_processing_url(mut self, url: &str) -> Self {
        self.login_processing_url = Some(url.to_string());
        self
    }
}

impl<H> Default for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            login_page: None,
            login_processing_url: None,
            inner: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2LoginConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Requires RelyingPartyRegistrationRepository + SAML2 filters.
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Will create Saml2WebSsoAuthenticationRequestFilter +
        // Saml2WebSsoAuthenticationFilter when SAML2 infrastructure is ready.
    }
}

impl<H> Deref for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
