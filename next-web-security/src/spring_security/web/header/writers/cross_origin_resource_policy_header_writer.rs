use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Inserts the Cross-Origin-Resource-Policy header.
///
/// See the [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cross-Origin-Resource-Policy)
/// for more details.
#[derive(Clone, Default)]
pub struct CrossOriginResourcePolicyHeaderWriter {
    policy: Option<CrossOriginResourcePolicy>,
}

impl CrossOriginResourcePolicyHeaderWriter {
    const RESOURCE_POLICY: &'static str = "Cross-Origin-Resource-Policy";

    /// Creates a new instance with the specified policy.
    ///
    /// # Arguments
    ///
    /// * `policy` - the `CrossOriginResourcePolicy` to use
    pub fn new(policy: CrossOriginResourcePolicy) -> Self {
        Self {
            policy: Some(policy),
        }
    }

    /// Sets the `CrossOriginResourcePolicy` value to be used in the
    /// `Cross-Origin-Resource-Policy` header.
    ///
    /// # Arguments
    ///
    /// * `resource_policy` - the `CrossOriginResourcePolicy` to use
    pub fn set_policy(&mut self, resource_policy: CrossOriginResourcePolicy) {
        self.policy = Some(resource_policy);
    }
}

impl HeaderWriter for CrossOriginResourcePolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if let Some(policy) = self.policy.as_ref() {
            if !response.contains_header(Self::RESOURCE_POLICY) {
                response.append_header(Self::RESOURCE_POLICY, policy.get_policy());
            }
        }
    }
}

/// The Cross-Origin-Resource-Policy directives as defined in the specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrossOriginResourcePolicy {
    /// same-site
    SameSite,
    /// same-origin
    SameOrigin,
    /// cross-origin
    CrossOrigin,
}

impl CrossOriginResourcePolicy {
    /// Returns the policy string value.
    pub fn get_policy(&self) -> &'static str {
        match self {
            CrossOriginResourcePolicy::SameSite => "same-site",
            CrossOriginResourcePolicy::SameOrigin => "same-origin",
            CrossOriginResourcePolicy::CrossOrigin => "cross-origin",
        }
    }

    /// Returns the `CrossOriginResourcePolicy` corresponding to the given policy string,
    /// or `None` if no match is found.
    pub fn from(resource_policy: &str) -> Option<CrossOriginResourcePolicy> {
        match resource_policy {
            "same-site" => Some(CrossOriginResourcePolicy::SameSite),
            "same-origin" => Some(CrossOriginResourcePolicy::SameOrigin),
            "cross-origin" => Some(CrossOriginResourcePolicy::CrossOrigin),
            _ => None,
        }
    }
}

impl std::fmt::Display for CrossOriginResourcePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_policy())
    }
}
