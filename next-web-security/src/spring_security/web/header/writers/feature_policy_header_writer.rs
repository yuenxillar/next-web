use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Provides support for [Feature Policy](https://wicg.github.io/feature-policy/).
///
/// Feature Policy allows web developers to selectively enable, disable, and modify the
/// behavior of certain APIs and web features in the browser.
///
/// A declaration of a feature policy contains a set of security policy directives, each
/// responsible for declaring the restrictions for a particular feature type.
#[derive(Clone)]
pub struct FeaturePolicyHeaderWriter {
    policy_directives: String,
}

impl FeaturePolicyHeaderWriter {
    const FEATURE_POLICY_HEADER: &'static str = "Feature-Policy";

    /// Creates a new instance of `FeaturePolicyHeaderWriter` with supplied security
    /// policy directive(s).
    ///
    /// # Arguments
    ///
    /// * `policy_directives` - the security policy directive(s)
    ///
    /// # Panics
    ///
    /// Panics if `policy_directives` is empty
    pub fn new(policy_directives: impl Into<String>) -> Self {
        let policy_directives = policy_directives.into();
        assert!(
            !policy_directives.is_empty(),
            "policyDirectives must not be null or empty"
        );

        Self { policy_directives }
    }

    /// Sets the security policy directive(s) to be used in the response header.
    ///
    /// # Arguments
    ///
    /// * `policy_directives` - the security policy directive(s)
    ///
    /// # Panics
    ///
    /// Panics if `policy_directives` is empty
    pub fn set_policy_directives(&mut self, policy_directives: &str) {
        assert!(
            !policy_directives.is_empty(),
            "policyDirectives must not be null or empty"
        );
        self.policy_directives = policy_directives.to_string();
    }
}

impl HeaderWriter for FeaturePolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if !response.contains_header(Self::FEATURE_POLICY_HEADER) {
            response.insert_header(Self::FEATURE_POLICY_HEADER, &self.policy_directives);
        }
    }
}

impl std::fmt::Display for FeaturePolicyHeaderWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FeaturePolicyHeaderWriter [policyDirectives={}]",
            self.policy_directives
        )
    }
}
