use std::sync::Arc;

use next_web_core::traits::{filter::HttpFilter, http::http_request::HttpRequest};
use tracing::debug;

use crate::{web::security_filter_chain::SecurityFilterChain, web::util::matcher::RequestMatcher};

#[derive(Clone)]
pub struct DefaultSecurityFilterChain {
    request_matcher: Arc<dyn RequestMatcher>,
    filters: Vec<Arc<dyn HttpFilter>>,

    #[allow(dead_code)]
    name: Box<str>,
}

impl DefaultSecurityFilterChain {
    pub fn new(
        request_matcher: Arc<dyn RequestMatcher>,
        filters: Vec<Arc<dyn HttpFilter>>,
    ) -> Self {
        if filters.is_empty() {
            debug!("Will not secure {:?}", request_matcher)
        } else {
            let filter_names = filters.iter().map(|f| f.name()).collect::<Vec<_>>();
            debug!(
                "Will secure {:?} with filters: {:?}",
                request_matcher, filter_names
            );
        }

        Self {
            request_matcher,
            filters,
            name: "defaultSecurityFilterChain".into(),
        }
    }
}

impl SecurityFilterChain for DefaultSecurityFilterChain {
    fn matches(&self, request: &mut dyn HttpRequest) -> bool {
        self.request_matcher.matches(request)
    }

    fn get_filters(&self) -> &[Arc<dyn HttpFilter>] {
        &self.filters
    }
}
