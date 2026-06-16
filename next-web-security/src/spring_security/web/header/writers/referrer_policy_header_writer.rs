use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::HeaderWriter;

/// Provides support for [Referrer Policy](https://www.w3.org/TR/referrer-policy/).
///
/// The list of policies defined can be found at
/// [Referrer Policies](https://www.w3.org/TR/referrer-policy/#referrer-policies).
///
/// This implementation of `HeaderWriter` writes the following header:
///
/// * Referrer-Policy
///
/// By default, the Referrer-Policy header is not included in the response. Policy
/// **no-referrer** is used by default if no `ReferrerPolicy` is set.
#[derive(Clone)]
pub struct ReferrerPolicyHeaderWriter {
    policy: ReferrerPolicy,
}

impl ReferrerPolicyHeaderWriter {
    const REFERRER_POLICY_HEADER: &'static str = "Referrer-Policy";

    /// Creates a new instance.
    ///
    /// # Arguments
    ///
    /// * `policy` - a referrer policy
    pub fn new(policy: ReferrerPolicy) -> Self {
        Self { policy }
    }

    /// Sets the policy to be used in the response header.
    ///
    /// # Arguments
    ///
    /// * `policy` - a referrer policy
    pub fn set_policy(&mut self, policy: ReferrerPolicy) {
        self.policy = policy;
    }
}

impl HeaderWriter for ReferrerPolicyHeaderWriter {
    fn write_headers(&self, _request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if !response.contains_header(Self::REFERRER_POLICY_HEADER) {
            response.insert_header(Self::REFERRER_POLICY_HEADER, self.policy.get_policy());
        }
    }
}

impl Default for ReferrerPolicyHeaderWriter {
    fn default() -> Self {
        Self {
            policy: ReferrerPolicy::NoReferrer,
        }
    }
}

/// The referrer policy directives as defined in the W3C Referrer Policy specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferrerPolicy {
    /// no-referrer
    NoReferrer,
    /// no-referrer-when-downgrade
    NoReferrerWhenDowngrade,
    /// same-origin
    SameOrigin,
    /// origin
    Origin,
    /// strict-origin
    StrictOrigin,
    /// origin-when-cross-origin
    OriginWhenCrossOrigin,
    /// strict-origin-when-cross-origin
    StrictOriginWhenCrossOrigin,
    /// unsafe-url
    UnsafeUrl,
}

impl ReferrerPolicy {
    /// Returns the policy string value.
    pub fn get_policy(&self) -> &'static str {
        match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        }
    }

    // /// Returns the `ReferrerPolicy` corresponding to the given policy string,
    // /// or `None` if no match is found.
    // pub fn get(referrer_policy: &str) -> Option<ReferrerPolicy> {
    //     Self::referrer_policies_map().get(referrer_policy).copied()
    // }

    // /// Returns a static reference to the map of policy strings to `ReferrerPolicy` variants.
    // fn referrer_policies_map() -> &'static HashMap<&'static str, ReferrerPolicy> {
    //     static REFERRER_POLICIES: std::sync::LazyLock<HashMap<&'static str, ReferrerPolicy>> =
    //         std::sync::LazyLock::new(|| {
    //             let mut map = HashMap::new();
    //             let values = [
    //                 ReferrerPolicy::NoReferrer,
    //                 ReferrerPolicy::NoReferrerWhenDowngrade,
    //                 ReferrerPolicy::SameOrigin,
    //                 ReferrerPolicy::Origin,
    //                 ReferrerPolicy::StrictOrigin,
    //                 ReferrerPolicy::OriginWhenCrossOrigin,
    //                 ReferrerPolicy::StrictOriginWhenCrossOrigin,
    //                 ReferrerPolicy::UnsafeUrl,
    //             ];
    //             for policy in values {
    //                 map.insert(policy.get_policy(), policy);
    //             }
    //             map
    //         });
    //     &REFERRER_POLICIES
    // }
}

impl std::fmt::Display for ReferrerPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_policy())
    }
}
