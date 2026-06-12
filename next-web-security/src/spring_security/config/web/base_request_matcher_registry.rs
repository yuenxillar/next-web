use std::{marker::PhantomData, sync::Arc};

use next_web_core::{util::http_method::HttpMethod, ApplicationContext};
use tracing::warn;

use crate::web::util::matcher::{AnyRequestMatcher, Builder, RequestMatcher};

#[derive(Clone)]
pub struct BaseRequestMatcherRegistry<C> {
    context: Option<ApplicationContext>,

    any_request_configured: bool,
    request_matcher_builder: Option<Builder>,

    pub(crate) _req_matchers: Vec<Arc<dyn RequestMatcher>>,

    _marker: PhantomData<C>,
}

impl<C> BaseRequestMatcherRegistry<C> {}

impl<C> Default for BaseRequestMatcherRegistry<C> {
    fn default() -> Self {
        Self {
            context: None,
            any_request_configured: false,
            request_matcher_builder: None,

            _marker: PhantomData,
            _req_matchers: Default::default(),
        }
    }
}

impl<C> BaseRequestMatcherRegistry<C> {
    /// Sets the ApplicationContext
    pub fn set_application_context(&mut self, context: ApplicationContext) {
        self.context = Some(context);
    }

    /// Gets the ApplicationContext
    pub fn application_context(&self) -> Option<&ApplicationContext> {
        self.context.as_ref()
    }

    /// Maps any request.
    /// Returns the object that is chained after creating the RequestMatcher
    pub fn any_request(&mut self) -> &mut Self {
        assert!(
            !self.any_request_configured,
            "Can't configure anyRequest after itself"
        );

        self.any_request_configured = true;
        self._request_matchers(vec![AnyRequestMatcher::instance()])
    }

    /// Associates a list of RequestMatcher instances with the AbstractRequestMatcherRegistry
    /// request_matchers - the RequestMatcher instances
    /// Returns the object that is chained after creating the RequestMatcher
    fn _request_matchers(&mut self, request_matchers: Vec<Arc<dyn RequestMatcher>>) -> &mut Self {
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
    pub fn request_matchers(&mut self, method: Option<HttpMethod>, patterns: &[&str]) -> &mut Self {
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
        self._request_matchers(matchers)
    }

    /// Match when the request URI matches one of `patterns`.
    /// See PathPattern for matching rules.
    /// If a specific RequestMatcher must be specified, use request_matchers(RequestMatcher...) instead
    /// patterns - the patterns to match on
    /// Returns the object that is chained after creating the RequestMatcher.
    pub fn request_matchers_with_patterns(&mut self, patterns: &[&str]) -> &mut Self {
        self.request_matchers(None, patterns)
    }

    /// Match when the HttpMethod is `method`
    /// If a specific RequestMatcher must be specified, use request_matchers(RequestMatcher...) instead
    /// method - the HttpMethod to use or None for any HttpMethod.
    /// Returns the object that is chained after creating the RequestMatcher.
    pub fn request_matchers_with_method(&mut self, method: HttpMethod) -> &mut Self {
        self.request_matchers(Some(method), &["/**"])
    }

    fn get_request_matcher_builder(&mut self) -> &Builder {
        if self.request_matcher_builder.is_none() {
            let builder = self
                .context
                .as_ref()
                .map(|ctx| ctx.get_single::<Builder>())
                .cloned()
                .expect("Builder not found. Ensure it is registered with the ApplicationContext.");
            self.request_matcher_builder = Some(builder);
        }

        self.request_matcher_builder
            .as_ref()
            .expect("Builder not found. Ensure it is registered with the ApplicationContext.")
    }
}

// Helper function
fn any_paths_dont_start_with_leading_slash(patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| !pattern.starts_with('/'))
}
