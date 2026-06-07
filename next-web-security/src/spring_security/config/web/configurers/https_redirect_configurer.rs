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
        authentication::https_redirect_filter::HttpsRedirectFilter,
        default_security_filter_chain::DefaultSecurityFilterChain,
        port_mapper::PortMapper,
        util::matcher::{OrRequestMatcher, RequestMatcher},
    },
};

/// Configures HTTP-to-HTTPS redirect. Uses `PortMapper` (if available
/// as a shared object) to map ports during redirects.
#[derive(Clone)]
pub struct HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H>>,
{
    request_matchers: Vec<Arc<dyn RequestMatcher>>,
    base_http_configurer: BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H>,
}

impl<H> HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H>>,
{
    pub fn request_matchers(mut self, m: Vec<Arc<dyn RequestMatcher>>) -> Self {
        self.request_matchers.extend(m);
        self
    }
}

impl<H> Default for HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self { request_matchers: Vec::new(), base_http_configurer: Default::default() }
    }
}

impl<H> Required<BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H>>
    for HttpsRedirectConfigurer<H> where H: HttpSecurityBuilder<H>
{
    fn get_object(&self) -> &BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H> { &self.base_http_configurer }
    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<HttpsRedirectConfigurer<H>, H> { &mut self.base_http_configurer }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for HttpsRedirectConfigurer<H>
where H: HttpSecurityBuilder<H>, H: SecurityBuilder<DefaultSecurityFilterChain>
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> { self.base_http_configurer.get_object() }
    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> { self.base_http_configurer.get_mut_object() }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for HttpsRedirectConfigurer<H>
where H: HttpSecurityBuilder<H>
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let mut filter = HttpsRedirectFilter::new();
        if !self.request_matchers.is_empty() {
            filter.set_request_matcher(Arc::new(OrRequestMatcher::new(
                self.request_matchers.clone(),
            )));
        }
        // Read PortMapper from shared objects if available
        if let Some(mapper) = http.get_shared_object::<Arc<dyn PortMapper>>() {
            filter.set_port_mapper(mapper.clone());
        }
        http.add_filter(filter);
    }
}
