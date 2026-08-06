use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::util::matcher::RequestMatcher;

/// A `RequestMatcher` that can be used to match request that contain a header with
/// an expected header name and an expected value.
///
/// For example, the following will match a request that contains a header with the name
/// X-Requested-With no matter what the value is.
///
/// ```text
/// RequestMatcher matcher = new RequestHeaderRequestMatcher("X-Requested-With");
/// ```
///
/// Alternatively, the RequestHeaderRequestMatcher can be more precise and require a
/// specific value. For example the following will match on requests with the header name
/// of X-Requested-With with the value of "XMLHttpRequest", but will not match on header
/// name of "X-Requested-With" with the value of "Other".
///
/// ```text
/// RequestMatcher matcher = new RequestHeaderRequestMatcher("X-Requested-With",
///     "XMLHttpRequest");
/// ```
///
/// The value used to compare is the first header value, so in the previous example if the
/// header "X-Requested-With" contains the values "Other" and "XMLHttpRequest", then it
/// will not match.
#[derive(Clone)]
pub struct RequestHeaderRequestMatcher {
    expected_header_name: String,
    expected_header_value: Option<String>,
}

impl RequestHeaderRequestMatcher {
    /// Creates a new instance that will match if a header by the name of
    /// `expected_header_name` is present and if the `expected_header_value` is
    /// non-empty the first value matches.
    ///
    /// # Arguments
    /// * `expected_header_name` - the name of the expected header. Cannot be empty.
    /// * `expected_header_value` - the expected header value or `None` if the value
    ///   does not matter.
    pub fn new(
        expected_header_name: impl Into<String>,
        expected_header_value: Option<String>,
    ) -> Self {
        Self {
            expected_header_name: expected_header_name.into(),
            expected_header_value,
        }
    }

    /// Creates a new instance that will match if a header by the name of
    /// `expected_header_name` is present. In this instance, the value does not matter.
    ///
    /// # Arguments
    /// * `expected_header_name` - the name of the expected header that if present the
    ///   request will match. Cannot be empty.
    pub fn with_name(expected_header_name: impl Into<String>) -> Self {
        Self::new(expected_header_name, None)
    }

    /// Returns the expected header name that requests must contain to match.
    ///
    /// # Returns
    /// The expected header name
    pub fn get_expected_header_name(&self) -> &str {
        &self.expected_header_name
    }

    /// Returns the expected header value that the first header value must equal,
    /// or `None` if any value is accepted.
    ///
    /// # Returns
    /// The expected header value, or `None` if not specified
    pub fn get_expected_header_value(&self) -> Option<&str> {
        self.expected_header_value.as_deref()
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

impl std::fmt::Display for RequestHeaderRequestMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.expected_header_value {
            Some(value) => write!(
                f,
                "RequestHeaderRequestMatcher [expectedHeaderName={}, expectedHeaderValue={}]",
                self.expected_header_name, value
            ),
            None => write!(
                f,
                "RequestHeaderRequestMatcher [expectedHeaderName={}, expectedHeaderValue=null]",
                self.expected_header_name
            ),
        }
    }
}

impl std::fmt::Debug for RequestHeaderRequestMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestHeaderRequestMatcher")
            .field("expected_header_name", &self.expected_header_name)
            .field("expected_header_value", &self.expected_header_value)
            .finish()
    }
}
