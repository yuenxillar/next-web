use std::{fmt, sync::Arc};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::util::matcher::RequestMatcher;

/// A RequestMatcher that will negate the RequestMatcher passed in. For
/// example, if the RequestMatcher passed in returns true,
/// NegatedRequestMatcher will return false. If the RequestMatcher passed
/// in returns false, NegatedRequestMatcher will return true.
pub struct NegatedRequestMatcher {
    request_matcher: Arc<dyn RequestMatcher>,
}

impl NegatedRequestMatcher {
    /// Creates a new instance
    ///
    /// # Arguments
    /// * `request_matcher` - the RequestMatcher that will be negated.
    ///
    pub fn new<T>(request_matcher: T) -> Self
    where
        T: RequestMatcher,
        T: 'static,
    {
        Self {
            request_matcher: Arc::new(request_matcher),
        }
    }
}

impl RequestMatcher for NegatedRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        !self.request_matcher.matches(request)
    }
}

impl PartialEq for NegatedRequestMatcher {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        std::ptr::eq(
            self.request_matcher.as_ref() as *const _ as *const (),
            other.request_matcher.as_ref() as *const _ as *const (),
        )
    }
}

impl Eq for NegatedRequestMatcher {}

impl std::hash::Hash for NegatedRequestMatcher {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(
            self.request_matcher.as_ref() as *const _ as *const (),
            state,
        );
    }
}

impl fmt::Display for NegatedRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not [{:?}]", self.request_matcher)
    }
}

impl fmt::Debug for NegatedRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NegatedRequestMatcher")
            .field("request_matcher", &self.request_matcher)
            .finish()
    }
}
