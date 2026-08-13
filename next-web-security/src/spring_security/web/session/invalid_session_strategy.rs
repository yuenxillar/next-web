use std::fmt::Debug;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

/// Strategy for handling an invalid (expired or otherwise invalid) session ID
/// detected during request processing.
///
/// This mirrors the Spring Security `InvalidSessionStrategy` interface.
pub trait InvalidSessionStrategy
where
    Self: Send + Sync,
    Self: Debug,
{
    /// Called when an invalid session ID is detected in the request.
    /// Implementations typically redirect to a login page or show an error.
    fn on_invalid_session_detected(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;
}
