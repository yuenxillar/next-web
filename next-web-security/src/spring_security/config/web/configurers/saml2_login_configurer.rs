use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
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
    Self: Required<BaseHttpConfigurer<Saml2LoginConfigurer<H>, H>>,
{
    login_page: Option<String>,
    login_processing_url: Option<String>,

    base_http_configurer: BaseHttpConfigurer<Saml2LoginConfigurer<H>, H>,
}

impl<H> Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2LoginConfigurer<H>, H>>,
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
    Self: Required<BaseHttpConfigurer<Saml2LoginConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            login_page: None,
            login_processing_url: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<Saml2LoginConfigurer<H>, H>>
    for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<Saml2LoginConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<Saml2LoginConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
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
