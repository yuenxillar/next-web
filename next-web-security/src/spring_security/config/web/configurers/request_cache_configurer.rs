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
        savedrequest::{HttpSessionRequestCache, RequestCache, RequestCacheAwareFilter},
    },
};

/// Configures request caching so that a saved request can be replayed after
/// authentication. Activated by default with `@EnableWebSecurity`.
///
/// The default implementation uses `HttpSessionRequestCache` to store the
/// original request before authentication, then replays it afterwards.
#[derive(Clone)]
pub struct RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    request_cache: Option<Arc<dyn RequestCache>>,
    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Set a custom `RequestCache`. If not set, defaults to `HttpSessionRequestCache`.
    pub fn request_cache(mut self, request_cache: Arc<dyn RequestCache>) -> Self {
        self.request_cache = Some(request_cache);
        self
    }

    fn get_request_cache(&self, _http: &H) -> Arc<dyn RequestCache> {
        self.request_cache
            .clone()
            .unwrap_or_else(|| Arc::new(HttpSessionRequestCache::default()))
    }
}

impl<H> Default for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            request_cache: None,
            inner: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for RequestCacheConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        // Set the RequestCache as a shared object
        let cache = self.get_request_cache(http);
        http.set_shared_object(cache);
    }

    fn configure(&mut self, http: &mut H) {
        let request_cache = self.get_request_cache(http);
        let mut filter = RequestCacheAwareFilter::new(request_cache);
        self.inner.get_mut_object().post_process(&mut filter);
        http.add_filter(filter);
    }
}

impl<H> Deref for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
