use crate::config::web::util::matcher::request_matcher::RequestMatcher;



#[derive(Clone)]
pub struct RequestMatcherEntry<T> {
    request_matcher: Box<dyn RequestMatcher>,
    entry: T,
}

impl<T> RequestMatcherEntry<T> {
    pub fn new(request_matcher: Box<dyn RequestMatcher>, entry: T) -> Self {
        Self {
            request_matcher,
            entry,
        }
    }
}