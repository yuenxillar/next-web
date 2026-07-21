use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::util::matcher::RequestMatcher;

#[derive(Debug, Clone)]
pub struct RequestHeaderRequestMatcher {
    expected_header_name: String,
    expected_header_value: Option<String>,
}

impl RequestHeaderRequestMatcher {
    pub fn new(
        expected_header_name: impl Into<String>,
        expected_header_value: Option<String>,
    ) -> Self {
        Self {
            expected_header_name: expected_header_name.into(),
            expected_header_value,
        }
    }
}

impl RequestMatcher for RequestHeaderRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        let actual_header_value = request.header(&self.expected_header_name);

        match self.expected_header_value.as_deref() {
            Some(expected) => actual_header_value == Some(expected),
            None => actual_header_value.is_some(),
        }
    }
}
