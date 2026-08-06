use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::debug;

use crate::{
    core::authentication_error::AuthenticationError,
    web::{
        util::matcher::{RequestMatcher, RequestMatcherEntry},
        AuthenticationEntryPoint,
    },
};

/// An `AuthenticationEntryPoint` which selects a concrete `AuthenticationEntryPoint`
/// based on a `RequestMatcher` evaluation.
///
/// # Example
///
/// A configuration might look like this:
///
/// ```ignore
/// let entry_points = vec![
///     RequestMatcherEntry::new(
///         ELRequestMatcher::new("hasIpAddress('192.168.1.0/24') and hasHeader('User-Agent','Mozilla')"),
///         first_aep,
///     ),
///     RequestMatcherEntry::new(
///         ELRequestMatcher::new("hasHeader('User-Agent','MSIE')"),
///         second_aep,
///     ),
/// ];
/// let daep = DelegatingAuthenticationEntryPoint::new(default_aep, entry_points);
/// ```
pub struct DelegatingAuthenticationEntryPoint {
    entry_points: Vec<RequestMatcherEntry<Arc<dyn AuthenticationEntryPoint>>>,
    default_entry_point: Arc<dyn AuthenticationEntryPoint>,
}

impl DelegatingAuthenticationEntryPoint {
    /// Creates a new instance with the provided mappings.
    ///
    /// # Arguments
    ///
    /// * `default_entry_point` - The default `AuthenticationEntryPoint`. Cannot be null.
    /// * `entry_points` - The mapping of `RequestMatcher` to `AuthenticationEntryPoint`.
    ///   Cannot be null or empty.
    pub fn new(
        default_entry_point: Arc<dyn AuthenticationEntryPoint>,
        entry_points: Vec<RequestMatcherEntry<Arc<dyn AuthenticationEntryPoint>>>,
    ) -> Self {
        assert!(!entry_points.is_empty(), "entry_points cannot be empty");
        Self {
            entry_points,
            default_entry_point,
        }
    }

    /// Validates that required properties are set.
    pub fn after_properties_set(&self) {
        assert!(
            !self.entry_points.is_empty(),
            "entry_points must be specified"
        );
    }

    /// Creates a new `Builder`.
    pub fn builder() -> DelegatingAuthenticationEntryPointBuilder {
        DelegatingAuthenticationEntryPointBuilder::default()
    }
}

impl AuthenticationEntryPoint for DelegatingAuthenticationEntryPoint {
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        for entry in &self.entry_points {
            let request_matcher = entry.request_matcher();
            debug!("Trying to match using {:?}", request_matcher);
            if request_matcher.matches(request) {
                let entry_point = entry.entry();
                debug!("Match found! Executing AuthenticationEntryPoint");
                return entry_point.commence(request, response, auth_error);
            }
        }
        debug!("No match found. Using default entry point ");
        // No EntryPoint matched, use defaultEntryPoint
        self.default_entry_point
            .commence(request, response, auth_error)
    }
}

/// Builder for `DelegatingAuthenticationEntryPoint`.
///
/// Used to build a new instance of `DelegatingAuthenticationEntryPoint`.
#[derive(Default, Clone)]
pub struct DelegatingAuthenticationEntryPointBuilder {
    default_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,
    entry_points: Vec<RequestMatcherEntry<Arc<dyn AuthenticationEntryPoint>>>,
}

impl DelegatingAuthenticationEntryPointBuilder {
    /// Set the default `AuthenticationEntryPoint` if none match. The default is to use
    /// the first `AuthenticationEntryPoint` added in
    /// `add_entry_point_for`.
    ///
    /// # Arguments
    ///
    /// * `default_entry_point` - The default `AuthenticationEntryPoint` to use.
    pub fn default_entry_point(
        &mut self,
        default_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> &mut Self {
        self.default_entry_point = Some(default_entry_point);
        self
    }

    /// Adds an `AuthenticationEntryPoint` for the provided `RequestMatcher`.
    ///
    /// # Arguments
    ///
    /// * `entry_point` - The `AuthenticationEntryPoint` to use. Cannot be null.
    /// * `request_matcher` - The `RequestMatcher` to use. Cannot be null.
    pub fn add_entry_point_for(
        &mut self,
        entry_point: Arc<dyn AuthenticationEntryPoint>,
        request_matcher: Arc<dyn RequestMatcher>,
    ) -> &mut Self {
        self.entry_points
            .push(RequestMatcherEntry::new(request_matcher, entry_point));
        self
    }

    /// Builds the `AuthenticationEntryPoint`.
    ///
    /// If `default_entry_point` is not set, then the first entry point added via
    /// `add_entry_point_for` is used as the default. If `default_entry_point` is not
    /// set and there is only a single entry point, then that `AuthenticationEntryPoint`
    /// is returned directly rather than wrapping it in
    /// `DelegatingAuthenticationEntryPoint`.
    pub fn build(&mut self) -> Arc<dyn AuthenticationEntryPoint> {
        let default_entry_point = self.default_entry_point.take();

        match default_entry_point {
            None => {
                assert!(
                    !self.entry_points.is_empty(),
                    "entry_points cannot be empty if default_entry_point is null"
                );
                let first_authentication_entry_point = self.entry_points[0].entry().clone();
                if self.entry_points.len() == 1 {
                    return first_authentication_entry_point;
                }
                Arc::new(DelegatingAuthenticationEntryPoint::new(
                    first_authentication_entry_point,
                    std::mem::take(&mut self.entry_points),
                ))
            }
            Some(default_ep) => {
                if self.entry_points.is_empty() {
                    return default_ep;
                }
                Arc::new(DelegatingAuthenticationEntryPoint::new(
                    default_ep,
                    std::mem::take(&mut self.entry_points),
                ))
            }
        }
    }
}
