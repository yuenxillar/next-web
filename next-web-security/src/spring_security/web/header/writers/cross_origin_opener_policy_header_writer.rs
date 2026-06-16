use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Inserts the Cross-Origin-Opener-Policy header.
///
/// See the [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cross-Origin-Opener-Policy)
/// for more details.
#[derive(Clone, Default)]
pub struct CrossOriginOpenerPolicyHeaderWriter {
    policy: Option<CrossOriginOpenerPolicy>,
}

impl CrossOriginOpenerPolicyHeaderWriter {
    const OPENER_POLICY: &'static str = "Cross-Origin-Opener-Policy";

    /// Creates a new instance with the specified policy.
    ///
    /// # Arguments
    ///
    /// * `policy` - the `CrossOriginOpenerPolicy` to use
    pub fn with_policy(policy: CrossOriginOpenerPolicy) -> Self {
        Self {
            policy: Some(policy),
        }
    }

    /// Sets the `CrossOriginOpenerPolicy` value to be used in the
    /// `Cross-Origin-Opener-Policy` header.
    ///
    /// # Arguments
    ///
    /// * `opener_policy` - the `CrossOriginOpenerPolicy` to use
    pub fn set_policy(&mut self, opener_policy: CrossOriginOpenerPolicy) {
        self.policy = Some(opener_policy);
    }
}

impl HeaderWriter for CrossOriginOpenerPolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if let Some(ref policy) = self.policy {
            if !response.contains_header(Self::OPENER_POLICY) {
                response.insert_header(Self::OPENER_POLICY, policy.get_policy());
            }
        }
    }
}

/// The Cross-Origin-Opener-Policy directives as defined in the specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrossOriginOpenerPolicy {
    /// unsafe-none
    UnsafeNone,
    /// same-origin-allow-popups
    SameOriginAllowPopups,
    /// same-origin
    SameOrigin,
}

impl CrossOriginOpenerPolicy {
    /// Returns the policy string value.
    pub fn get_policy(&self) -> &'static str {
        match self {
            CrossOriginOpenerPolicy::UnsafeNone => "unsafe-none",
            CrossOriginOpenerPolicy::SameOriginAllowPopups => "same-origin-allow-popups",
            CrossOriginOpenerPolicy::SameOrigin => "same-origin",
        }
    }

    /// Returns the `CrossOriginOpenerPolicy` corresponding to the given policy string,
    /// or `None` if no match is found.
    pub fn from(opener_policy: &str) -> Option<CrossOriginOpenerPolicy> {
        match opener_policy {
            "unsafe-none" => Some(CrossOriginOpenerPolicy::UnsafeNone),
            "same-origin-allow-popups" => Some(CrossOriginOpenerPolicy::SameOriginAllowPopups),
            "same-origin" => Some(CrossOriginOpenerPolicy::SameOrigin),
            _ => None,
        }
    }
}

impl std::fmt::Display for CrossOriginOpenerPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_policy())
    }
}
