use std::{fmt, sync::Arc};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::util::matcher::{MatchResult, RequestMatcher};

/// A RequestMatcher that will return true if any of the passed in
/// RequestMatcher instances match.
pub struct OrRequestMatcher {
    request_matchers: Vec<Arc<dyn RequestMatcher>>,
}

impl OrRequestMatcher {
    /// Creates a new instance
    ///
    /// # Arguments
    /// * `request_matchers` - the RequestMatcher instances to try
    ///
    /// # Panics
    /// Panics if request_matchers is empty or contains null elements
    pub fn new(request_matchers: Vec<Arc<dyn RequestMatcher>>) -> Self {
        assert!(
            !request_matchers.is_empty(),
            "requestMatchers must contain a value"
        );
        assert!(
            request_matchers
                .iter()
                .all(|m| m.as_ref() as *const _ as *const () != std::ptr::null()),
            "requestMatchers cannot contain null values"
        );
        Self { request_matchers }
    }
}

impl RequestMatcher for OrRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        for matcher in &self.request_matchers {
            if matcher.matches(request) {
                return true;
            }
        }
        false
    }

    /// Returns a MatchResult for this HttpServletRequest. In the case of a
    /// match, request variables are any request variables from the first underlying
    /// matcher.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    ///
    /// # Returns
    /// a MatchResult based on the given HTTP request
    fn matcher(&self, request: &dyn HttpRequest) -> MatchResult {
        for matcher in &self.request_matchers {
            let result = matcher.matcher(request);
            if result.is_match() {
                return result;
            }
        }
        MatchResult::not_match()
    }
}

impl PartialEq for OrRequestMatcher {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        if self.request_matchers.len() != other.request_matchers.len() {
            return false;
        }
        self.request_matchers
            .iter()
            .zip(other.request_matchers.iter())
            .all(|(a, b)| {
                std::ptr::eq(
                    a.as_ref() as *const _ as *const (),
                    b.as_ref() as *const _ as *const (),
                )
            })
    }
}

impl Eq for OrRequestMatcher {}

impl std::hash::Hash for OrRequestMatcher {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for matcher in &self.request_matchers {
            std::ptr::hash(matcher.as_ref() as *const _ as *const (), state);
        }
    }
}

impl fmt::Display for OrRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Or {:?}", self.request_matchers)
    }
}

impl fmt::Debug for OrRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrRequestMatcher")
            .field("request_matchers", &self.request_matchers)
            .finish()
    }
}
