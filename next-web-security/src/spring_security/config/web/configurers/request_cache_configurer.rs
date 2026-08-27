use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    http::{HttpMethod, MediaType},
    traits::required::Required,
    web::accept::{ContentNegotiationStrategy, HeaderContentNegotiationStrategy},
};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{BaseHttpConfigurer, CsrfConfigurer},
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        savedrequest::{
            HttpSessionRequestCache, NullRequestCache, RequestCache, RequestCacheAwareFilter,
        },
        util::matcher::{
            AndRequestMatcher, Builder, MediaTypeRequestMatcher, NegatedRequestMatcher,
            RequestHeaderRequestMatcher, RequestMatcher,
        },
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
    base: BaseHttpConfigurer<Self, H>,
}

impl<H> RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Set a custom `RequestCache`. If not set, defaults to `HttpSessionRequestCache`.
    pub fn request_cache(&mut self, request_cache: Arc<dyn RequestCache>) -> &mut Self {
        self.request_cache = Some(request_cache);
        self
    }

    /// Disables the request cache. A `NullRequestCache` is used so that no request
    /// is saved or replayed.
    pub fn disable(&mut self) {
        self.request_cache = Some(Arc::new(NullRequestCache));
    }

    /// Gets the `RequestCache` to use. If one is defined using `request_cache`, then it
    /// is used. Otherwise, an attempt to find a `RequestCache` shared object is made. If
    /// that fails, an `HttpSessionRequestCache` is used.
    ///
    /// # Arguments
    ///
    /// * `http` - the builder to attempt to find the shared object
    fn get_request_cache(&self, http: &H) -> Arc<dyn RequestCache>
    where
        H: 'static,
    {
        if let Some(cache) = &self.request_cache {
            return cache.clone();
        }
        if let Some(cache) = http.shared_object::<Arc<dyn RequestCache>>() {
            return cache.clone();
        }
        let mut default_cache = HttpSessionRequestCache::default();
        default_cache.set_request_matcher(self.create_default_saved_request_matcher(http));
        Arc::new(default_cache)
    }

    /// Creates the default `RequestMatcher` used by the default `HttpSessionRequestCache`.
    /// Requests matching this matcher are the only ones eligible to be saved and
    /// replayed. The matcher excludes favicon requests, requests with the
    /// `X-Requested-With: XMLHttpRequest` header, websocket requests, and requests
    /// matching JSON, multipart and text/event-stream media types. When CSRF is enabled
    /// only GET requests are saved.
    fn create_default_saved_request_matcher(&self, http: &H) -> Arc<dyn RequestMatcher>
    where
        H: 'static,
    {
        let builder = http
            .shared_object::<Builder>()
            .map(Clone::clone)
            .unwrap_or_default();

        let favicon_request_matcher = builder.matcher(None, "/favicon.*");
        let not_fav_icon = Arc::new(NegatedRequestMatcher::new(favicon_request_matcher));
        let not_x_requested_with = Arc::new(NegatedRequestMatcher::new(
            RequestHeaderRequestMatcher::new("X-Requested-With", Some("XMLHttpRequest".into())),
        ));
        let not_web_socket = Arc::new(NegatedRequestMatcher::new(
            RequestHeaderRequestMatcher::new("Upgrade", Some("websocket".into())),
        ));

        let is_csrf_enabled = http.configurer::<CsrfConfigurer<H>>().is_some();
        let mut matchers: Vec<Arc<dyn RequestMatcher>> = Vec::new();
        if is_csrf_enabled {
            matchers.push(Arc::new(builder.matcher(Some(HttpMethod::GET), "/**")));
        }
        matchers.push(not_fav_icon);
        matchers.push(self.not_matching_media_type(http, MediaType::application_json()));
        matchers.push(not_x_requested_with);
        matchers.push(self.not_matching_media_type(http, MediaType::multipart_form_data()));
        matchers.push(self.not_matching_media_type(http, MediaType::text_event_stream()));
        matchers.push(not_web_socket);

        Arc::new(AndRequestMatcher::new(matchers))
    }

    /// Creates a matcher that matches any request whose media type does not equal the
    /// given media type.
    fn not_matching_media_type(&self, http: &H, media_type: MediaType) -> Arc<dyn RequestMatcher>
    where
        H: 'static,
    {
        let content_negotiation_strategy = http
            .shared_object::<Arc<dyn ContentNegotiationStrategy>>()
            .map(Clone::clone)
            .unwrap_or_else(|| Arc::new(HeaderContentNegotiationStrategy::default()));
        let mut media_request =
            MediaTypeRequestMatcher::with_strategy(content_negotiation_strategy, vec![media_type]);
        media_request.set_ignored_media_types(vec![MediaType::all()]);
        Arc::new(NegatedRequestMatcher::new(media_request))
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for RequestCacheConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        // Set the RequestCache as a shared object
        http.set_shared_object::<Arc<dyn RequestCache>>(self.get_request_cache(http));
    }

    fn configure(&mut self, http: &mut H) {
        let request_cache = self.get_request_cache(http);
        let mut filter = RequestCacheAwareFilter::new(request_cache);
        self.base.get_mut_object().post_process(&mut filter);
        http.add_filter(filter);
    }
}

impl<H> Deref for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Default for RequestCacheConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            request_cache: None,
            base: Default::default(),
        }
    }
}
