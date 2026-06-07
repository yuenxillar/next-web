use std::sync::{Arc, LazyLock};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::util::matcher::RequestMatcher;

static INSTANCE: LazyLock<Arc<dyn RequestMatcher + Send + Sync>> =
    LazyLock::new(|| Arc::new(AnyRequestMatcher::default()));

#[derive(Default, Debug, Clone)]
pub struct AnyRequestMatcher;

impl AnyRequestMatcher {
    pub fn instance() -> Arc<dyn RequestMatcher> {
        INSTANCE.clone()
    }
}

impl RequestMatcher for AnyRequestMatcher {
    fn matches(&self, _request: &dyn HttpRequest) -> bool {
        true
    }
}
