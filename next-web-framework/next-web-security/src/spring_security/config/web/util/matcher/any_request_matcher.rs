use crate::config::web::util::matcher::request_matcher::RequestMatcher;

#[derive(Default, Debug, Clone)]
pub struct AnyRequestMatcher;

impl RequestMatcher for AnyRequestMatcher {
    fn matches(&self, _request: &axum::extract::Request) -> bool {
        true
    }
}
