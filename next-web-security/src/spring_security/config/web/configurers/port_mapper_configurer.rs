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
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        port_mapper::{PortMapper, PortMapperImpl},
    },
};

/// Configures a `PortMapper` shared object used by redirect filters
/// (e.g. `HttpsRedirectConfigurer`) to determine port mappings between
/// HTTP and HTTPS.
///
/// Example:
/// ```ignore
/// http.port_mapper(|pm| {
///     pm.http(9090).maps_to(9443);
/// });
/// ```
#[derive(Clone)]
pub struct PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<PortMapperConfigurer<H>, H>>,
{
    port_mapper: Option<Arc<dyn PortMapper>>,

    base_http_configurer: BaseHttpConfigurer<PortMapperConfigurer<H>, H>,
}

impl<H> PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<PortMapperConfigurer<H>, H>>,
{
    /// Provide a custom `PortMapper`. If not set, `PortMapperImpl` is used.
    pub fn port_mapper(mut self, port_mapper: Arc<dyn PortMapper>) -> Self {
        self.port_mapper = Some(port_mapper);
        self
    }

    fn get_port_mapper(&self) -> Arc<dyn PortMapper> {
        self.port_mapper
            .clone()
            .unwrap_or_else(|| Arc::new(PortMapperImpl::new()))
    }
}

impl<H> Default for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<PortMapperConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            port_mapper: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<PortMapperConfigurer<H>, H>> for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<PortMapperConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<PortMapperConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        // Set PortMapper as a shared object for downstream configurers
        let mapper = self.get_port_mapper();
        http.set_shared_object(mapper);
    }

    fn configure(&mut self, _http: &mut H) {
        // No filter to add — this configurer only produces a shared object.
    }
}
