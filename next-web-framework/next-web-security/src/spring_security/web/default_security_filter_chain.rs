use std::sync::Arc;

use crate::{
    config::web::util::matcher::request_matcher::RequestMatcher, core::filter::Filter,
    web::security_filter_chain::SecurityFilterChain,
};

#[derive(Clone)]
pub struct DefaultSecurityFilterChain {
    request_matcher: Arc<dyn RequestMatcher>,
    filters: Vec<Arc<dyn Filter>>,
    name: Box<str>,
}

impl DefaultSecurityFilterChain {
    pub fn new<T, F>(request_matcher: T, filters: F) -> Self
    where
        T: RequestMatcher + 'static,
        F: IntoIterator<Item = Arc<dyn Filter>>,
    {

        let filters = filters.into_iter().collect::<Vec<_>>();
        Self {
            request_matcher: Arc::new(request_matcher),
            filters,
            name: "defaultSecurityFilterChain".into(),
        }
    }
}

impl SecurityFilterChain for DefaultSecurityFilterChain {
    fn matches(&self, request: &axum::extract::Request) -> bool {
        self.request_matcher.matches(request)
    }

    fn get_filters(&self) -> Vec<&dyn Filter> {
        self.filters.iter().map(|f| f.as_ref()).collect()
    }
}
