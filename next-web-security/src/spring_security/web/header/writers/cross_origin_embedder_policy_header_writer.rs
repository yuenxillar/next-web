use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Inserts the Cross-Origin-Embedder-Policy header.
///
/// See the [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cross-Origin-Embedder-Policy)
/// for more details.
#[derive(Clone, Default)]
pub struct CrossOriginEmbedderPolicyHeaderWriter {
    policy: Option<CrossOriginEmbedderPolicy>,
}

impl CrossOriginEmbedderPolicyHeaderWriter {
    const EMBEDDER_POLICY: &'static str = "Cross-Origin-Embedder-Policy";

    /// Creates a new instance with the specified policy.
    ///
    /// # Arguments
    ///
    /// * `policy` - the `CrossOriginEmbedderPolicy` to use
    pub fn new(policy: CrossOriginEmbedderPolicy) -> Self {
        Self {
            policy: Some(policy),
        }
    }

    /// Sets the `CrossOriginEmbedderPolicy` value to be used in the
    /// `Cross-Origin-Embedder-Policy` header.
    ///
    /// # Arguments
    ///
    /// * `embedder_policy` - the `CrossOriginEmbedderPolicy` to use
    pub fn set_policy(&mut self, embedder_policy: CrossOriginEmbedderPolicy) {
        self.policy = Some(embedder_policy);
    }
}

impl HeaderWriter for CrossOriginEmbedderPolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if let Some(ref policy) = self.policy {
            if !response.contains_header(Self::EMBEDDER_POLICY) {
                response.append_header(Self::EMBEDDER_POLICY, policy.get_policy());
            }
        }
    }
}

/// The Cross-Origin-Embedder-Policy directives as defined in the specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrossOriginEmbedderPolicy {
    /// unsafe-none
    UnsafeNone,
    /// require-corp
    RequireCorp,
    /// credentialless
    Credentialless,
}

impl CrossOriginEmbedderPolicy {
    /// Returns the policy string value.
    pub fn get_policy(&self) -> &'static str {
        match self {
            CrossOriginEmbedderPolicy::UnsafeNone => "unsafe-none",
            CrossOriginEmbedderPolicy::RequireCorp => "require-corp",
            CrossOriginEmbedderPolicy::Credentialless => "credentialless",
        }
    }

    /// Returns the `CrossOriginEmbedderPolicy` corresponding to the given policy string,
    /// or `None` if no match is found.
    pub fn from(embedder_policy: &str) -> Option<CrossOriginEmbedderPolicy> {
        match embedder_policy {
            "unsafe-none" => Some(CrossOriginEmbedderPolicy::UnsafeNone),
            "require-corp" => Some(CrossOriginEmbedderPolicy::RequireCorp),
            "credentialless" => Some(CrossOriginEmbedderPolicy::Credentialless),
            _ => None,
        }
    }
}

impl std::fmt::Display for CrossOriginEmbedderPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_policy())
    }
}
