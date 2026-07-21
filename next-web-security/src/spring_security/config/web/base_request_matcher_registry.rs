use std::{any::Any, marker::PhantomData, sync::Arc};

use next_web_core::util::http_method::HttpMethod;
use tracing::warn;

use crate::web::util::matcher::{AnyRequestMatcher, Builder, RequestMatcher};

#[derive(Clone)]
pub struct BaseRequestMatcherRegistry<C> {
    any_request_configured: bool,
    request_matcher_builder: Option<Builder>,

    pub(crate) _req_matchers: Vec<Arc<dyn RequestMatcher>>,

    _marker: PhantomData<C>,
}

impl<C> BaseRequestMatcherRegistry<C> {}

impl<C> Default for BaseRequestMatcherRegistry<C> {
    fn default() -> Self {
        Self {
            any_request_configured: false,
            request_matcher_builder: None,
            _req_matchers: Default::default(),

            _marker: PhantomData,
        }
    }
}

impl<C> BaseRequestMatcherRegistry<C> {
    /// Maps any request.
    /// Returns the object that is chained after creating the RequestMatcher
    pub fn any_request(&mut self) -> &mut Self {
        assert!(
            !self.any_request_configured,
            "Can't configure anyRequest after itself"
        );

        self.any_request_configured = true;
        self._extend_matchers(vec![AnyRequestMatcher::instance()])
    }

    /// Associates a list of RequestMatcher instances with the AbstractRequestMatcherRegistry
    /// request_matchers - the RequestMatcher instances
    /// Returns the object that is chained after creating the RequestMatcher
    fn _extend_matchers(&mut self, request_matchers: Vec<Arc<dyn RequestMatcher>>) -> &mut Self {
        assert!(
            !self.any_request_configured,
            "Can't configure requestMatchers after anyRequest"
        );

        self._req_matchers.extend(request_matchers);
        self
    }

    /// Match when the HttpMethod is `method` and when the request URI matches one of `patterns`.
    /// See PathPattern for matching rules.
    /// If a specific RequestMatcher must be specified, use request_matchers(RequestMatcher...) instead
    /// method - the HttpMethod to use or None for any HttpMethod.
    /// patterns - the patterns to match on
    /// Returns the object that is chained after creating the RequestMatcher.
    pub fn request_matchers<T>(&mut self, mut matcher: T) -> &mut Self
    where
        T: Any,
    {
        let any = &mut matcher as &mut dyn Any;

        let (http_method, patterns) =
            if let Some(matchers) = any.downcast_mut::<Vec<Arc<dyn RequestMatcher>>>() {
                return self._extend_matchers(std::mem::take(matchers));
            } else if let Some((http_method, patterns)) =
                any.downcast_ref::<(HttpMethod, Vec<&'static str>)>()
            {
                (Some(*http_method), patterns.to_owned())
            } else if let Some(patterns) = any.downcast_ref::<Vec<&'static str>>() {
                (None, patterns.to_owned())
            } else if let Some(http_method) = any.downcast_ref::<HttpMethod>() {
                (Some(*http_method), vec!["/**"])
            } else {
                unimplemented!("Unsupported matcher type: {}", std::any::type_name::<T>())
            };

        self._request_matchers(http_method, &patterns)
    }

    fn _request_matchers(&mut self, method: Option<HttpMethod>, patterns: &[&str]) -> &mut Self {
        if any_paths_dont_start_with_leading_slash(patterns) {
            warn!(
                "One of the patterns in {:?} is missing a leading slash. This is discouraged; \
                 please include the leading slash in all your request matcher patterns. \
                 Leaving out the leading slash \
                 will result in an exception.",
                patterns
            );
        }
        assert!(
            !self.any_request_configured,
            "Can't configure requestMatchers after anyRequest"
        );
        let builder = self.get_request_matcher_builder();
        let matchers = patterns
            .iter()
            .map(|pattern| Arc::new(builder.matcher(method, pattern)) as Arc<dyn RequestMatcher>)
            .collect();
        self._extend_matchers(matchers)
    }

    /// Match when the request URI matches one of `patterns`.
    /// See PathPattern for matching rules.
    /// If a specific RequestMatcher must be specified, use request_matchers(RequestMatcher...) instead
    /// patterns - the patterns to match on
    /// Returns the object that is chained after creating the RequestMatcher.
    pub fn request_matchers_with_patterns(&mut self, patterns: &[&str]) -> &mut Self {
        self._request_matchers(None, patterns)
    }

    /// Match when the HttpMethod is `method`
    /// If a specific RequestMatcher must be specified, use request_matchers(RequestMatcher...) instead
    /// method - the HttpMethod to use or None for any HttpMethod.
    /// Returns the object that is chained after creating the RequestMatcher.
    pub fn request_matchers_with_method(&mut self, method: HttpMethod) -> &mut Self {
        self._request_matchers(Some(method), &["/**"])
    }

    /// Sets the request matcher builder to use for creating RequestMatchers.
    pub fn set_request_matcher_builder(&mut self, builder: Builder) {
        self.request_matcher_builder = Some(builder);
    }

    /// Takes the request matchers from this registry and returns them.
    pub fn take_request_matchers(&mut self) -> Vec<Arc<dyn RequestMatcher>> {
        std::mem::take(&mut self._req_matchers)
    }

    fn get_request_matcher_builder(&mut self) -> &Builder {
        self.request_matcher_builder
            .as_ref()
            .expect("Builder not found. Ensure it is registered with the ApplicationContext.")
    }
}

pub trait BaseRequestMatcherRegistryExt<C> {
    fn chain_request_matchers(&mut self, request_matchers: Vec<Arc<dyn RequestMatcher>>) -> C;
}

// Helper function
fn any_paths_dont_start_with_leading_slash(patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| !pattern.starts_with('/'))
}
