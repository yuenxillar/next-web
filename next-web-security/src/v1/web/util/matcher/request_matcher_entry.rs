use std::sync::Arc;

use crate::web::util::matcher::RequestMatcher;

#[derive(Clone)]
pub struct RequestMatcherEntry<T> {
    request_matcher: Arc<dyn RequestMatcher>,
    entry: T,
}

impl<T> RequestMatcherEntry<T> {
    pub fn new(request_matcher: Arc<dyn RequestMatcher>, entry: T) -> Self {
        Self {
            request_matcher,
            entry,
        }
    }

    pub fn request_matcher(&self) -> &dyn RequestMatcher {
        self.request_matcher.as_ref()
    }

    pub fn entry(&self) -> &T {
        &self.entry
    }

    pub fn entry_mut(&mut self) -> &mut T {
        &mut self.entry
    }
}
