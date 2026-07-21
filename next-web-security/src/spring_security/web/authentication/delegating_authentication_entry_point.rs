use std::sync::Arc;

use crate::web::{
    util::matcher::{RequestMatcher, RequestMatcherEntry},
    AuthenticationEntryPoint,
};

pub struct DelegatingAuthenticationEntryPoint {}

pub struct DelegatingAuthenticationEntryPointBuilder {
    entry_points: Vec<RequestMatcherEntry<Arc<dyn AuthenticationEntryPoint>>>,
}

impl DelegatingAuthenticationEntryPointBuilder {
    pub fn add_entry_point_for<T>(
        &mut self,
        entry_point: T,
        request_matcher: Arc<dyn RequestMatcher>,
    ) where
        T: AuthenticationEntryPoint,
        T: 'static,
    {
        self.entry_points.push(RequestMatcherEntry::new(
            request_matcher,
            Arc::new(entry_point),
        ));
    }
}
