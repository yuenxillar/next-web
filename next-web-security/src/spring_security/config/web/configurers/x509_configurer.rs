use std::sync::Arc;

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

/// Configures X509 client-certificate based pre-authentication.
///
/// Extracts the subject DN from the client certificate and authenticates
/// via `PreAuthenticatedAuthenticationProvider`.
#[derive(Clone)]
pub struct X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<X509Configurer<H>, H>>,
{
    subject_principal_regex: Option<String>,

    base_http_configurer: BaseHttpConfigurer<X509Configurer<H>, H>,
}

impl<H> X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<X509Configurer<H>, H>>,
{
    /// Set the regex used to extract the principal from the subject DN.
    /// Default: `CN=(.*?)(?:,|$)`.
    pub fn subject_principal_regex(mut self, regex: &str) -> Self {
        self.subject_principal_regex = Some(regex.to_string());
        self
    }

    fn get_subject_principal_regex(&self) -> String {
        self.subject_principal_regex
            .clone()
            .unwrap_or_else(|| "CN=(.*?)(?:,|$)".to_string())
    }
}

impl<H> Default for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<X509Configurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            subject_principal_regex: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<X509Configurer<H>, H>> for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<X509Configurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<X509Configurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for X509Configurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Register PreAuthenticatedAuthenticationProvider when X509AuthenticationFilter
        // is fully implemented. Requires AuthenticationUserDetailsService +
        // UserDetailsChecker wiring.
    }

    fn configure(&mut self, http: &mut H) {
        // X509AuthenticationFilter would be created here.
        // For now, this is a stub — the filter implementation requires
        // client certificate extraction from the TLS layer.
        //
        // When implemented:
        //   let filter = X509AuthenticationFilter::new();
        //   filter.set_principal_extractor(...);
        //   http.add_filter(filter);
    }
}
