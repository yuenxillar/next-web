use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        transport::HttpsRedirectFilter,
        util::matcher::{OrRequestMatcher, RequestMatcher},
        PortMapper,
    },
};

/// Specifies for what requests the application should redirect to HTTPS.
///  When this configurer is added, it redirects all HTTP requests by default to HTTPS.
#[derive(Clone)]
pub struct HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    request_matchers: Option<Arc<dyn RequestMatcher>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn request_matchers(&mut self, matchers: Vec<Arc<dyn RequestMatcher>>) -> &mut Self {
        self.request_matchers = Some(Arc::new(OrRequestMatcher::new(matchers)));
        self
    }
}

impl<H> Default for HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            request_matchers: Default::default(),

            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for HttpsRedirectConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let mut filter = HttpsRedirectFilter::default();
        if let Some(matchers) = self.request_matchers.take() {
            filter.set_request_matcher(matchers);
        }
        // Read PortMapper from shared objects if available
        if let Some(mapper) = http.shared_object::<Arc<dyn PortMapper>>() {
            filter.set_port_mapper(mapper.clone());
        }
        http.add_filter(filter);
    }
}

impl<H> Deref for HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for HttpsRedirectConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
