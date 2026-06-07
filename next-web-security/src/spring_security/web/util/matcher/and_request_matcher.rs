use std::fmt;
use std::sync::Arc;

use indexmap::IndexMap;
use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::util::matcher::{MatchResult, RequestMatcher};

/// A RequestMatcher that will return true if all of the passed in
/// RequestMatcher instances match.
pub struct AndRequestMatcher {
    request_matchers: Vec<Arc<dyn RequestMatcher>>,
}

impl AndRequestMatcher {
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

impl RequestMatcher for AndRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        for matcher in self.request_matchers.iter() {
            if !matcher.matches(request) {
                return false;
            }
        }
        true
    }

    /// Returns a MatchResult for this HttpServletRequest. In the case of a
    /// match, request variables are a composition of the request variables in underlying
    /// matchers. In the event that two matchers have the same key, the last key is the one
    /// propagated.
    ///
    /// # Arguments
    /// * `request` - the HTTP request
    ///
    /// # Returns
    /// a MatchResult based on the given HTTP request
    fn matcher(&self, request: &dyn HttpRequest) -> MatchResult {
        let mut variables = IndexMap::new();
        for matcher in self.request_matchers.iter() {
            let result = matcher.matcher(request);
            if !result.is_match() {
                return MatchResult::not_match();
            }

            result.get_own_variables().map(|v| {
                variables.extend(v);
            });
        }
        MatchResult::match_with_variables(variables)
    }
}

impl PartialEq for AndRequestMatcher {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        if self.request_matchers.len() != other.request_matchers.len() {
            return false;
        }
        // Note: Comparing trait objects for equality is done by pointer comparison
        // This matches the Java behavior of using Objects.equals
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

impl Eq for AndRequestMatcher {}

impl std::hash::Hash for AndRequestMatcher {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for matcher in &self.request_matchers {
            std::ptr::hash(matcher.as_ref() as *const _ as *const (), state);
        }
    }
}

impl fmt::Display for AndRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "And {:?}", self.request_matchers)
    }
}

impl fmt::Debug for AndRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AndRequestMatcher")
            .field("request_matchers", &self.request_matchers)
            .finish()
    }
}
