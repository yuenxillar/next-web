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

/// Configures SAML 2.0 metadata endpoint.
///
/// When fully implemented, this will:
/// - Create `Saml2MetadataFilter` serving metadata at configurable endpoints
///   (default: `/saml2/metadata`, `/saml2/metadata/{registrationId}`,
///   `/saml2/service-provider-metadata/{registrationId}`)
/// - Serve per-registration or aggregated `EntityDescriptor` XML
///
/// Requires: `RelyingPartyRegistrationRepository` (from `Saml2LoginConfigurer` or bean).
#[derive(Clone)]
pub struct Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H>>,
{
    metadata_url: Option<String>,

    base_http_configurer: BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H>,
}

impl<H> Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H>>,
{
    /// Set the metadata endpoint URL. Supports `{registrationId}` placeholder.
    /// Default: `"/saml2/metadata"`.
    pub fn metadata_url(mut self, url: &str) -> Self {
        self.metadata_url = Some(url.to_string());
        self
    }
}

impl<H> Default for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            metadata_url: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H>>
    for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<Saml2MetadataConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2MetadataConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H>
    for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Requires Saml2MetadataFilter + RelyingPartyRegistrationRepository.
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Will create Saml2MetadataFilter when SAML2 infra is ready.
    }
}
