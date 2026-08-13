use std::{marker::PhantomData, sync::Arc};

use next_web_core::http::HttpMethod;
use tracing::warn;

use crate::web::util::matcher::{AnyRequestMatcher, Builder, MatcherInput, RequestMatcher};

/// A base class for registering RequestMatcher's.
/// For example, it might allow for specifying which RequestMatcher require a certain level of authorization.
#[derive(Clone)]
pub struct BaseRequestMatcherRegistry<C> {
    any_request_configured: bool,
    request_matcher_builder: Option<Builder>,
    pub(crate) _req_matchers: Vec<Arc<dyn RequestMatcher>>,

    _marker: PhantomData<C>,
}

impl<C> BaseRequestMatcherRegistry<C> {
    /// Maps any request.
    /// Returns the object that is chained after creating the RequestMatcher
    pub fn any_request(&mut self) -> &mut Self {
        assert!(
            !self.any_request_configured,
            "Can't configure anyRequest after itself"
        );

        self._req_matchers
            .extend(vec![AnyRequestMatcher::instance()]);
        self.any_request_configured = true;
        self
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
    pub fn request_matchers<T>(&mut self, matcher: T) -> &mut Self
    where
        T: Into<MatcherInput>,
    {
        match matcher.into() {
            MatcherInput::Matchers(matchers) => self._extend_matchers(matchers),
            MatcherInput::MethodWithPatterns(method, patterns) => {
                self._request_matchers(Some(method), &patterns)
            }
            MatcherInput::Paths(patterns) => self._request_matchers(None, &patterns),
            MatcherInput::Method(method) => self._request_matchers(Some(method), &["/**"]),
        }
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
            .map(|pattern| {
                Arc::new(builder.matcher(method.clone(), pattern)) as Arc<dyn RequestMatcher>
            })
            .collect();
        self._extend_matchers(matchers)
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

pub trait BaseRequestMatcherRegistryExt<C> {
    fn chain_request_matchers(&mut self, request_matchers: Vec<Arc<dyn RequestMatcher>>) -> C;
}

// Helper function
fn any_paths_dont_start_with_leading_slash(patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| !pattern.starts_with('/'))
}
