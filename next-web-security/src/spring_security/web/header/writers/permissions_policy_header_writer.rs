use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Provides support for
/// [Permissions Policy](https://w3c.github.io/webappsec-permissions-policy/).
///
/// Permissions Policy allows web developers to selectively enable, disable, and modify the
/// behavior of certain APIs and web features in the browser.
///
/// A declaration of a permissions policy contains a set of security policies, each
/// responsible for declaring the restrictions for a particular feature type.
#[derive(Clone, Default)]
pub struct PermissionsPolicyHeaderWriter {
    policy: Option<String>,
}

impl PermissionsPolicyHeaderWriter {
    const PERMISSIONS_POLICY_HEADER: &'static str = "Permissions-Policy";

    /// Creates a new instance of `PermissionsPolicyHeaderWriter` with supplied
    /// security policy.
    ///
    /// # Arguments
    ///
    /// * `policy` - the security policy
    ///
    /// # Panics
    ///
    /// Panics if `policy` is empty
    pub fn new(policy: &str) -> Self {
        let mut writer = Self::default();
        writer.set_policy(policy);

        writer
    }

    /// Sets the policy to be used in the response header.
    ///
    /// # Arguments
    ///
    /// * `policy` - a permissions policy
    ///
    /// # Panics
    ///
    /// Panics if `policy` is empty
    pub fn set_policy(&mut self, policy: impl Into<String>) {
        let policy = policy.into();
        assert!(!policy.is_empty(), "policy can not be null or empty");
        self.policy = Some(policy);
    }
}

impl HeaderWriter for PermissionsPolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if let Some(ref policy) = self.policy {
            if !response.contains_header(Self::PERMISSIONS_POLICY_HEADER) {
                response.insert_header(Self::PERMISSIONS_POLICY_HEADER, policy);
            }
        }
    }
}

impl std::fmt::Display for PermissionsPolicyHeaderWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PermissionsPolicyHeaderWriter [policy={}]",
            self.policy.as_deref().unwrap_or("null")
        )
    }
}
