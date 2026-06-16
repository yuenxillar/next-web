use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Provides support for [Content Security Policy (CSP) Level 2](https://www.w3.org/TR/CSP2/).
///
/// CSP provides a mechanism for web applications to mitigate content injection
/// vulnerabilities, such as cross-site scripting (XSS). CSP is a declarative policy that
/// allows web application authors to inform the client (user-agent) about the sources from
/// which the application expects to load resources.
///
/// For example, a web application can declare that it only expects to load script from
/// specific, trusted sources. This declaration allows the client to detect and block
/// malicious scripts injected into the application by an attacker.
///
/// A declaration of a security policy contains a set of security policy directives (for
/// example, script-src and object-src), each responsible for declaring the restrictions
/// for a particular resource type. The list of directives defined can be found at
/// [Directives](https://www.w3.org/TR/CSP2/#directives).
///
/// Each directive has a name and value. For detailed syntax on writing security policies,
/// see [Syntax and Algorithms](https://www.w3.org/TR/CSP2/#syntax-and-algorithms).
///
/// This implementation of `HeaderWriter` writes one of the following headers:
///
/// * Content-Security-Policy
/// * Content-Security-Policy-Report-Only
///
/// By default, the Content-Security-Policy header is included in the response. However,
/// calling `set_report_only` with `true` will include the
/// Content-Security-Policy-Report-Only header in the response. **NOTE:** The
/// supplied security policy directive(s) will be used for whichever header is enabled
/// (included).
///
/// **CSP is not intended as a first line of defense against content injection
/// vulnerabilities. Instead, CSP is used to reduce the harm caused by content injection
/// attacks. As a first line of defense against content injection, web application authors
/// should validate their input and encode their output.
#[derive(Clone)]
pub struct ContentSecurityPolicyHeaderWriter {
    policy_directives: String,
    report_only: bool,
}

impl ContentSecurityPolicyHeaderWriter {
    const CONTENT_SECURITY_POLICY_HEADER: &'static str = "Content-Security-Policy";
    const CONTENT_SECURITY_POLICY_REPORT_ONLY_HEADER: &'static str =
        "Content-Security-Policy-Report-Only";
    const DEFAULT_SRC_SELF_POLICY: &'static str = "default-src 'self'";

    /// Creates a new instance
    ///
    /// # Arguments
    ///
    /// * `policy_directives` - maps to `set_policy_directives`
    ///
    /// # Panics
    ///
    /// Panics if `policy_directives` is null or empty
    pub fn new(policy_directives: impl Into<String>) -> Self {
        let mut writer = Self::default();
        writer.set_policy_directives(policy_directives);
        writer
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
    pub fn set_policy_directives(&mut self, policy_directives: impl Into<String>) {
        let policy_directives = policy_directives.into();
        assert!(
            !policy_directives.is_empty(),
            "policyDirectives cannot be null or empty"
        );
        self.policy_directives = policy_directives;
    }

    /// If true, includes the Content-Security-Policy-Report-Only header in the response,
    /// otherwise, defaults to the Content-Security-Policy header.
    ///
    /// # Arguments
    ///
    /// * `report_only` - set to true for reporting policy violations only
    pub fn set_report_only(&mut self, report_only: bool) {
        self.report_only = report_only;
    }
}

impl HeaderWriter for ContentSecurityPolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        let header_name = if !self.report_only {
            Self::CONTENT_SECURITY_POLICY_HEADER
        } else {
            Self::CONTENT_SECURITY_POLICY_REPORT_ONLY_HEADER
        };

        if !response.contains_header(header_name) {
            response.insert_header(header_name, &self.policy_directives);
        }
    }
}

impl std::fmt::Display for ContentSecurityPolicyHeaderWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ContentSecurityPolicyHeaderWriter [policyDirectives={}; reportOnly={}]",
            self.policy_directives, self.report_only
        )
    }
}

impl Default for ContentSecurityPolicyHeaderWriter {
    fn default() -> Self {
        Self {
            policy_directives: Self::DEFAULT_SRC_SELF_POLICY.to_string(),
            report_only: false,
        }
    }
}
