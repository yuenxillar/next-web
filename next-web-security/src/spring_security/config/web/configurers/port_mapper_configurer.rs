use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

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
    web::{default_security_filter_chain::DefaultSecurityFilterChain, PortMapper, PortMapperImpl},
};

/// Allows configuring a shared PortMapper instance used to determine the
/// ports when redirecting between HTTP and HTTPS. The PortMapper can be obtained from HttpSecurity::shared_object(Self).
#[derive(Clone)]
pub struct PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    port_mapper: Option<Arc<dyn PortMapper>>,
    https_port_mappings: HashMap<String, String>,

    base: BaseHttpConfigurer<PortMapperConfigurer<H>, H>,
}

impl<H> PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Allows specifying the PortMapper instance.
    pub fn port_mapper(&mut self, port_mapper: Arc<dyn PortMapper>) -> &mut Self {
        self.port_mapper = Some(port_mapper);

        self
    }

    pub fn http<'a>(&'a mut self, http_port: u16) -> HttpPortMapping<'a, H> {
        HttpPortMapping::new(self, http_port)
    }

    /// Gets the PortMapper to use. If portMapper(PortMapper) was not invoked,
    /// builds a PortMapperImpl using the port mappings specified with http(int).
    fn get_port_mapper(&mut self) -> Arc<dyn PortMapper> {
        match self.port_mapper.as_ref().cloned() {
            Some(port_mapper) => port_mapper,
            None => {
                let mut var = PortMapperImpl::default();
                var.set_port_mappings(&self.https_port_mappings);
                let port_mapper = Arc::new(var);
                self.port_mapper.replace(port_mapper.clone());

                port_mapper
            }
        }
    }
}

impl<H> Default for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            port_mapper: None,
            https_port_mappings: Default::default(),

            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for PortMapperConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        http.set_shared_object(self.get_port_mapper());
    }

    fn configure(&mut self, _http: &mut H) {}
}

/// Allows specifying the HTTPS port for a given HTTP port when redirecting between
/// HTTP and HTTPS.
pub struct HttpPortMapping<'a, H: HttpSecurityBuilder<H>> {
    configurer: &'a mut PortMapperConfigurer<H>,
    http_port: u16,
}

impl<'a, H: HttpSecurityBuilder<H>> HttpPortMapping<'a, H> {
    /// Creates a new instance.
    ///
    /// # Arguments
    ///
    /// * `configurer` - the parent `PortMapperConfigurer`
    /// * `http_port` - the HTTP port
    fn new(configurer: &'a mut PortMapperConfigurer<H>, http_port: u16) -> Self {
        HttpPortMapping {
            configurer,
            http_port,
        }
    }

    /// Maps the given HTTP port to the provided HTTPS port and vice versa.
    ///
    /// # Arguments
    ///
    /// * `https_port` - the HTTPS port to map to
    ///
    /// # Returns
    ///
    /// The `PortMapperConfigurer` for further customization.
    pub fn maps_to(&'a mut self, https_port: u16) -> &'a mut PortMapperConfigurer<H> {
        self.configurer
            .https_port_mappings
            .insert(self.http_port.to_string(), https_port.to_string());

        self.configurer
    }
}

impl<H> Deref for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for PortMapperConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
